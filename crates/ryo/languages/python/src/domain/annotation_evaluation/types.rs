//! Values of annotation evaluation timing: plain data, public fields, no constructor.
use ryo::languages::Position;

/// When the language evaluates an annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Evaluation {
    /// Evaluated when its definition or statement executes (through Python 3.13 without the annotations future).
    Eager,
    /// Stored as a string under the annotations future (PEP 563).
    Postponed,
    /// Stored as a native lazy annotation (Python 3.14 onward without the future; PEP 649, PEP 749).
    Deferred,
    /// Never evaluated: a function-local variable annotation, or a non-simple variable annotation in a lazy file.
    Unevaluated,
}

/// Whether the file stores annotations without evaluating them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// The annotations future, or Python 3.14 onward.
    Lazy,
    /// Otherwise.
    Eager,
}

/// One element of one annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnnotationMember {
    /// The element.
    pub id: Position,
    /// The annotation expression it belongs to.
    pub root: Position,
    /// When the annotation is evaluated.
    pub evaluation: Evaluation,
    /// The file's annotation mode.
    pub mode: Mode,
}

/// Capacity-retaining rows of the current file, rebuilt on each read of the relation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AnnotationEvaluationScratch {
    /// Rows in tree order of their annotation, then of the element.
    pub members: Vec<AnnotationMember>,
}
