//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language works out of a file once, before its readers run.

use crate::domain::indentation::types::IndentationIndex;
use ryo::languages::{Position, Symbol};
use std::cell::RefCell;

/// Where a scope says a name it assigns is bound instead of in itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingRedirect {
    /// `global`: in the module.
    Global,
    /// `nonlocal`: in the nearest enclosing function.
    Nonlocal,
}

/// One name a scope declares `global` or `nonlocal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScopeDeclaration {
    /// The scope the declaration is in.
    pub scope: Position,
    /// The name, as the file's names hold it.
    pub name: Symbol,
    /// Where the scope binds it.
    pub redirect: BindingRedirect,
}

/// A name within one lexical scope, independent of its declaration position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BindingKey {
    /// The lexical scope owning the declaration.
    pub scope: Position,
    /// The interned name within that scope.
    pub name: Symbol,
}

/// One declaration ordered by lexical scope, name, and source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LexicalBinding {
    /// The lexical scope and interned name.
    pub key: BindingKey,
    /// The declaration position in source order.
    pub position: Position,
}

/// File-local scope redirects and indexed declarations, shared by relation readers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Declarations {
    /// Lazy canonical expression classes, invalidated between source revisions.
    pub expression_shapes: RefCell<crate::domain::expression_shapes::types::ExpressionShapeIndex>,
    /// Every declared name.
    pub declared: Vec<ScopeDeclaration>,
    /// Definitions indexed by lexical scope and name, in source order.
    pub bindings: Vec<LexicalBinding>,
    /// Lazy lexical indentation regions retain their storage between files.
    pub indentation: RefCell<IndentationIndex>,
    /// Lazy provenance uncertainty, invalidated with every parse.
    pub qualification: RefCell<QualificationIndex>,
    /// Enabled embedded-expression roots, computed only for semantic readers.
    pub semantic: RefCell<SemanticIndex>,
}

/// A lexical event that can invalidate otherwise known import or builtin provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QualificationHazardKind {
    /// A wildcard import may bind any name.
    Wildcard,
    /// Deletion removes one binding without introducing a new declaration.
    Delete(Symbol),
}
/// An event scoped and ordered independently of any linter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QualificationHazard {
    /// The namespace modified by the event.
    pub scope: Position,
    /// The source position of the event.
    pub position: Position,
    /// The event's effect on lexical provenance.
    pub kind: QualificationHazardKind,
}
/// Whether retained provenance events belong to the current parsed model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum QualificationState {
    /// The current source has not requested provenance.
    #[default]
    Fresh,
    /// Events have been indexed for the current source.
    Ready,
}
/// Lazily indexed uncertainty events; capacity survives reparsing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QualificationIndex {
    /// Source-ordered namespace events.
    pub hazards: Vec<QualificationHazard>,
    /// Cache validity for the current file.
    pub state: QualificationState,
}

/// Retained selection of annotation expressions after lexical context classification.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SemanticIndex {
    /// The known enabled roots, in parser append order.
    pub roots: Vec<Position>,
    /// Whether these roots belong to the current parse.
    pub state: QualificationState,
}
