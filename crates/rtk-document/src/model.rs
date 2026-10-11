//! The document: a project of views, each a tree of nodes.

use std::collections::BTreeMap;

use crate::kind::WidgetKind;
use crate::value::{FieldType, PropKind, Value};

/// Identifies a node within its view. Not part of the code: ids are
/// reassigned when a view is parsed again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// A widget, a container or a block of free code.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Unique within the view.
    pub id: NodeId,
    /// What the node is.
    pub kind: WidgetKind,
    /// Property values by key (see [`WidgetKind::spec`]). Optional
    /// properties that are unset are missing.
    pub props: BTreeMap<&'static str, Value>,
    /// Children, for containers.
    pub children: Vec<Node>,
}

impl Node {
    /// A node with the initial property values of its kind.
    pub fn new(id: NodeId, kind: WidgetKind) -> Self {
        let props = kind
            .spec()
            .props
            .iter()
            .filter_map(|spec| Some((spec.key, spec.init.to_value()?)))
            .collect();
        Self {
            id,
            kind,
            props,
            children: Vec::new(),
        }
    }

    /// A property value (`None` if unset or unknown).
    pub fn prop(&self, key: &str) -> Option<&Value> {
        self.props.get(key)
    }

    /// A text property, or `""`.
    pub fn text(&self, key: &str) -> &str {
        self.prop(key).and_then(Value::as_text).unwrap_or("")
    }

    /// A bool property, or `false`.
    pub fn flag(&self, key: &str) -> bool {
        self.prop(key).and_then(Value::as_bool).unwrap_or(false)
    }

    /// The field this widget is bound to, with its type.
    pub fn binding(&self) -> Option<(&str, FieldType)> {
        self.kind
            .spec()
            .props
            .iter()
            .find_map(|spec| match spec.kind {
                PropKind::Binding(ty) => Some((self.text(spec.key), ty)),
                _ => None,
            })
    }

    /// Short description for the component tree, e.g. `Button "Save"`.
    pub fn summary(&self) -> String {
        let name = self.kind.name();
        match self.kind {
            WidgetKind::RawCode => {
                let first = self.text("code").lines().find(|l| !l.trim().is_empty());
                format!("{name}: {}", first.unwrap_or("").trim())
            }
            WidgetKind::Stack => format!("{name} ({})", self.text("direction")),
            _ => {
                let text = ["text", "label"]
                    .iter()
                    .map(|key| self.text(key))
                    .find(|t| !t.is_empty());
                match (text, self.binding()) {
                    (Some(text), _) => format!("{name} \"{text}\""),
                    (None, Some((binding, _))) => format!("{name} ({binding})"),
                    (None, None) => name.to_owned(),
                }
            }
        }
    }

    /// Calls `f` on this node and all its descendants, parents first.
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Node)) {
        f(self);
        for child in &self.children {
            child.walk(f);
        }
    }

    /// Like [`Node::walk`], with mutable access.
    pub fn walk_mut(&mut self, f: &mut impl FnMut(&mut Node)) {
        f(self);
        for child in &mut self.children {
            child.walk_mut(f);
        }
    }
}

/// A field of the view's state struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Field name (snake_case).
    pub name: String,
    /// Field type.
    pub ty: FieldType,
}

/// Two widgets bound to the same field with different types, or a binding
/// that clashes with a field written by hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingConflict {
    /// The field name.
    pub name: String,
    /// The type the field already has.
    pub first: FieldType,
    /// The node that needs another type.
    pub node: NodeId,
    /// The type that node needs (`None`: clash with a hand-written field).
    pub other: Option<FieldType>,
}

/// A struct field the toolkit doesn't manage (not bound to a widget),
/// kept as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawField {
    /// Field name.
    pub name: String,
    /// The whole field as written, e.g. `pub items: Vec<String>`
    /// (attributes and doc comments included).
    pub code: String,
}

/// The window a view is shown in: every view is a window.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSettings {
    /// Title bar text.
    pub title: String,
    /// Width of the content, in points.
    pub width: f32,
    /// Height of the content, in points.
    pub height: f32,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Smallest size the user can resize the window to.
    pub min_size: Option<(f32, f32)>,
    /// Largest size the user can resize the window to.
    pub max_size: Option<(f32, f32)>,
}

impl WindowSettings {
    /// Smallest width or height.
    pub const MIN_SIZE: f32 = 50.0;
    /// Largest width or height.
    pub const MAX_SIZE: f32 = 10_000.0;

    /// The default window of view `name`: `login_form` → "Login form",
    /// 480 × 360, resizable.
    pub fn for_view(name: &str) -> Self {
        let words = name.replace('_', " ");
        let mut chars = words.trim().chars();
        let title = match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        };
        Self {
            title,
            width: 480.0,
            height: 360.0,
            resizable: true,
            min_size: None,
            max_size: None,
        }
    }

    /// Whether the sizes are within bounds.
    pub fn is_valid(&self) -> bool {
        let ok = |v: f32| v.is_finite() && (Self::MIN_SIZE..=Self::MAX_SIZE).contains(&v);
        let limit_ok = |size: Option<(f32, f32)>| size.is_none_or(|(w, h)| ok(w) && ok(h));
        ok(self.width) && ok(self.height) && limit_ok(self.min_size) && limit_ok(self.max_size)
    }

    /// `size` kept within the minimum and maximum sizes.
    pub fn clamp(&self, (mut w, mut h): (f32, f32)) -> (f32, f32) {
        if let Some((max_w, max_h)) = self.max_size {
            (w, h) = (w.min(max_w), h.min(max_h));
        }
        if let Some((min_w, min_h)) = self.min_size {
            (w, h) = (w.max(min_w), h.max(min_h));
        }
        (w, h)
    }
}

/// One screen: a file `<name>.rs` with a struct, its window settings and
/// its `show` method.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// File (module) name, snake_case; the struct is its PascalCase form.
    pub name: String,
    /// The window the view is shown in.
    pub window: WindowSettings,
    /// Style of the whole view ([`crate::kind::CONTENT_STYLE`] properties),
    /// set at the start of `show`.
    pub style: BTreeMap<&'static str, Value>,
    /// The statements of `show`, top to bottom.
    pub nodes: Vec<Node>,
    /// Derives of the struct (default: `Debug`, `Default`).
    pub derives: Vec<String>,
    /// Struct fields not bound to widgets.
    pub extra_fields: Vec<RawField>,
    /// Other items of `impl View` (methods besides `show`), as written.
    pub extra_methods: Vec<String>,
    /// Other items of the file (`impl Default`, functions, `use`s of other
    /// crates…), as written.
    pub extra_items: Vec<String>,
    /// Names imported from `rustroke` that the generator doesn't manage
    /// (used by free code), kept in the generated `use rustroke::{…}`.
    pub extra_imports: Vec<String>,
    next_id: u64,
}

impl View {
    /// An empty view.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            window: WindowSettings::for_view(&name),
            style: BTreeMap::new(),
            name,
            nodes: Vec::new(),
            derives: vec!["Debug".to_owned(), "Default".to_owned()],
            extra_fields: Vec::new(),
            extra_methods: Vec::new(),
            extra_items: Vec::new(),
            extra_imports: Vec::new(),
            next_id: 1,
        }
    }

    /// The struct name (`login_form` → `LoginForm`).
    pub fn struct_name(&self) -> String {
        crate::names::to_pascal_case(&self.name)
    }

    /// A fresh node id.
    pub fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Calls `f` on every node, parents first, top to bottom.
    pub fn walk<'a>(&'a self, mut f: impl FnMut(&'a Node)) {
        for node in &self.nodes {
            node.walk(&mut f);
        }
    }

    /// The node with this id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        let mut found = None;
        self.walk(|n| {
            if n.id == id {
                found = Some(n);
            }
        });
        found
    }

    /// The node with this id, mutable.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        fn find(nodes: &mut [Node], id: NodeId) -> Option<&mut Node> {
            for node in nodes {
                if node.id == id {
                    return Some(node);
                }
                if let Some(found) = find(&mut node.children, id) {
                    return Some(found);
                }
            }
            None
        }
        find(&mut self.nodes, id)
    }

    /// Indices from the top level down to the node (e.g. `[2, 0]` = first
    /// child of the third top-level node).
    pub fn path(&self, id: NodeId) -> Option<Vec<usize>> {
        fn find(nodes: &[Node], id: NodeId, path: &mut Vec<usize>) -> bool {
            for (i, node) in nodes.iter().enumerate() {
                path.push(i);
                if node.id == id || find(&node.children, id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut path = Vec::new();
        find(&self.nodes, id, &mut path).then_some(path)
    }

    /// The node at `path` (see [`View::path`]).
    pub fn node_at_path(&self, path: &[usize]) -> Option<&Node> {
        let (first, rest) = path.split_first()?;
        let mut node = self.nodes.get(*first)?;
        for &i in rest {
            node = node.children.get(i)?;
        }
        Some(node)
    }

    /// The parent of a node: `Some(None)` for top-level nodes, `None` if
    /// the node doesn't exist.
    pub fn parent(&self, id: NodeId) -> Option<Option<NodeId>> {
        let path = self.path(id)?;
        if path.len() == 1 {
            return Some(None);
        }
        Some(Some(self.node_at_path(&path[..path.len() - 1])?.id))
    }

    /// The children of `parent` (`None` = top level).
    pub fn children(&self, parent: Option<NodeId>) -> Option<&[Node]> {
        match parent {
            None => Some(&self.nodes),
            Some(id) => self.node(id).map(|n| n.children.as_slice()),
        }
    }

    pub(crate) fn children_mut(&mut self, parent: Option<NodeId>) -> Option<&mut Vec<Node>> {
        match parent {
            None => Some(&mut self.nodes),
            Some(id) => self.node_mut(id).map(|n| &mut n.children),
        }
    }

    /// The fields of the state struct that widgets are bound to, in order of
    /// first use, and the bindings whose types conflict.
    pub fn fields(&self) -> (Vec<Field>, Vec<BindingConflict>) {
        let mut fields: Vec<Field> = Vec::new();
        let mut conflicts = Vec::new();
        self.walk(|node| {
            let Some((name, ty)) = node.binding() else {
                return;
            };
            if self.extra_fields.iter().any(|f| f.name == name) {
                conflicts.push(BindingConflict {
                    name: name.to_owned(),
                    first: ty,
                    node: node.id,
                    other: None,
                });
            } else if let Some(field) = fields.iter().find(|f| f.name == name) {
                if field.ty != ty {
                    conflicts.push(BindingConflict {
                        name: name.to_owned(),
                        first: field.ty,
                        node: node.id,
                        other: Some(ty),
                    });
                }
            } else {
                fields.push(Field {
                    name: name.to_owned(),
                    ty,
                });
            }
        });
        (fields, conflicts)
    }
}

/// A set of views saved together in one folder.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Project {
    /// Project (folder) name.
    pub name: String,
    /// Views, in display order.
    pub views: Vec<View>,
}

impl Project {
    /// The view with this name.
    pub fn view(&self, name: &str) -> Option<&View> {
        self.views.iter().find(|v| v.name == name)
    }

    /// The view with this name, mutable.
    pub fn view_mut(&mut self, name: &str) -> Option<&mut View> {
        self.views.iter_mut().find(|v| v.name == name)
    }
}
