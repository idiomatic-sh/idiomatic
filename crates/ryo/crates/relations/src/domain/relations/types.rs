// the catalog's vocabulary of cells, kinds and names is one list a relation is described by
//! The types of the `relations` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl. Every name, id, span and cell the engine handles is one of
//! these; no text is held inside the engine.

use crate::domain::terms::types::Symbol;

/// A whole number a relation holds as a cell, positive or negative.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Integer(pub i64);

impl std::fmt::Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// How many elements are below an element of the table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Extent(pub u32);

/// A byte offset in a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(pub u32);

/// The place of an element in its file's element table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position(pub u32);

/// The number the element table holds for a kind of node or token, in the language's own numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KindBits(pub u16);

/// The bit flags of an element; [`TOKEN`](crate::domain::relations::constants::TOKEN) marks a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Flags(pub u16);

/// The number a value is stored as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bits(pub u64);

/// One half of a word: the high or the low thirty-two bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Half(pub u32);

/// How many of something there are: elements, names, kinds, relations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Count(pub usize);

/// A buffer of bytes: the notes a language writes of a file, the text a worker makes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);

/// A buffer of words: values written as numbers so they can leave a file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words(pub Vec<u64>);

/// A number a reader noted about a name while reading a file; nothing noted is zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NameNote(pub u32);

/// The hash of a name's text, which finds its symbol in an interner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextHash(pub u64);

/// The worker that made a piece of derived text, numbered from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerNumber(pub u32);

/// The "worker" of the steps that run after the files were read: the text they make is in the names they made.
pub const CROSS_FILE: WorkerNumber = WorkerNumber(u32::MAX);

/// Where the text of a name is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// In the bytes of the file it was read from.
    Source,
    /// In the text the worker made while evaluating, which the worker keeps for the whole run.
    Derived(WorkerNumber),
}

/// The number of a place in the log a table of names keeps; a name the steps after the files made has the symbol that holds it, with
/// [`CROSS_PLACE`] set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaceNumber(pub u32);

/// Set in the place of a name that the steps run after the files were read made: its text is the symbol that remains of the number.
pub const CROSS_PLACE: u32 = 1 << 31;

/// Where the text of a name that left its file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamePlace {
    /// Where the text is.
    pub origin: Origin,
    /// The file whose bytes hold the text, for a name read from a file.
    pub file: FileId,
    /// Where the text starts.
    pub start: Offset,
    /// Where the text ends.
    pub end: Offset,
}

/// A name that left its file: the hash of its text, and the number of the place that says where the text is. Across files the
/// spike compares names by the hash alone, and so do the cells here, after a check that no two spellings share a hash (`check_spellings`); within a file a name is its symbol, which interning makes
/// exact (hash, then the text on a hit). Packed to twelve bytes, so a value takes sixteen.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed(4))]
pub struct Name {
    /// The hash of the text.
    pub hash: TextHash,
    /// Where the text is, among the places of the table the name is in.
    pub place: PlaceNumber,
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        let (left, right) = (self.hash, other.hash);
        left == right
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (left, right) = (self.hash, other.hash);
        left.cmp(&right)
    }
}

impl std::hash::Hash for Name {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        let hash = self.hash;
        hash.hash(state);
    }
}

/// A file of the repository, numbered in the order the run met it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// A node of a file: the file in the high half, where the node starts in the low half.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// A stretch of a file, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// Where it starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
}

/// One cell of a row, typed by what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// A node.
    Node(NodeId),
    /// A name, in the file being read.
    Symbol(Symbol),
    /// A name that left its file.
    Name(Name),
    /// A file.
    File(FileId),
    /// A stretch of a file.
    Span(Span),
    /// A whole number.
    Number(Integer),
    /// A yes or no.
    Flag(Flag),
}

// A cell is sixteen bytes, as the prototype's: the tables of the rules are twice as dense as with a name that carries its place.
const _: () = assert!(size_of::<Value>() == 16, "a cell is sixteen bytes");

/// What a relation is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// The shape of the tree itself, which every language has.
    Structure,
    /// Literal payloads such as string and numeric values.
    Literal,
    /// Facts read directly from a complete lexical token stream, independent of AST recovery.
    Lexical,
    /// Code: declarations, scopes, references and what they say of one another.
    Code,
    /// Keyed documents: mappings, entries and sequences.
    Keyed,
    /// Documents of prose: the text a reader reads.
    Document,
    /// What a project's manifests say.
    Manifest,
    /// What the engine gives for every file.
    Engine,
}

/// What a column holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum CellKind {
    /// A node.
    Node,
    /// A name.
    Symbol,
    /// A file.
    File,
    /// A stretch of a file.
    Span,
    /// A whole number.
    Number,
    /// A yes or no.
    Flag,
}

/// What the catalog's three types are made of: the relations, their columns and the writer of their rows,
/// all generated from the list in `each_relation`.
macro_rules! catalog_types {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        /// The relations ryo owns, one variant each.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub enum Relation {
            $( $(
                #[doc = stringify!($module)]
                $variant,
            )* )*
        }

        /// A column of one of the relations, one variant per relation holding that relation's columns.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Column {
            $( $(
                #[doc = stringify!($module)]
                $variant($crate::domain::relations::collections::relations::$module::item::$column),
            )* )*
        }

        /// Where the engine writes the rows of every relation: one `Rows` for each row type of the catalog. A
        /// language never holds one: it is given the `Rows` of the one relation it reads.
        pub trait RowWriter:
            $( $( Rows<$crate::domain::relations::collections::relations::$module::item::$row> + )* )*
        {
        }

        impl<Writer> RowWriter for Writer where
            Writer: $( $( Rows<$crate::domain::relations::collections::relations::$module::item::$row> + )* )*
        {
        }
    };
}

crate::each_relation!(catalog_types);

/// Where the rows of one relation are written, a typed row at a time. A reader of the relation is given
/// this and nothing more: it cannot write a row of another relation.
pub trait Rows<Row> {
    /// Writes one row.
    fn write(&mut self, row: Row);

    /// Says the relation is only partly read in this file, for this cause: the file has a construct the reader cannot
    /// represent as rows, and `missing` says which rows. The engine does not trust an absence of rows of a partly read relation.
    fn partial(&mut self, cause: ReportText<'_>, missing: Missing);

    /// The same, for the rows that are about `file`, a file of the run (or one numbered after the files it lists): a reader of the
    /// project says in which file the part it could not read is. A writer that does not keep the file says it of the whole project.
    fn partial_at(&mut self, file: FileId, cause: ReportText<'_>, missing: Missing);
}

/// Which rows of a relation are missing from a file that is read in part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Missing {
    /// Any row could be.
    Any,
    /// Only rows whose `kind` is this one could be: a question about the rows of another kind is not affected.
    Kind(RowKind),
}

/// The kind a row of a relation says it is, in the relation's own column of kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowKind(pub &'static str);

/// Where a relation's rows come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// The trees of the files, read by a language.
    Trees,
    /// The manifests of the project and the links between its files, read by a language.
    Manifests,
    /// The engine, for every file: no language reads them.
    Engine,
}

/// A yes or no a relation holds as a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Flag {
    /// No.
    #[default]
    Lowered,
    /// Yes.
    Raised,
}

/// Whether a dependency is needed only when asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Optionality {
    /// It is needed.
    #[default]
    Required,
    /// It is needed only when asked for.
    Optional,
}

/// Whether a relation says something of what a file declares, which an empty file has nothing of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationDeclaring {
    /// It does: code, and the text written in a file.
    Declares,
    /// It does not.
    Silent,
}

/// Whether a column says which row a row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnKeyness {
    /// No two rows of the relation hold the same value in it.
    Key,
    /// Rows may share a value in it.
    Plain,
}

/// What a parser or a reader says about a file, borrowed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReportText<'a>(pub &'a str);

/// One column of the rows a worker's file wrote to a relation: the numbers of the cells, in row order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactColumn(pub Vec<u64>);

/// The rows of one relation that a worker's file wrote, read where the worker keeps them: the columns are the worker's own,
/// borrowed, and nothing is copied.
#[derive(Clone, Copy, Debug)]
pub struct ColumnsView<'a> {
    /// What each column holds, in column order.
    pub kinds: &'static [CellKind],
    /// The numbers of each column.
    pub columns: &'a [FactColumn],
}

/// The role of an element under its parent, in the language's own closed vocabulary of roles: a number, the same in every file. The
/// vocabulary lists the role of an edge that has none first, so `NO_FIELD` is the number `0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldId(pub u16);

/// A scalar property supplied by a parser, separate from a child-edge role.
/// Zero is reserved for the element's decoded value; other keys belong to the language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyId(pub u16);

/// Whether the parser assigned a real source location.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locatedness {
    /// A real source interval.
    Located,
    /// The parser supplied no source interval.
    Absent,
}
