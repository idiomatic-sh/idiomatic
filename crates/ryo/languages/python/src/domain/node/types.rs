//! The types of the `node` concept: the parser's own tree of one file, read in place. A syntax node has the id the walk gave it as it
//! opened (pre-order, from zero), which the walk also keeps in the parser's own node index, so a typed child answers its id; the
//! typed node of each id is kept beside the engine's node table.

use crate::domain::labels::types::Field;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{FileId, NameText, NodeTable, Position, SyntaxCompleteness};

/// Whether the parser read the whole file, and whether its lexer did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Wholeness {
    /// Whether every statement was parsed: incomplete once the parser or the language refuses a part of the file.
    pub syntax: SyntaxCompleteness,
    /// Whether every token was read: a refusal over a complete token stream leaves the tokens whole.
    pub lexical: SyntaxCompleteness,
}

/// What the syntax adapter lends once it has walked a file into the node table, while its parse lives.
#[derive(Debug)]
pub(crate) struct WalkedTree<'t> {
    /// The typed node of each syntax node, by its id.
    pub nodes: Vec<AnyNodeRef<'t>>,
    /// The text the parser read: the file's own bytes when they are UTF-8.
    pub text: NameText<'t>,
    /// Whether the file was read whole.
    pub completeness: Wholeness,
}

/// One walked file as a filler reads it: the engine's node table, the typed node of each syntax node and the text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SyntaxFile<'f> {
    /// The node table the walk wrote.
    pub table: &'f NodeTable,
    /// The typed node of each syntax node, by its id.
    pub nodes: &'f [AnyNodeRef<'f>],
    /// The text the parser read.
    pub text: NameText<'f>,
    /// The number the run gave the file.
    pub file: FileId,
    /// Whether the file was read whole.
    pub completeness: Wholeness,
}

/// A syntax node of a walked file: its id and its typed node.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Numbered<'f> {
    /// The id the walk gave it.
    pub id: Position,
    /// The parser's node.
    pub node: AnyNodeRef<'f>,
}

/// A child of a node and the field it fills, as the typed node says it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChildField<'t> {
    /// The child.
    pub child: AnyNodeRef<'t>,
    /// The field it fills.
    pub field: Field,
}

/// A node where it stands: what the fields of its children are read from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed<'t> {
    /// The node.
    pub node: AnyNodeRef<'t>,
    /// The field the node itself fills.
    pub field: Field,
    /// Its parent; none for the root.
    pub parent: Option<AnyNodeRef<'t>>,
}

/// The fixed words of the language a file's fillers have written so far, each with the word the file interned it as: a fixed
/// word is interned once for a file and written from here after that.
#[derive(Clone, Debug, Default)]
pub(crate) struct FixedWords {
    /// The texts, in the order they were first written.
    pub texts: Vec<NameText<'static>>,
    /// The word of each text.
    pub words: Vec<ryo::languages::Symbol>,
}
