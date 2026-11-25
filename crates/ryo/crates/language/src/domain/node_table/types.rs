//! The types of the `node_table` concept: one file's node table in the engine's one id space. The syntax nodes come first, in
//! pre-order (ids `0..nodes`), then the tokens in source order (ids `nodes..`), each token with its parent node and its place
//! among that node's children (nodes and tokens merged in source order). Embedded subtrees (a string read as an expression, a
//! macro argument read as code) form a third segment after the tokens, each in the same nodes-then-tokens order, its root's
//! parent the host node.

use crate::domain::diagnostics::types::Diagnostics;
use crate::domain::elements::types::{DecodedProperty, OpenNode, SyntaxAcceptance};
use ryo_relations::domain::elements::types::EmbeddedMapping;
use ryo_relations::domain::relations::types::{Bytes, Count, KindBits, Offset, Position, Span};
use ryo_relations::domain::terms::types::{ChildOrdinal, NameText};

/// The node table of one file: columns cleared between files, never reallocated. Written only through `NodeTableBuilder`,
/// read through the readers of `node_table::service`.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::node_table::types::NodeTable(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, finished, node_count};
/// use ryo_language::domain::node_table::types::NodeTable;
/// use ryo_relations::domain::relations::types::Count;
///
/// let mut table = NodeTable::default();
/// assert_eq!(node_count(finished(&mut builder(&mut table, Count(1)))), Count(0));
/// ```
#[derive(Debug, Default)]
pub struct NodeTable(pub(crate) NodeTableEngine);

/// The private columns of a `NodeTable`, one entry per id unless said otherwise.
#[derive(Debug, Default)]
pub(crate) struct NodeTableEngine {
    /// End of the concrete view, for providers whose semantic population includes appended trees.
    pub(crate) visible: Option<usize>,
    /// Kind code of each node and token, in the language's own numbering: the column an engine reads as it is.
    pub(crate) kind: Vec<u16>,
    /// Parent id of each node and token (a root is its own parent; an embedded root's parent is its host).
    pub(crate) parent: Vec<u32>,
    /// For a node, the id after its last descendant node; for a token, the id after it.
    pub(crate) subtree_end: Vec<Position>,
    /// First byte of each element, in the file's bytes.
    pub(crate) start: Vec<u32>,
    /// The byte after each element's last.
    pub(crate) end: Vec<u32>,
    /// The field code each element fills in its parent.
    pub(crate) field: Vec<u16>,
    /// The place of each element among its parent's children.
    pub(crate) ordinal: Vec<ChildOrdinal>,
    /// Where each element's children are in `children`: written as the element closes (a token has none).
    pub(crate) child_range: Vec<ChildRange>,
    /// The children of every node, nodes and tokens merged in source order, one block a node in the order the nodes close.
    pub(crate) children: Vec<Position>,
    /// Ids of each kind, in id order, one list a kind code: each id is added as it is numbered. Lists are cleared between
    /// files, never dropped.
    pub(crate) by_kind: Vec<Vec<Position>>,
    /// Ids of each field, in id order, one list a field code (zero: no field), added as each id is numbered.
    pub(crate) by_field: Vec<Vec<Position>>,
    /// The kinds present, read off the kind lists when the table is finished.
    pub(crate) present: KindsPresent,
    /// How many syntax nodes the file's own tree has: its tokens start here.
    pub(crate) nodes: Count,
    /// The decoded text of the tokens that have one, and the logical text of embedded tokens, in id order.
    pub(crate) decoded: Vec<DecodedProperty>,
    /// The scalar properties the parser recorded on nodes, in (id, property) order.
    pub(crate) properties: Vec<DecodedProperty>,
    /// The bytes of every decoded text and property, one after another.
    pub(crate) texts: Bytes,
    /// What the parser reported.
    pub(crate) diagnostics: Diagnostics,
    /// Whether the parser accepted the file.
    pub(crate) acceptance: SyntaxAcceptance,
    /// The embedded segments, in the order they were written.
    pub(crate) embedded: Vec<EmbeddedSegment>,
    /// The nodes the parser implied, in id order.
    pub(crate) implied: Vec<Position>,
    /// Per element, its neighbours' sets (written as each id is numbered, only when the builder was made `structured`).
    pub(crate) sets: Vec<ElementSets>,
    /// The kind sets a `structured` builder was given, one enclosing column each.
    pub(crate) enclosing_sets: Vec<KindSet>,
    /// Per kind set, per element: the nearest strict ancestor whose kind is in the set, `u32::MAX` for none.
    pub(crate) enclosing: Vec<Vec<u32>>,
}

/// A set of kind codes below 256, one bit a code: the kinds an enclosing column looks for.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::KindSet;
///
/// assert_eq!(KindSet([0b10, 0, 0, 0]).0[0] & 0b10, 0b10);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KindSet(pub [u64; 4]);

/// What an engine's checks read of an element's neighbours, in one cache line: the kinds of its children (codes below 256),
/// the fields its children fill (codes below 128), its parent's kind (its own for a root) and whether it is a root (its own
/// parent). An embedded root is no root: its parent is its host, whose sets hold it.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::ElementSets;
///
/// assert!(!ElementSets::default().root);
/// assert_eq!(std::mem::size_of::<ElementSets>(), 64);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C, align(64))]
pub struct ElementSets {
    /// One bit for each kind among the element's children.
    pub child_kinds: [u64; 4],
    /// One bit for each field its children fill.
    pub child_fields: [u64; 2],
    /// The parent's kind.
    pub parent_kind: u16,
    /// Whether the element is its own parent.
    pub root: bool,
}

/// Where one element's children are in the table's `children`: `from..to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChildRange {
    /// The first child's place.
    pub(crate) from: Position,
    /// The place after the last child.
    pub(crate) to: Position,
}

impl Default for ChildRange {
    /// No children.
    fn default() -> Self {
        Self {
            from: Position(0),
            to: Position(0),
        }
    }
}

/// One embedded segment of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EmbeddedSegment {
    /// The node of the file whose text was read as the segment.
    pub(crate) host: Position,
    /// The segment's root.
    pub(crate) root: Position,
    /// The id after the segment's last element.
    pub(crate) end: Position,
    /// Where the segment's tokens start: its nodes are numbered from `root`, its tokens after them.
    pub(crate) tokens: Position,
    /// How the segment's logical offsets map to the file's bytes.
    pub(crate) mapping: EmbeddedMapping,
}

/// A byte range of the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::ByteSpan;
/// use ryo_relations::domain::relations::types::Offset;
///
/// assert!(ByteSpan { start: Offset(1), end: Offset(4) }.end > Offset(1));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ByteSpan {
    /// The first byte.
    pub start: Offset,
    /// The byte after the last.
    pub end: Offset,
}

/// Whether the parser implied a node: such a node is in no byte of the file, so nothing is placed by it.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::{builder, enter_implied_node, finished, implied_of, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, Implied, NodeTable};
/// use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
///
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_implied_node(&mut writer, KindBits(1), FieldCode(0));
/// leave_node(&mut writer, Offset(0));
/// assert_eq!(implied_of(finished(&mut writer), Position(0)), Implied::Yes);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Implied {
    /// The file wrote the node or token: its span is where it is.
    No,
    /// The parser implied the node: it stands zero wide at the furthest byte the walk had written.
    Yes,
}

/// A field code in the language's own numbering; zero is "fills no field".
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::FieldCode;
///
/// assert_eq!(FieldCode::default(), FieldCode(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldCode(pub u16);

/// The kinds that occur in a file, one bit per kind code, `KIND_COUNT.div_ceil(64)` words.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::{KindsPresent, PresenceWord};
///
/// assert_eq!(KindsPresent(vec![PresenceWord(1)]).0.len(), 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KindsPresent(pub Vec<PresenceWord>);

/// Sixty-four kinds' presence bits.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::PresenceWord;
///
/// assert_eq!(PresenceWord(0b10).0 & 0b10, 0b10);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresenceWord(pub u64);

/// The first id of an embedded segment: its root, a node whose parent is the host.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::EmbeddedRoot;
/// use ryo_relations::domain::relations::types::Position;
///
/// assert_eq!(EmbeddedRoot(Position(3)).0, Position(3));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbeddedRoot(pub Position);

/// The text a language reads as an embedded tree, and how its logical offsets map to the file's bytes.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::EmbeddedText;
/// use ryo_relations::domain::elements::types::EmbeddedMapping;
/// use ryo_relations::domain::relations::types::Offset;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let text = EmbeddedText { text: NameText("x + 1"), mapping: EmbeddedMapping::Exact(Offset(2)) };
/// assert_eq!(text.text, NameText("x + 1"));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedText<'a> {
    /// The logical text the language's library reads.
    pub text: NameText<'a>,
    /// Where its offsets are in the file.
    pub mapping: EmbeddedMapping,
}

/// What a language's walk writes the table through: open a node, add a token, close a node. Made by `node_table::service::builder`
/// for the engine, finished by `node_table::service::finished`.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::node_table::types::NodeTableBuilder(todo!());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::service::builder;
/// use ryo_language::domain::node_table::types::NodeTable;
/// use ryo_relations::domain::relations::types::Count;
///
/// let mut table = NodeTable::default();
/// let _writer = builder(&mut table, Count(1));
/// ```
#[derive(Debug)]
pub struct NodeTableBuilder<'a>(pub(crate) BuilderState<'a>);

/// The private state of a `NodeTableBuilder`.
#[derive(Debug)]
pub(crate) struct BuilderState<'a> {
    /// The table written.
    pub(crate) table: &'a mut NodeTable,
    /// How many kinds the language has: the kinds present have a bit for each.
    pub(crate) kinds: Count,
    /// The open nodes, innermost last, each with the ordinal its next child takes.
    pub(crate) open: Vec<OpenNode>,
    /// The children of the open nodes written so far, each open node's after the place `opened` holds for it: a node by
    /// its id, a token by `PENDING` with its place among `pending`.
    pub(crate) written: Vec<Position>,
    /// For each open node, innermost last, where its children start in `written`.
    pub(crate) opened: Vec<Position>,
    /// The tokens of the segment being written, kept until its nodes are numbered.
    pub(crate) pending: Vec<PendingToken>,
    /// The id the segment's first token takes once its nodes are numbered: the file's tree starts at zero.
    pub(crate) segment_start: Position,
    /// The segment being written: the file's own tree, or an embedded text.
    pub(crate) segment: BuilderSegment,
    /// The furthest byte a call has reached: where an implied node is.
    pub(crate) cursor: Offset,
    /// Whether the table has been finished and may be read.
    pub(crate) finished: Finish,
    /// The open implied nodes, innermost last: their end stays their start.
    pub(crate) implied: Vec<Position>,
    /// The logical text of the embedded segment being written.
    pub(crate) logical: LogicalText,
    /// Whether each element's sets and enclosing ids are written as it is numbered.
    pub(crate) structured: bool,
}

/// A token written before the nodes of its segment are all numbered.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PendingToken {
    /// Its kind.
    pub(crate) kind: KindBits,
    /// Its field in its parent.
    pub(crate) field: FieldCode,
    /// Its parent node.
    pub(crate) parent: Position,
    /// Its place among the parent's children.
    pub(crate) ordinal: ChildOrdinal,
    /// Where it is in the file.
    pub(crate) span: ByteSpan,
    /// Where its decoded text is in the table's `texts`, when it has one.
    pub(crate) decoded: Option<Span>,
    /// Its place in the table's `children`, once its parent has closed.
    pub(crate) slot: Position,
}

/// Which segment the builder is writing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuilderSegment {
    /// The file's own tree.
    File,
    /// An embedded tree over a host, its offsets mapped so.
    Embedded(Position, EmbeddedMapping),
}

/// Whether a builder's table is complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Finish {
    /// Calls may still write.
    Open,
    /// The columns are complete and readable.
    Done,
}

/// Where the next element of an open node goes: its parent and its place among the parent's children.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChildPlace {
    /// The parent.
    pub(crate) parent: Position,
    /// The place among its children.
    pub(crate) ordinal: ChildOrdinal,
}

/// The logical text of the embedded segment a builder is writing.
#[derive(Clone, Debug, Default)]
pub(crate) struct LogicalText(pub(crate) String);

/// One embedded segment of a table, as `node_table::service::embedded_segments` reads it: the node it was read from, its root
/// and the id after its last element.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::node_table::types::EmbeddedSpan;
/// use ryo_relations::domain::relations::types::Position;
///
/// let segment = EmbeddedSpan { host: Position(0), root: Position(2), end: Position(4) };
/// assert!(segment.root < segment.end);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbeddedSpan {
    /// The node of the file's own tree the segment was read from: the parent of its root.
    pub host: Position,
    /// The segment's root: its first id.
    pub root: Position,
    /// The id after the segment's last element.
    pub end: Position,
}
