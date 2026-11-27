//! The types of the model's `lexical` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::model::names::types::{KeyHasher, NameCode};
use crate::domain::model::types::BindingAnnotationTiming;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{Count, EmbeddedMapping, Position};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// The file as the model's passes read it: the parser's own node of each place, its parent and where its subtree ends. A place
/// is the node's id for a node of the file's own tree; the walk hands the three columns over before its table is finished.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TypedTree<'t, 'a> {
    /// The embedded segments among the places, in the order of their first place: a place before the first of them is a node
    /// of the file's own tree, and its id.
    pub(crate) segments: &'t [EmbeddedSite],
    /// The parent of each node, by its place; the root is its own.
    pub(crate) parents: &'t [Position],
    /// Where the subtree of each node ends, by its place: the place after its last descendant.
    pub(crate) ends: &'t [Position],
    /// The parser's node of each place.
    pub(crate) nodes: &'t [AnyNodeRef<'a>],
    /// Whether the module postpones the evaluation of its annotations (`from __future__ import annotations`).
    pub(crate) annotations: BindingAnnotationTiming,
    /// Whether a bare annotated target declares a name in this source.
    pub(crate) declarations: crate::domain::model::types::AnnotatedDeclarations,
    /// The names the walk interned: the name of each place, and the table.
    pub(crate) named: &'t crate::domain::model::names::types::WalkNames,
}

/// An embedded segment the walk wrote: the expression of a string the language reads as a type expression. The nodes of a
/// segment have consecutive places after the file's own nodes, and consecutive ids in the table from its root's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EmbeddedSite {
    /// The place of the string literal that holds the expression.
    pub(crate) host: Position,
    /// The place of the segment's root: its first node.
    pub(crate) first: Position,
    /// The id the table gave the segment's root.
    pub(crate) root: Position,
    /// How the segment's bytes map to the file's.
    pub(crate) mapping: EmbeddedMapping,
}

/// What a node is to the language's name rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NodeClass {
    /// The module, or a lone expression read as one.
    Module,
    /// A `def`.
    FunctionDef,
    /// A `class`.
    ClassDef,
    /// A `lambda`.
    Lambda,
    /// A list comprehension.
    ListComp,
    /// A set comprehension.
    SetComp,
    /// A dictionary comprehension.
    DictComp,
    /// A generator expression.
    Generator,
    /// One `for ... in ... if ...` clause of a comprehension.
    Comprehension,
    /// A name expression.
    Name,
    /// An attribute expression.
    Attribute,
    /// An identifier that is not an expression: a definition's name, an attribute's, a keyword's.
    Identifier,
    /// One imported name.
    Alias,
    /// A parameter.
    Parameter,
    /// The parameters of a callable.
    Parameters,
    /// A parameter with its default.
    ParameterWithDefault,
    /// An `except` clause.
    ExceptHandler,
    /// An `as` pattern or a capture.
    MatchAs,
    /// A starred pattern.
    MatchStar,
    /// A mapping pattern.
    MatchMapping,
    /// A type variable.
    TypeVar,
    /// A type variable tuple.
    TypeVarTuple,
    /// A parameter specification.
    ParamSpec,
    /// The type parameters of a definition.
    TypeParams,
    /// A `global` statement.
    Global,
    /// A `nonlocal` statement.
    Nonlocal,
    /// A named expression (`:=`).
    NamedExpr,
    /// A `type` statement.
    TypeAlias,
    /// An annotated assignment.
    AnnAssign,
    /// An assignment.
    Assign,
    /// A `from ... import`.
    ImportFrom,
    /// An `import`.
    Import,
    /// Every other statement.
    Statement,
    /// Every other expression.
    Expression,
    /// Everything else.
    #[default]
    Other,
}

/// How a node opens a namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeForm {
    /// The module.
    Module,
    /// A function: its body runs when it is called.
    Function,
    /// A class: its body runs where it stands, and nested functions do not see its names.
    Class,
    /// A lambda: its body runs when it is called.
    Lambda,
    /// A list, set or dictionary comprehension: its body runs where it stands.
    Comprehension,
    /// A generator expression: its body runs when it is iterated.
    Generator,
}

/// Whether a node is a statement, an expression or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntaxForm {
    /// A statement.
    Statement,
    /// An expression.
    Expression,
    /// Neither.
    Neither,
}

/// What a name or attribute expression does with its target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NameUse {
    /// It reads it.
    #[default]
    Load,
    /// It binds it.
    Store,
    /// It deletes it.
    Delete,
}

/// Whether an expression is an annotation: of a parameter, of an annotated assignment, or a function's return annotation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AnnotationMark {
    /// It is not.
    #[default]
    Plain,
    /// It is, and Python evaluates it (or, when the file defers annotations, stores it to be evaluated later).
    Annotation,
    /// It is the annotation of a variable of a function, which Python never evaluates (PEP 526: "annotations for local
    /// variables will not be evaluated").
    Unevaluated,
}

/// Where a scope says a name it assigns is bound instead of in itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeRedirect {
    /// `global`: in the module.
    Global,
    /// `nonlocal`: in the nearest enclosing function that has the name (`LexicalModel::targets`).
    Nonlocal,
}

/// A name in a namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NamespaceKey {
    /// The scope whose namespace it is.
    pub(crate) scope: Position,
    /// The name.
    pub name: NameCode,
}

/// Something that leaves the origin of a name unproven after it: a wildcard import, or a deletion of the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NameHazard {
    /// The namespace it changes.
    pub(crate) scope: Position,
    /// The import's alias, or the deleted name.
    pub(crate) at: Position,
    /// The deleted name; the code of no name for a wildcard import.
    pub name: NameCode,
}

/// How many dots a `from` import starts with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ImportLevel(pub u32);

/// Whether an import gives its name another one with `as`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImportRename {
    /// It keeps the imported name.
    Kept,
    /// It renames it.
    Renamed,
}

/// Where one imported name comes from, as its statement writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImportOrigin {
    /// The alias.
    pub(crate) at: Position,
    /// The dots of a `from` import; none for an `import`.
    pub(crate) level: Option<ImportLevel>,
    /// The module of a `from` import, when it names one; the code of no name otherwise.
    pub(crate) module: NameCode,
    /// The imported name, dotted as written.
    pub name: NameCode,
    /// The first name of the imported name: what an `import a.b` binds.
    pub(crate) head: NameCode,
    /// Whether the alias renames it.
    pub(crate) rename: ImportRename,
}

/// The lexical part of the model: one entry per node unless said otherwise, cleared between files.
#[derive(Clone, Debug, Default)]
pub(crate) struct LexicalModel {
    /// How many nodes the file has.
    pub(crate) nodes: Count,
    /// What each node is.
    pub(crate) class: Vec<NodeClass>,
    /// The scope each node is evaluated in.
    pub(crate) scope_of: Vec<Position>,
    /// The name a name expression or an identifier carries, and the attribute an attribute expression reads; the code of no
    /// name for every other node.
    pub name: Vec<NameCode>,
    /// What a name or attribute expression does with its target.
    pub(crate) usage: Vec<NameUse>,
    /// Whether the node is an annotation.
    pub(crate) annotation: Vec<AnnotationMark>,
    /// The `global` and `nonlocal` names of each scope: the first declaration of a name in a scope is the one that holds.
    pub(crate) declared: HashMap<NamespaceKey, ScopeRedirect, BuildHasherDefault<KeyHasher>>,
    /// The function each `nonlocal` name of a scope is bound in, when it is not the nearest function around the scope: the
    /// nearest enclosing function that binds the name (Language Reference 7.13), a function that declares the name
    /// `nonlocal` itself passed through.
    pub(crate) targets: HashMap<NamespaceKey, Position, BuildHasherDefault<KeyHasher>>,
    /// The nodes that may define a name, in source order.
    pub(crate) binders: Vec<Position>,
    /// The deleted name expressions, in source order.
    pub(crate) deleted: Vec<Position>,
    /// The wildcard imports and deletions, in source order.
    pub(crate) hazards: Vec<NameHazard>,
    /// Where each imported name comes from, in source order.
    pub(crate) imports: Vec<ImportOrigin>,
}
