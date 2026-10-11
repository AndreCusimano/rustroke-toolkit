//! Undo and redo: snapshots of the project taken before each change.
//!
//! Changes made in quick succession to the same thing (typing in a
//! property field, dragging a window's size, editing the code) are merged
//! into one step, so one undo takes back the whole edit.

use rtk_document::{NodeId, Project};

/// Most steps kept.
const MAX_STEPS: usize = 200;

/// Seconds within which changes with the same tag merge into one step.
const MERGE_WINDOW: f64 = 1.0;

/// The state an undo step restores.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The whole project.
    pub project: Project,
    /// The view shown.
    pub current: usize,
    /// The selected node.
    pub selected: Option<NodeId>,
}

/// Undo and redo stacks.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Tag and time of the last recorded change.
    last: Option<(String, f64)>,
}

impl History {
    /// Records `before`, the state before a change. With a `tag`, a change
    /// right after another with the same tag joins its step.
    pub fn record(&mut self, before: Snapshot, tag: Option<&str>, now: f64) {
        self.redo.clear();
        let merge = match (tag, &self.last) {
            (Some(tag), Some((last, at))) => tag == last && now - at < MERGE_WINDOW,
            _ => false,
        };
        self.last = tag.map(|tag| (tag.to_owned(), now));
        if merge && !self.undo.is_empty() {
            return;
        }
        self.undo.push(before);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
    }

    /// Takes the last step back: returns the state to restore, given the
    /// current one (which redo will restore).
    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        self.last = None;
        Some(previous)
    }

    /// Redoes the last undone step.
    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        self.last = None;
        Some(next)
    }

    /// Whether there is something to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is something to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Forgets everything (another project was opened).
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(name: &str) -> Snapshot {
        Snapshot {
            project: Project {
                name: name.to_owned(),
                views: Vec::new(),
            },
            current: 0,
            selected: None,
        }
    }

    #[test]
    fn undo_redo_and_merging() {
        let mut h = History::default();
        h.record(snap("a"), None, 0.0);
        h.record(snap("b"), Some("prop"), 1.0);
        h.record(snap("c"), Some("prop"), 1.5); // merged into "b"
        h.record(snap("d"), Some("prop"), 3.0); // too late: new step
        assert_eq!(h.undo(snap("e")).unwrap().project.name, "d");
        assert_eq!(h.undo(snap("d")).unwrap().project.name, "b");
        assert_eq!(h.redo(snap("b")).unwrap().project.name, "d");
        assert!(h.can_redo());
        h.record(snap("x"), None, 5.0);
        assert!(!h.can_redo(), "a new change drops the redo steps");
        assert_eq!(h.undo(snap("y")).unwrap().project.name, "x");
        assert_eq!(h.undo(snap("x")).unwrap().project.name, "b");
        assert_eq!(h.undo(snap("b")).unwrap().project.name, "a");
        assert!(!h.can_undo());
    }
}
