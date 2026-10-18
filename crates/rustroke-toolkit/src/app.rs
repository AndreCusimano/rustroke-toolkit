//! The application state and the frame loop.

use std::collections::HashMap;
use std::path::PathBuf;

use rtk_document::{EditError, Node, NodeId, Project, View, WidgetKind, WindowSettings};
use rtk_preview::{CanvasState, Mode};
use rustroke::{App, CentralPanel, Frame, Key, KeyboardShortcut, Modifiers, Panel, Toast};

use crate::history::{History, Snapshot};
use crate::icons::Icons;
use crate::panels;
use crate::project_io::{self, Then};

/// Shortcuts, shown in the menus.
pub const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
/// Duplicate the selected node.
pub const DUPLICATE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
/// Delete the selected node.
pub const DELETE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
/// Undo the last change.
pub const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
/// Redo the last undone change.
pub const REDO: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);
/// Cut the selected node.
pub const CUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::X);
/// Copy the selected node.
pub const COPY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
/// Paste the copied node.
pub const PASTE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);

/// What is being dragged.
#[derive(Clone, Debug, PartialEq)]
pub enum DragPayload {
    /// A new widget, from the palette.
    New(WidgetKind),
    /// A node of view `view` (index), from the component tree.
    Node { view: usize, id: NodeId },
}

/// A dialog in front of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    /// A new view: its name and the window it is shown in. The title
    /// follows the name until it is edited.
    NewView {
        name: String,
        window: WindowSettings,
        title_edited: bool,
    },
    /// New name of a view.
    RenameView { old: String, name: String },
    /// Confirm deleting a view.
    DeleteView { name: String },
    /// Save the changes before `then`?
    UnsavedChanges { then: Then },
    /// Delete the files of views removed or renamed since the last save?
    StaleFiles { dir: PathBuf, files: Vec<PathBuf> },
}

/// A side panel of the main window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// The widget palette.
    Widgets,
    /// The views and their component trees.
    Views,
    /// The properties of the selection, or of the view.
    Properties,
    /// The generated code of the view.
    Code,
}

impl Section {
    /// Every section, in the order they are stored.
    pub const ALL: [Self; 4] = [Self::Widgets, Self::Views, Self::Properties, Self::Code];

    /// The sections on the left edge, from the edge inwards.
    pub const LEFT: [Self; 2] = [Self::Widgets, Self::Views];

    /// The sections on the right edge, from the edge inwards.
    pub const RIGHT: [Self; 2] = [Self::Properties, Self::Code];

    /// The name shown in the interface and read by screen readers.
    pub fn title(self) -> &'static str {
        match self {
            Self::Widgets => "Widgets",
            Self::Views => "Views",
            Self::Properties => "Properties",
            Self::Code => "Code",
        }
    }

    /// The key in the saved state.
    fn key(self) -> &'static str {
        match self {
            Self::Widgets => "widgets",
            Self::Views => "views",
            Self::Properties => "properties",
            Self::Code => "code",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Widgets => 0,
            Self::Views => 1,
            Self::Properties => 2,
            Self::Code => 3,
        }
    }
}

/// Which side panels are open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sections {
    open: [bool; 4],
}

impl Default for Sections {
    fn default() -> Self {
        Self { open: [true; 4] }
    }
}

impl Sections {
    /// Whether `section` is open.
    pub fn is_open(&self, section: Section) -> bool {
        self.open[section.index()]
    }

    /// Shows `section` if it is hidden, hides it if it is shown.
    pub fn toggle(&mut self, section: Section) {
        let open = &mut self.open[section.index()];
        *open = !*open;
    }

    /// The closed sections, as written in the saved state.
    fn stored(&self) -> String {
        Section::ALL
            .iter()
            .filter(|section| !self.is_open(**section))
            .map(|section| section.key())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Reads the saved state; missing and unknown sections are open.
    fn parse(stored: &str) -> Self {
        let mut sections = Self::default();
        for key in stored.split(',') {
            if let Some(section) = Section::ALL.into_iter().find(|s| s.key() == key) {
                sections.open[section.index()] = false;
            }
        }
        sections
    }
}

/// The whole application.
#[derive(Debug)]
pub struct Toolkit {
    /// The project being edited.
    pub project: Project,
    /// Folder the project was last saved to or opened from.
    pub path: Option<PathBuf>,
    /// Index of the view shown on the canvas.
    pub current: usize,
    /// Selected node of the current view.
    pub selected: Option<NodeId>,
    /// Design or preview.
    pub mode: Mode,
    /// Canvas state (binding values, node areas) per view name.
    pub canvases: HashMap<String, CanvasState>,
    /// Where each view's window sits on the canvas, by view name.
    pub window_positions: HashMap<String, rustroke::Vec2>,
    /// Incremented on every change of the project.
    pub revision: u64,
    /// The project changed since it was last saved.
    pub dirty: bool,
    /// The code panel.
    pub code: panels::code::CodePanel,
    /// The properties panel.
    pub properties: panels::properties::PropertiesPanel,
    /// The widget palette.
    pub palette: panels::palette::PalettePanel,
    /// Which side panels are open.
    pub sections: Sections,
    /// Open dialog.
    pub dialog: Option<Dialog>,
    /// Message in the status bar.
    pub status: String,
    /// Dark or light theme.
    pub dark: bool,
    /// Undo and redo steps.
    pub history: History,
    /// The node copied or cut, with its children.
    pub clipboard: Option<Node>,
    /// Time of the current frame, in seconds.
    pub now: f64,
    /// Notifications to show next frame.
    pub toasts: Vec<Toast>,
    /// Recently opened or saved project folders, most recent first.
    pub recent: Vec<PathBuf>,
    /// `recent` changed and must be stored.
    pub recent_changed: bool,
    /// What to do after the current save succeeds.
    pub after_save: Option<Then>,
    /// Close the window this frame (the user agreed).
    pub close: bool,
    /// The recent projects were read from the saved state.
    loaded_state: bool,
    /// The theme set in the context (`Some(true)` = dark).
    theme_applied: Option<bool>,
    /// The icons, loaded at the first frame.
    icons: Option<Icons>,
}

impl Dialog {
    /// The "New view" dialog with a free name.
    pub fn new_view(project: &Project) -> Self {
        let name = project.suggest_view_name("view");
        Self::NewView {
            window: WindowSettings::for_view(&name),
            name,
            title_edited: false,
        }
    }
}

impl Toolkit {
    /// The app with `project` open.
    pub fn new(project: Project) -> Self {
        Self {
            project,
            path: None,
            current: 0,
            selected: None,
            mode: Mode::Design,
            canvases: HashMap::new(),
            window_positions: HashMap::new(),
            revision: 1,
            dirty: false,
            code: panels::code::CodePanel::default(),
            properties: panels::properties::PropertiesPanel::default(),
            palette: panels::palette::PalettePanel::default(),
            sections: Sections::default(),
            dialog: None,
            status: "Ready".to_owned(),
            dark: true,
            history: History::default(),
            clipboard: None,
            now: 0.0,
            toasts: Vec::new(),
            recent: Vec::new(),
            recent_changed: false,
            after_save: None,
            close: false,
            loaded_state: false,
            theme_applied: None,
            icons: None,
        }
    }

    /// The icons (loaded before any panel is drawn).
    pub fn icons(&self) -> Icons {
        self.icons.expect("icons are loaded at the first frame")
    }

    /// The view shown on the canvas.
    pub fn view(&self) -> Option<&View> {
        self.project.views.get(self.current)
    }

    /// Records a change made outside [`Toolkit::edit`] (e.g. a parsed view).
    pub fn touch(&mut self) {
        self.revision += 1;
        self.dirty = true;
    }

    /// The state an undo step restores.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            project: self.project.clone(),
            current: self.current,
            selected: self.selected,
        }
    }

    /// Changes the current view through `f`; on success the project is
    /// marked changed (one undo step, unless nothing changed), on error the
    /// status bar shows why.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut View) -> Result<R, EditError>) -> Option<R> {
        self.edit_tagged(None, f)
    }

    /// Like [`Toolkit::edit`]; changes with the same `tag` in quick
    /// succession (typing in a field) make one undo step.
    pub fn edit_tagged<R>(
        &mut self,
        tag: Option<&str>,
        f: impl FnOnce(&mut View) -> Result<R, EditError>,
    ) -> Option<R> {
        let before = self.snapshot();
        let view = self.project.views.get_mut(self.current)?;
        match f(view) {
            Ok(result) => {
                self.record(before, tag);
                Some(result)
            }
            Err(err) => {
                self.status = err.to_string();
                None
            }
        }
    }

    /// Changes the project (views list) through `f`.
    pub fn edit_project<R>(
        &mut self,
        f: impl FnOnce(&mut Project) -> Result<R, EditError>,
    ) -> Option<R> {
        let before = self.snapshot();
        match f(&mut self.project) {
            Ok(result) => {
                self.record(before, None);
                self.current = self.current.min(self.project.views.len().saturating_sub(1));
                Some(result)
            }
            Err(err) => {
                self.status = err.to_string();
                None
            }
        }
    }

    /// After a successful change: an undo step to `before`, if the project
    /// changed (a widget dropped where it was changes nothing).
    fn record(&mut self, before: Snapshot, tag: Option<&str>) {
        if self.project != before.project {
            self.history.record(before, tag, self.now);
            self.touch();
        }
    }

    /// Puts back a state from the history.
    fn restore(&mut self, state: Snapshot) {
        self.project = state.project;
        self.current = state
            .current
            .min(self.project.views.len().saturating_sub(1));
        self.selected = state
            .selected
            .filter(|id| self.view().is_some_and(|v| v.node(*id).is_some()));
        self.touch();
    }

    /// Takes back the last change.
    pub fn undo(&mut self) {
        let current = self.snapshot();
        match self.history.undo(current) {
            Some(state) => {
                self.restore(state);
                self.status = "Undone".to_owned();
            }
            None => self.status = "Nothing to undo".to_owned(),
        }
    }

    /// Redoes the last undone change.
    pub fn redo(&mut self) {
        let current = self.snapshot();
        match self.history.redo(current) {
            Some(state) => {
                self.restore(state);
                self.status = "Redone".to_owned();
            }
            None => self.status = "Nothing to redo".to_owned(),
        }
    }

    /// Copies the selected node (with its children).
    pub fn copy_selected(&mut self) {
        let Some(node) = self.selected.and_then(|id| self.view()?.node(id)).cloned() else {
            return;
        };
        self.status = format!("Copied {}", node.kind.name());
        self.clipboard = Some(node);
    }

    /// Copies the selected node, then deletes it.
    pub fn cut_selected(&mut self) {
        self.copy_selected();
        if self.clipboard.is_some() && self.selected.is_some() {
            self.delete_selected();
            if let Some(node) = &self.clipboard {
                self.status = format!("Cut {}", node.kind.name());
            }
        }
    }

    /// Pastes the copied node where a new widget would go (inside the
    /// selected container, or after the selection), and selects it.
    pub fn paste(&mut self) {
        let Some(node) = self.clipboard.clone() else {
            self.status = "Nothing to paste".to_owned();
            return;
        };
        let selected = self.selected;
        let kind = node.kind;
        let pasted = self.edit(|view| {
            let (parent, index) =
                view.insertion_point(selected, kind)
                    .ok_or(EditError::NotAccepted {
                        parent: selected.and_then(|id| view.node(id)).map(|n| n.kind),
                        child: kind,
                    })?;
            let node = view.adopt(node);
            let id = node.id;
            view.insert(parent, index, node)?;
            Ok(id)
        });
        if let Some(id) = pasted {
            self.selected = Some(id);
            self.status = format!("Pasted {}", kind.name());
        }
    }

    /// Shows view `index` on the canvas.
    pub fn open_view(&mut self, index: usize) {
        if index != self.current {
            self.current = index;
            self.selected = None;
        }
    }

    /// Adds a node of `kind` where [`View::insertion_point`] says, and
    /// selects it.
    pub fn add_widget(&mut self, kind: WidgetKind) {
        let selected = self.selected;
        let added = self.edit(|view| {
            let (parent, index) =
                view.insertion_point(selected, kind)
                    .ok_or(EditError::NotAccepted {
                        parent: selected.and_then(|id| view.node(id)).map(|n| n.kind),
                        child: kind,
                    })?;
            let node = view.create(kind);
            let id = node.id;
            view.insert(parent, index, node)?;
            Ok(id)
        });
        if let Some(id) = added {
            self.selected = Some(id);
            self.status = format!("Added {}", kind.name());
        }
    }

    /// Whether a node of `kind` can be added now.
    pub fn can_add(&self, kind: WidgetKind) -> bool {
        self.view()
            .is_some_and(|view| view.insertion_point(self.selected, kind).is_some())
    }

    /// The kind of node a payload would insert.
    pub fn payload_kind(&self, payload: &DragPayload) -> Option<WidgetKind> {
        match payload {
            DragPayload::New(kind) => Some(*kind),
            DragPayload::Node { view, id } => Some(self.project.views.get(*view)?.node(*id)?.kind),
        }
    }

    /// Drops a payload at `index` among the children of `parent` in view
    /// `view` (`index` counted before a moved node is taken out), and
    /// selects the result.
    pub fn drop_payload(
        &mut self,
        payload: &DragPayload,
        view: usize,
        parent: Option<NodeId>,
        index: usize,
    ) {
        let dropped = match *payload {
            DragPayload::New(kind) => {
                self.open_view(view);
                self.edit(|v| {
                    let node = v.create(kind);
                    let id = node.id;
                    v.insert(parent, index, node)?;
                    Ok(id)
                })
            }
            DragPayload::Node { view: from, id } if from == view => {
                self.open_view(view);
                if parent == Some(id) {
                    return;
                }
                // Within a view the node keeps its id (a Flex may wrap it).
                self.edit(|v| v.move_node(id, parent, index)).map(|_| id)
            }
            DragPayload::Node { view: from, id } => {
                let moved =
                    self.edit_project(|p| p.move_node_to_view(from, id, view, parent, index));
                self.open_view(view);
                moved
            }
        };
        if let Some(id) = dropped {
            self.selected = Some(id);
        }
    }

    /// Deletes the selected node.
    pub fn delete_selected(&mut self) {
        let Some(id) = self.selected else { return };
        // Select the next sibling, else the previous one, else the parent.
        let next = self.view().and_then(|view| {
            let parent = view.parent(id)?;
            let siblings = view.children(parent)?;
            let i = siblings.iter().position(|n| n.id == id)?;
            siblings
                .get(i + 1)
                .or_else(|| i.checked_sub(1).and_then(|i| siblings.get(i)))
                .map(|n| n.id)
                .or(parent)
        });
        if let Some(node) = self.edit(|view| view.remove(id)) {
            self.status = format!("Deleted {}", node.kind.name());
            self.selected = next;
        }
    }

    /// Duplicates the selected node and selects the copy.
    pub fn duplicate_selected(&mut self) {
        let Some(id) = self.selected else { return };
        if let Some(copy) = self.edit(|view| view.duplicate(id)) {
            self.selected = Some(copy);
        }
    }

    /// Moves the selected node one place up (`-1`) or down (`1`) among its
    /// siblings.
    pub fn move_selected(&mut self, delta: isize) {
        let Some(id) = self.selected else { return };
        self.edit(|view| {
            let parent = view.parent(id).ok_or(EditError::NoSuchNode(id))?;
            let siblings = view.children(parent).expect("parent exists");
            let i = siblings.iter().position(|n| n.id == id).expect("child");
            // `move_node` counts positions before removing the node.
            let target = match delta {
                d if d < 0 => i.checked_sub(1),
                _ => (i + 1 < siblings.len()).then_some(i + 2),
            };
            match target {
                Some(target) => view.move_node(id, parent, target),
                None => Ok(id),
            }
        });
    }

    fn shortcuts(&mut self, frame: &mut Frame<'_>) {
        if frame.ctx().wants_keyboard_input() || frame.ctx().is_modal_open() {
            return;
        }
        let input = frame.ctx().input_mut();
        let delete =
            input.consume_shortcut(&DELETE) || input.consume_key(Key::Backspace, Modifiers::NONE);
        let duplicate = input.consume_shortcut(&DUPLICATE);
        let save = input.consume_shortcut(&SAVE);
        let redo = input.consume_shortcut(&REDO);
        let undo = input.consume_shortcut(&UNDO);
        let cut = input.consume_shortcut(&CUT);
        let copy = input.consume_shortcut(&COPY);
        let paste = input.consume_shortcut(&PASTE);
        let deselect = input.consume_key(Key::Escape, Modifiers::NONE);
        if delete {
            self.delete_selected();
        }
        if duplicate {
            self.duplicate_selected();
        }
        if deselect {
            self.selected = None;
        }
        if save {
            project_io::save(self);
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if cut {
            self.cut_selected();
        }
        if copy {
            self.copy_selected();
        }
        if paste {
            self.paste();
        }
    }

    fn title(&self) -> String {
        let name = if self.project.name.is_empty() {
            "untitled"
        } else {
            &self.project.name
        };
        let dirty = if self.dirty { " •" } else { "" };
        format!("{name}{dirty} — Rustroke Toolkit")
    }
}

/// Key of the recent projects in the saved state (one path per line).
const RECENT_KEY: &str = "recent_projects";

/// Key of the closed side panels in the saved state (comma separated).
const SECTIONS_KEY: &str = "closed_sections";

impl Toolkit {
    /// Reads and writes the state kept between runs.
    fn persist(&mut self, frame: &mut Frame<'_>) {
        if !self.loaded_state {
            self.loaded_state = true;
            self.recent = frame
                .value(RECENT_KEY)
                .unwrap_or("")
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect();
            self.dark = frame.value("dark_theme") != Some("false");
            self.sections = Sections::parse(frame.value(SECTIONS_KEY).unwrap_or(""));
            // Reopen the last project, if it still opens.
            if let Some(dir) = self.recent.first().cloned()
                && rtk_document::project::load_project(&dir).is_ok()
            {
                project_io::open(self, &dir);
            }
        }
        if self.recent_changed {
            self.recent_changed = false;
            let lines: Vec<String> = self
                .recent
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            frame.set_value(RECENT_KEY, lines.join("\n"));
        }
        let dark = if self.dark { "true" } else { "false" };
        if frame.value("dark_theme") != Some(dark) {
            frame.set_value("dark_theme", dark);
        }
        let sections = self.sections.stored();
        if frame.value(SECTIONS_KEY) != Some(sections.as_str()) {
            frame.set_value(SECTIONS_KEY, &sections);
        }
    }
}

impl App for Toolkit {
    fn on_close_requested(&mut self) -> bool {
        if self.dirty && !self.close {
            self.dialog = Some(Dialog::UnsavedChanges { then: Then::Close });
            return false;
        }
        true
    }

    fn update(&mut self, frame: &mut Frame<'_>) {
        self.now = frame.time.as_secs_f64();
        if self.icons.is_none() {
            self.icons = Some(Icons::load(frame.fonts()));
        }
        self.persist(frame);
        if self.close {
            frame.close();
        }
        for toast in self.toasts.drain(..) {
            frame.ctx().toast(toast);
        }
        if self.theme_applied != Some(self.dark) {
            self.theme_applied = Some(self.dark);
            let style = if self.dark {
                crate::theme::dark()
            } else {
                crate::theme::light()
            };
            frame.ctx().set_style(style);
        }
        frame.clear_color = frame.ctx().style().visuals.background;
        frame.set_title(self.title());
        self.shortcuts(frame);

        Panel::top("menu").show(frame, |ui| panels::toolbar::show(ui, self));
        Panel::bottom("status").show(frame, |ui| panels::status::show(ui, self));
        // The rails stay at the window edges and show or hide the panels
        // inside them.
        Panel::left("rail left")
            .default_size(32.0)
            .resizable(false)
            .min_size(32.0)
            .max_size(32.0)
            .margin(0.0)
            .show(frame, |ui| {
                panels::rail::show(ui, self, &Section::LEFT);
            });
        if self.sections.is_open(Section::Widgets) {
            Panel::left("palette")
                .default_size(240.0)
                .resizable(true)
                .min_size(180.0)
                .max_size(420.0)
                .show(frame, |ui| panels::palette::show(ui, self));
        }
        if self.sections.is_open(Section::Views) {
            Panel::left("views")
                .default_size(240.0)
                .resizable(true)
                .min_size(180.0)
                .max_size(420.0)
                .show(frame, |ui| panels::views::show(ui, self));
        }
        Panel::right("rail right")
            .default_size(32.0)
            .resizable(false)
            .min_size(32.0)
            .max_size(32.0)
            .margin(0.0)
            .show(frame, |ui| {
                panels::rail::show(ui, self, &Section::RIGHT);
            });
        let now = frame.time.as_secs_f64();
        if self.sections.is_open(Section::Code) {
            Panel::right("code")
                .default_size(440.0)
                .resizable(true)
                .min_size(280.0)
                .max_size(900.0)
                .show(frame, |ui| panels::code::show(ui, self, now));
        }
        if self.sections.is_open(Section::Properties) {
            Panel::right("properties")
                .default_size(280.0)
                .resizable(true)
                .min_size(240.0)
                .max_size(520.0)
                .show(frame, |ui| panels::properties::show(ui, self));
        }
        // The canvas fills the central area edge to edge.
        CentralPanel::default()
            .margin(0.0)
            .show(frame, |ui| panels::canvas::show(ui, self));
        panels::dialogs::show(frame, self);
    }
}
