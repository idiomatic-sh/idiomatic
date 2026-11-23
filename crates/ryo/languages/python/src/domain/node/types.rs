//! The types of the `node` concept: the parser's own tree of one file, read in place. A syntax node has the id the walk gave it as it
//! opened (pre-order, from zero), which the walk also keeps in the parser's own node index, so a typed child answers its id; the
//! typed node of each id is kept beside the engine's node table.

use crate::domain::labels::types::Field;
use crate::domain::model::lexical::types::{EmbeddedSite, TypedTree};
use crate::domain::model::types::BindingAnnotationTiming;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{FileId, NameText, NodeTable, NodeTableBuilder, Position, SyntaxCompleteness};

/// Whether the parser read the whole file, and whether its lexer did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Wholeness {
    /// Whether every statement was parsed: incomplete once the parser or the language refuses a part of the file.
    pub(crate) syntax: SyntaxCompleteness,
    /// Whether every token was read: a refusal over a complete token stream leaves the tokens whole.
    pub(crate) lexical: SyntaxCompleteness,
}

/// Import-statement count and physical-line comment presence accumulated in source order.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ImportPresence {
    pub(crate) statements: usize,
    pub(crate) line_comment: bool,
}

/// What the syntax adapter lends once it has walked a file into the node table, while its parse lives.
#[derive(Debug)]
pub(crate) struct WalkedTree<'t> {
    /// The typed node of each syntax node, by its id.
    pub(crate) nodes: Vec<AnyNodeRef<'t>>,
    /// The parent of each syntax node, by its id; the root is its own.
    pub(crate) parents: Vec<Position>,
    /// Where the subtree of each syntax node ends, by its id: the id after its last descendant.
    pub(crate) ends: Vec<Position>,
    /// Whether the module postpones the evaluation of its annotations.
    pub(crate) annotations: BindingAnnotationTiming,
    /// What a declaration without a value does to its name: binds it in a stub.
    pub(crate) declarations: crate::domain::model::types::AnnotatedDeclarations,
    /// The string literals the walk met since it last asked which of them are type expressions.
    pub(crate) strings: Vec<StringSite>,
    /// Import presence for the file's own physical source.
    pub(crate) import_presence: ImportPresence,
    /// The imported names the walk met: the places of the aliases of the file's import statements.
    pub(crate) imports: Vec<Position>,
    /// From-imports with at least one leading dot, in source order.
    pub(crate) relative_froms: Vec<Position>,
    /// The embedded segments written so far, in the order they were written.
    pub(crate) segments: Vec<EmbeddedSite>,
    /// The text the parser read: the file's own bytes when they are UTF-8.
    pub(crate) text: NameText<'t>,
    /// Whether the file was read whole.
    pub(crate) completeness: Wholeness,
    /// The names the walk interned: of each node, and of each name token of the file's own tree.
    pub(crate) names: crate::domain::model::names::types::WalkNames,
}

/// One walked file as a filler reads it: the engine's node table, the typed node of each syntax node and the text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SyntaxFile<'f> {
    /// The node table the walk wrote.
    pub(crate) table: &'f NodeTable,
    /// The typed node of each syntax node of the file's own tree, by its id.
    pub(crate) nodes: &'f [AnyNodeRef<'f>],
    /// Every typed node by its place, with its parent and the end of its subtree: the file's own tree, then the nodes of each
    /// embedded segment.
    pub(crate) tree: TypedTree<'f, 'f>,
    /// The text the parser read.
    pub(crate) text: NameText<'f>,
    /// The number the run gave the file.
    pub(crate) file: FileId,
    /// Whether the file was read whole.
    pub(crate) completeness: Wholeness,
}

/// A syntax node of a walked file: its id and its typed node.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Numbered<'f> {
    /// The id the walk gave it.
    pub(crate) id: Position,
    /// The parser's node.
    pub(crate) node: AnyNodeRef<'f>,
}

/// A child of a node and the field it fills, as the typed node says it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChildField<'t> {
    /// The child.
    pub(crate) child: AnyNodeRef<'t>,
    /// The field it fills.
    pub(crate) field: Field,
}

/// A node where it stands: what the fields of its children are read from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed<'t> {
    /// The node.
    pub(crate) node: AnyNodeRef<'t>,
    /// The field the node itself fills.
    pub(crate) field: Field,
    /// Its parent; none for the root.
    pub(crate) parent: Option<AnyNodeRef<'t>>,
}

/// Where a string literal stands, as its syntax alone says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnnotationSeed {
    /// In an annotation: of a parameter, of a return, of an annotated assignment; or in a string already read as a type.
    Annotation,
    /// Anywhere else: a type expression only where a typing form above it says so.
    Candidate,
}

/// A string literal the walk met, with where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StringSite {
    /// The place of the string literal.
    pub(crate) at: Position,
    /// Where it stands.
    pub(crate) seed: AnnotationSeed,
}

/// What a walk hands its caller, and what it asks of it.
#[derive(Debug)]
pub(crate) enum Walked<'b, 'w, 't> {
    /// The tree so far, with the strings the walk met since it last asked: the caller answers the places of those the language
    /// reads as type expressions, and the walk embeds the expression of each that parses as one.
    Strings(&'b WalkedTree<'t>),
    /// The whole tree, whose table the builder holds, lent while the parses live: the caller reads it and answers nothing.
    Whole(&'b mut NodeTableBuilder<'w>, WalkedTree<'t>),
}
