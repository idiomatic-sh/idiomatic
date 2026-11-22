//! The types of the model's `definitions` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::model::lexical::types::NamespaceKey;
use crate::domain::model::names::types::{KeyHasher, NameCode};
use ryo::languages::{Count, DeclarationKind, Position, Span};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// A definition of the file, by its place in source order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DefinitionCode(pub u32);

impl From<DefinitionCode> for usize {
    fn from(code: DefinitionCode) -> Self {
        Self::try_from(code.0).unwrap_or(Self::MAX)
    }
}

/// What a definition is, more finely than its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefinitionWord {
    /// A `def` outside a class.
    Function,
    /// An `async def` outside a class.
    AsyncFunction,
    /// A `def` in a class.
    Method,
    /// An `async def` in a class.
    AsyncMethod,
    /// A `class`.
    Class,
    /// The name of an `except ... as`.
    Exception,
    /// A name a pattern captures.
    Pattern,
    /// A name of a `from ... import`.
    FromImport,
    /// A name of an `import`.
    Import,
    /// A name that is assigned.
    Variable,
    /// The name of a `type` statement.
    TypeAlias,
    /// A parameter before `/`.
    PositionalOnly,
    /// A parameter given by position or by keyword.
    PositionalOrKeyword,
    /// A parameter after `*`.
    KeywordOnly,
    /// A `*args` parameter.
    VarPositional,
    /// A `**kwargs` parameter.
    VarKeyword,
    /// A type variable.
    TypeVar,
    /// A type variable tuple.
    TypeVarTuple,
    /// A parameter specification.
    ParamSpec,
}

/// One definition: a node that binds a name in a namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NameDefinition {
    /// The node that defines.
    pub at: Position,
    /// The name; the unknown name for a wildcard import.
    pub name: NameCode,
    /// The scope whose namespace holds the name.
    pub owner: Position,
    /// What kind of thing is defined.
    pub kind: DeclarationKind,
    /// The same, more finely.
    pub word: DefinitionWord,
    /// The bytes that name it: an identifier, a name, or the first name of `import a.b`.
    pub named: Span,
}

/// How a parameter takes its argument, and the callable it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlacedParameter {
    /// How it takes its argument.
    pub word: DefinitionWord,
    /// The callable.
    pub callable: Position,
}

/// The definitions of one (namespace, name), as a range of the sorted definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefinitionRange {
    /// The first.
    pub start: Count,
    /// One past the last.
    pub end: Count,
}

/// Whether routing the `nonlocal` names moved one off the nearest function around its scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Routing {
    /// None moved: the definitions stand.
    Kept,
    /// One moved: the definitions are to be built again.
    Moved,
}

/// The definitions part of the model, cleared between files.
#[derive(Clone, Debug, Default)]
pub(crate) struct DefinitionModel {
    /// Every definition, in source order.
    pub all: Vec<NameDefinition>,
    /// The definition of each node; the code of no definition for a node that defines nothing.
    pub of_node: Vec<DefinitionCode>,
    /// The definitions by owner, then name, then source order.
    pub sorted: Vec<DefinitionCode>,
    /// Where the definitions of each (namespace, name) are in `sorted`.
    pub index: HashMap<NamespaceKey, DefinitionRange, BuildHasherDefault<KeyHasher>>,
}
