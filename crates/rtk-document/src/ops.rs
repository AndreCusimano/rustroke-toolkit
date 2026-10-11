//! Editing operations. Every change made by the canvas, the component tree
//! or the properties panel goes through these, so they keep the document
//! valid (containment rules, unique ids, typed properties).

use std::collections::BTreeSet;

use crate::kind::WidgetKind;
use crate::model::{Node, NodeId, Project, View, WindowSettings};
use crate::names::{NameError, check_snake_case};
use crate::value::{PropKind, Value};

/// Why an edit was refused.
#[derive(Clone, Debug, PartialEq)]
pub enum EditError {
    /// No node with this id.
    NoSuchNode(NodeId),
    /// The parent can't contain this kind of node.
    NotAccepted {
        /// The parent kind (`None` = top level).
        parent: Option<WidgetKind>,
        /// The child kind.
        child: WidgetKind,
    },
    /// A node can't be moved inside itself.
    IntoItself,
    /// The node has no property with this key.
    NoSuchProp(String),
    /// The value doesn't fit the property's type or bounds.
    InvalidValue(String),
    /// Only optional properties can be unset.
    Required(String),
    /// A view name is invalid or taken.
    Name(NameError),
    /// No view with this name.
    NoSuchView(String),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchNode(id) => write!(f, "no node {}", id.0),
            Self::NotAccepted { parent, child } => write!(
                f,
                "{} can't go inside {}",
                child.name(),
                parent.map_or("the view", WidgetKind::name)
            ),
            Self::IntoItself => f.write_str("a node can't be moved inside itself"),
            Self::NoSuchProp(key) => write!(f, "no property `{key}`"),
            Self::InvalidValue(key) => write!(f, "invalid value for `{key}`"),
            Self::Required(key) => write!(f, "`{key}` can't be empty"),
            Self::Name(err) => err.fmt(f),
            Self::NoSuchView(name) => write!(f, "no view `{name}`"),
        }
    }
}

impl std::error::Error for EditError {}

/// `base`, or `base_2`, `base_3`… — the first one not in `used`.
fn unique_name(base: &str, used: &BTreeSet<String>) -> String {
    if !used.contains(base) {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|name| !used.contains(name))
        .expect("unbounded")
}

impl View {
    /// Every binding name and id salt in use (plus hand-written fields).
    fn used_names(&self) -> BTreeSet<String> {
        let mut used: BTreeSet<String> = self.extra_fields.iter().map(|f| f.name.clone()).collect();
        self.walk(|node| {
            for spec in node.kind.spec().props {
                if matches!(spec.kind, PropKind::Binding(_) | PropKind::IdSalt) {
                    used.insert(node.text(spec.key).to_owned());
                }
            }
        });
        used
    }

    /// A new node of `kind` with fresh ids and its own binding and id salt
    /// (`checked`, `checked_2`…), not yet in the view.
    pub fn create(&mut self, kind: WidgetKind) -> Node {
        let mut node = Node::new(self.alloc_id(), kind);
        self.make_names_unique(&mut node, &mut self.used_names());
        node
    }

    /// Gives `node` and its descendants fresh ids and unique bindings and
    /// id salts, so it can be inserted (a copy, or a node from another view).
    /// Radio buttons and selectable labels keep their binding: copies join
    /// the same group.
    pub fn adopt(&mut self, mut node: Node) -> Node {
        let mut used = self.used_names();
        node.walk_mut(&mut |n| n.id = NodeId(0));
        let mut next = || self.alloc_id();
        node.walk_mut(&mut |n| n.id = next());
        self.make_names_unique(&mut node, &mut used);
        node
    }

    fn make_names_unique(&self, node: &mut Node, used: &mut BTreeSet<String>) {
        node.walk_mut(&mut |n| {
            let grouped = matches!(n.kind, WidgetKind::RadioValue | WidgetKind::SelectableValue);
            for spec in n.kind.spec().props {
                let rename = match spec.kind {
                    PropKind::IdSalt => true,
                    PropKind::Binding(_) => !grouped,
                    _ => false,
                };
                if rename && let Some(Value::Text(name)) = n.props.get_mut(spec.key) {
                    *name = unique_name(name, used);
                    used.insert(name.clone());
                }
            }
        });
    }

    /// Inserts `node` at `index` among the children of `parent` (`None` =
    /// top level; the index is clamped). Returns the id of the inserted
    /// node.
    pub fn insert(
        &mut self,
        parent: Option<NodeId>,
        index: usize,
        node: Node,
    ) -> Result<NodeId, EditError> {
        let parent_kind = self.kind_of(parent)?;
        if !WidgetKind::accepts(parent_kind, node.kind) {
            return Err(EditError::NotAccepted {
                parent: parent_kind,
                child: node.kind,
            });
        }
        let id = node.id;
        let children = self.children_mut(parent).expect("checked above");
        let index = index.min(children.len());
        children.insert(index, node);
        Ok(id)
    }

    /// Whether a node of `kind` can be inserted among the children of
    /// `parent` (`None` = top level).
    pub fn accepts_child(&self, parent: Option<NodeId>, kind: WidgetKind) -> bool {
        let parent_kind = match parent {
            None => None,
            Some(id) => match self.node(id) {
                Some(node) => Some(node.kind),
                None => return false,
            },
        };
        WidgetKind::accepts(parent_kind, kind)
    }

    /// Where a new node of `kind` goes when added from the menus: at the end
    /// of the selected container if it accepts it, else right after the
    /// selected node, else at the end of the view; climbing up through the
    /// ancestors of the selection until a parent accepts it. `None` if no
    /// place accepts it (e.g. an end of row with no grid around).
    pub fn insertion_point(
        &self,
        selected: Option<NodeId>,
        kind: WidgetKind,
    ) -> Option<(Option<NodeId>, usize)> {
        let accepts = |parent: Option<NodeId>| self.accepts_child(parent, kind);
        let mut current = selected.filter(|id| self.node(*id).is_some());
        if let Some(id) = current {
            let node = self.node(id).expect("checked");
            if node.kind.is_container() && accepts(Some(id)) {
                return Some((Some(id), node.children.len()));
            }
        }
        while let Some(id) = current {
            let parent = self.parent(id).expect("node exists");
            if accepts(parent) {
                let index = self
                    .children(parent)
                    .and_then(|c| c.iter().position(|n| n.id == id))
                    .expect("child of parent");
                return Some((parent, index + 1));
            }
            current = parent;
        }
        accepts(None).then_some((None, self.nodes.len()))
    }

    fn kind_of(&self, parent: Option<NodeId>) -> Result<Option<WidgetKind>, EditError> {
        match parent {
            None => Ok(None),
            Some(id) => Ok(Some(self.node(id).ok_or(EditError::NoSuchNode(id))?.kind)),
        }
    }

    /// Removes a node (with its children) and returns it.
    pub fn remove(&mut self, id: NodeId) -> Result<Node, EditError> {
        let parent = self.parent(id).ok_or(EditError::NoSuchNode(id))?;
        let children = self.children_mut(parent).expect("parent exists");
        let index = children
            .iter()
            .position(|n| n.id == id)
            .expect("child of parent");
        Ok(children.remove(index))
    }

    /// Moves a node to `index` among the children of `parent`, where
    /// `index` counts the children *before* the node is taken out (as a
    /// drop indicator between two rows does).
    pub fn move_node(
        &mut self,
        id: NodeId,
        parent: Option<NodeId>,
        index: usize,
    ) -> Result<NodeId, EditError> {
        let node = self.node(id).ok_or(EditError::NoSuchNode(id))?;
        if let Some(parent) = parent {
            let mut inside = false;
            node.walk(&mut |n| inside |= n.id == parent);
            if inside {
                return Err(EditError::IntoItself);
            }
        }
        let parent_kind = self.kind_of(parent)?;
        if !WidgetKind::accepts(parent_kind, node.kind) {
            return Err(EditError::NotAccepted {
                parent: parent_kind,
                child: node.kind,
            });
        }
        let old_parent = self.parent(id).expect("node exists");
        let old_index = self
            .children(old_parent)
            .and_then(|c| c.iter().position(|n| n.id == id))
            .expect("child of parent");
        let index = if old_parent == parent && old_index < index {
            index - 1
        } else {
            index
        };
        let node = self.remove(id)?;
        self.insert(parent, index, node)
    }

    /// Inserts a copy of a node right after it; returns the copy's id.
    pub fn duplicate(&mut self, id: NodeId) -> Result<NodeId, EditError> {
        let node = self.node(id).ok_or(EditError::NoSuchNode(id))?.clone();
        let parent = self.parent(id).expect("node exists");
        let index = self
            .children(parent)
            .and_then(|c| c.iter().position(|n| n.id == id))
            .expect("child of parent");
        let mut copy = self.adopt(node);
        if matches!(
            copy.kind,
            WidgetKind::RadioValue | WidgetKind::SelectableValue
        ) {
            // Next value of the group.
            let binding = copy.text("binding").to_owned();
            let mut max = -1;
            self.walk(|n| {
                if n.kind == copy.kind && n.text("binding") == binding {
                    max = max.max(n.prop("value").and_then(Value::as_int).unwrap_or(0));
                }
            });
            copy.props.insert("value", Value::Int(max + 1));
        }
        self.insert(parent, index + 1, copy)
    }

    /// Changes the window settings.
    pub fn set_window(&mut self, window: WindowSettings) -> Result<(), EditError> {
        if !window.is_valid() {
            return Err(EditError::InvalidValue("window size".to_owned()));
        }
        self.window = window;
        Ok(())
    }

    /// Sets (`Some`) or unsets (`None`) a style property of the whole view
    /// (see [`crate::kind::CONTENT_STYLE`]).
    pub fn set_style(&mut self, key: &str, value: Option<Value>) -> Result<(), EditError> {
        let spec = crate::kind::CONTENT_STYLE
            .iter()
            .find(|s| s.key == key)
            .ok_or_else(|| EditError::NoSuchProp(key.to_owned()))?;
        match value {
            Some(value) if spec.kind.accepts(&value) => {
                self.style.insert(spec.key, value);
            }
            Some(_) => return Err(EditError::InvalidValue(key.to_owned())),
            None => {
                self.style.remove(spec.key);
            }
        }
        Ok(())
    }

    /// Sets (`Some`) or unsets (`None`, optional properties only) a property.
    pub fn set_prop(
        &mut self,
        id: NodeId,
        key: &str,
        value: Option<Value>,
    ) -> Result<(), EditError> {
        let node = self.node_mut(id).ok_or(EditError::NoSuchNode(id))?;
        let spec = node
            .kind
            .prop(key)
            .ok_or_else(|| EditError::NoSuchProp(key.to_owned()))?;
        match value {
            Some(value) => {
                if !spec.kind.accepts(&value) {
                    return Err(EditError::InvalidValue(key.to_owned()));
                }
                node.props.insert(spec.key, value);
            }
            None if spec.optional() => {
                node.props.remove(spec.key);
            }
            None => return Err(EditError::Required(key.to_owned())),
        }
        Ok(())
    }
}

impl Project {
    /// Moves node `id` of view `from` into view `to` (another view), at
    /// `index` among the children of `parent`; it gets fresh ids there.
    /// Returns its new id.
    pub fn move_node_to_view(
        &mut self,
        from: usize,
        id: NodeId,
        to: usize,
        parent: Option<NodeId>,
        index: usize,
    ) -> Result<NodeId, EditError> {
        let missing = |i: usize| EditError::NoSuchView(format!("#{i}"));
        let kind = self
            .views
            .get(from)
            .ok_or(missing(from))?
            .node(id)
            .ok_or(EditError::NoSuchNode(id))?
            .kind;
        let target = self.views.get(to).ok_or(missing(to))?;
        if !target.accepts_child(parent, kind) {
            return Err(EditError::NotAccepted {
                parent: parent.and_then(|p| target.node(p)).map(|n| n.kind),
                child: kind,
            });
        }
        let node = self.views[from].remove(id)?;
        let target = &mut self.views[to];
        let node = target.adopt(node);
        target.insert(parent, index, node)
    }

    fn check_new_view_name(&self, name: &str) -> Result<(), EditError> {
        check_snake_case(name).map_err(EditError::Name)?;
        // `mod.rs` holds the module list.
        if name == "mod" || self.view(name).is_some() {
            return Err(EditError::Name(NameError::Duplicate));
        }
        Ok(())
    }

    /// A name for a new view: `view`, `view_2`…
    pub fn suggest_view_name(&self, base: &str) -> String {
        let used = self.views.iter().map(|v| v.name.clone()).collect();
        unique_name(base, &used)
    }

    /// Adds an empty view at the end, with the default window.
    pub fn add_view(&mut self, name: &str) -> Result<(), EditError> {
        self.add_view_with_window(name, WindowSettings::for_view(name))
    }

    /// Adds an empty view at the end, shown in `window`.
    pub fn add_view_with_window(
        &mut self,
        name: &str,
        window: WindowSettings,
    ) -> Result<(), EditError> {
        self.check_new_view_name(name)?;
        let mut view = View::new(name);
        view.set_window(window)?;
        self.views.push(view);
        Ok(())
    }

    /// Renames a view (its file and struct).
    pub fn rename_view(&mut self, old: &str, new: &str) -> Result<(), EditError> {
        if old == new {
            return Ok(());
        }
        self.check_new_view_name(new)?;
        let view = self
            .view_mut(old)
            .ok_or_else(|| EditError::NoSuchView(old.to_owned()))?;
        view.name = new.to_owned();
        Ok(())
    }

    /// Removes a view and returns it.
    pub fn remove_view(&mut self, name: &str) -> Result<View, EditError> {
        let index = self
            .views
            .iter()
            .position(|v| v.name == name)
            .ok_or_else(|| EditError::NoSuchView(name.to_owned()))?;
        Ok(self.views.remove(index))
    }

    /// Copies a view under a new name, right after it.
    pub fn duplicate_view(&mut self, name: &str, new: &str) -> Result<(), EditError> {
        self.check_new_view_name(new)?;
        let index = self
            .views
            .iter()
            .position(|v| v.name == name)
            .ok_or_else(|| EditError::NoSuchView(name.to_owned()))?;
        let mut copy = self.views[index].clone();
        copy.name = new.to_owned();
        self.views.insert(index + 1, copy);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use WidgetKind as K;

    fn view_with(kinds: &[WidgetKind]) -> (View, Vec<NodeId>) {
        let mut view = View::new("test");
        let ids = kinds
            .iter()
            .map(|&kind| {
                let node = view.create(kind);
                view.insert(None, usize::MAX, node).unwrap()
            })
            .collect();
        (view, ids)
    }

    #[test]
    fn create_gives_unique_bindings_and_salts() {
        let (view, ids) = view_with(&[K::Checkbox, K::Checkbox, K::Grid, K::Grid]);
        let text = |i: usize, key| view.node(ids[i]).unwrap().text(key).to_owned();
        assert_eq!(text(0, "binding"), "checked");
        assert_eq!(text(1, "binding"), "checked_2");
        assert_eq!(text(2, "id"), "grid");
        assert_eq!(text(3, "id"), "grid_2");
        let (fields, conflicts) = view.fields();
        assert_eq!(fields.len(), 2);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn insert_checks_containment() {
        let (mut view, ids) = view_with(&[K::Plot, K::Label, K::Stack]);
        let end_row = view.create(K::EndRow);
        assert!(matches!(
            view.insert(None, 0, end_row),
            Err(EditError::NotAccepted { .. })
        ));
        let label = view.create(K::Label);
        assert!(
            view.insert(Some(ids[0]), 0, label).is_err(),
            "a plot only holds series"
        );
        let line = view.create(K::PlotLine);
        view.insert(Some(ids[0]), 0, line).unwrap();
        let item = view.create(K::StackItem);
        view.insert(Some(ids[2]), 0, item).unwrap();
        let child = view.create(K::Button);
        assert!(
            view.insert(Some(ids[1]), 0, child).is_err(),
            "a label isn't a container"
        );
    }

    #[test]
    fn move_within_and_between_parents() {
        let (mut view, ids) = view_with(&[K::Label, K::Button, K::Stack, K::Separator]);
        // Drop the label between the button and the vertical (index 2 before removal).
        view.move_node(ids[0], None, 2).unwrap();
        let kinds: Vec<_> = view.nodes.iter().map(|n| n.kind).collect();
        assert_eq!(kinds, [K::Button, K::Label, K::Stack, K::Separator]);
        view.move_node(ids[3], Some(ids[2]), 0).unwrap();
        assert_eq!(view.parent(ids[3]), Some(Some(ids[2])));
        assert_eq!(
            view.move_node(ids[2], Some(ids[2]), 0),
            Err(EditError::IntoItself)
        );
        assert_eq!(view.path(ids[3]), Some(vec![2, 0]));
    }

    #[test]
    fn duplicate_and_remove() {
        let (mut view, ids) = view_with(&[K::RadioValue, K::Checkbox]);
        let radio = view.duplicate(ids[0]).unwrap();
        let copy = view.node(radio).unwrap();
        assert_eq!(
            copy.text("binding"),
            "choice",
            "radio copies join the group"
        );
        assert_eq!(copy.prop("value"), Some(&Value::Int(1)));
        let check = view.duplicate(ids[1]).unwrap();
        assert_eq!(view.node(check).unwrap().text("binding"), "checked_2");
        assert_eq!(view.nodes.len(), 4);
        view.remove(radio).unwrap();
        assert!(view.node(radio).is_none());
        assert!(view.remove(radio).is_err());
    }

    #[test]
    fn insertion_points() {
        let (mut view, ids) = view_with(&[K::Label, K::Grid, K::Plot]);
        let (label, grid, plot) = (ids[0], ids[1], ids[2]);
        assert_eq!(view.insertion_point(None, K::Button), Some((None, 3)));
        assert_eq!(
            view.insertion_point(Some(label), K::Button),
            Some((None, 1))
        );
        assert_eq!(
            view.insertion_point(Some(grid), K::Button),
            Some((Some(grid), 0))
        );
        assert_eq!(view.insertion_point(None, K::EndRow), None);
        assert_eq!(
            view.insertion_point(Some(grid), K::EndRow),
            Some((Some(grid), 0))
        );
        // A widget selected in a grid: an end of row goes after it.
        let cell = view.create(K::Label);
        let cell = view.insert(Some(grid), 0, cell).unwrap();
        assert_eq!(
            view.insertion_point(Some(cell), K::EndRow),
            Some((Some(grid), 1))
        );
        // A plot takes series; other widgets go after it.
        assert_eq!(
            view.insertion_point(Some(plot), K::PlotLine),
            Some((Some(plot), 0))
        );
        assert_eq!(view.insertion_point(Some(plot), K::Button), Some((None, 3)));
        assert_eq!(view.insertion_point(Some(label), K::PlotPoints), None);
    }

    #[test]
    fn move_between_views() {
        let mut project = Project::default();
        project.add_view("a").unwrap();
        project.add_view("b").unwrap();
        let checkbox = project.views[0].create(K::Checkbox);
        let id = project.views[0].insert(None, 0, checkbox).unwrap();
        let grid = project.views[1].create(K::Grid);
        let grid = project.views[1].insert(None, 0, grid).unwrap();
        let moved = project.move_node_to_view(0, id, 1, Some(grid), 0).unwrap();
        assert!(project.views[0].nodes.is_empty());
        assert_eq!(project.views[1].parent(moved), Some(Some(grid)));
        let end_row = project.views[1].create(K::EndRow);
        let end_row = project.views[1].insert(Some(grid), 1, end_row).unwrap();
        assert!(
            project.move_node_to_view(1, end_row, 0, None, 0).is_err(),
            "an end of row needs a grid"
        );
        assert!(
            project.views[1].node(end_row).is_some(),
            "a refused move keeps the node"
        );
    }

    #[test]
    fn set_prop_validates() {
        let (mut view, ids) = view_with(&[K::Slider]);
        let id = ids[0];
        view.set_prop(id, "text", Some(Value::Text("Volume".into())))
            .unwrap();
        view.set_prop(id, "text", None).unwrap();
        assert_eq!(view.node(id).unwrap().prop("text"), None);
        assert_eq!(
            view.set_prop(id, "binding", None),
            Err(EditError::Required("binding".into()))
        );
        assert!(
            view.set_prop(id, "binding", Some(Value::Text("2x".into())))
                .is_err()
        );
        assert!(view.set_prop(id, "step", Some(Value::Bool(true))).is_err());
        assert!(view.set_prop(id, "nope", None).is_err());
    }

    #[test]
    fn binding_conflicts() {
        let (mut view, ids) = view_with(&[K::Checkbox, K::Slider]);
        view.set_prop(ids[1], "binding", Some(Value::Text("checked".into())))
            .unwrap();
        let (fields, conflicts) = view.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].node, ids[1]);
    }

    #[test]
    fn views() {
        let mut project = Project::default();
        project.add_view("login").unwrap();
        assert!(project.add_view("login").is_err());
        assert!(project.add_view("Login").is_err());
        assert!(project.add_view("mod").is_err());
        project.duplicate_view("login", "login_2").unwrap();
        project.rename_view("login", "sign_in").unwrap();
        assert_eq!(project.suggest_view_name("login"), "login");
        let names: Vec<_> = project.views.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["sign_in", "login_2"]);
        project.remove_view("sign_in").unwrap();
        assert_eq!(project.views.len(), 1);
    }
}
