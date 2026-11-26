//! The types of this concept.
use ryo::languages::Position;

/// What the file's compiler features and target say about when annotations are evaluated.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Target {
    /// The file states whether annotations are postponed.
    pub(crate) known: bool,
    /// The file postpones them.
    pub(crate) enabled: bool,
    /// The file states it does not.
    pub(crate) disabled: bool,
    /// The target evaluates annotations lazily on its own.
    pub(crate) native: bool,
    /// The target evaluates them with their definition.
    pub(crate) immediate: bool,
}

/// One row: an element inside an annotation, the annotation, when it is evaluated and in which mode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Row {
    /// The element.
    pub(crate) id: Position,
    /// The annotation it belongs to.
    pub(crate) root: Position,
    /// Its evaluation.
    pub(crate) evaluation: &'static str,
    /// The mode of the answer.
    pub(crate) mode: &'static str,
}
