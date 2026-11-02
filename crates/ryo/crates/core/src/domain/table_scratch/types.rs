//! The types of the `table_scratch` concept: the invocation-owned rooms operators work in, plain
//! data with public fields and derives, and no constructor or inherent impl.

use crate::domain::table_cells::types::{ColumnPlaces, RowIndex};
use crate::domain::table_groups::types::Group;
use crate::domain::table_groups::types::Hashes;
use crate::domain::table_model::types::KeyIndex;
use crate::domain::table_model::types::Table;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_reach::types::Walk;
use ryo_language::domain::elements::types::KindSum;
use ryo_language::domain::relations::types::{FileId, KindBits, TextHash, Value};
use ryo_language::domain::terms::types::Symbol;

/// Text being made in the room the operators work in: cleared and grown in place for every row, so it is a `String` and not a `Box<str>`.
#[derive(Debug, Default)]
pub(crate) struct ScratchText(pub String);

impl std::ops::Deref for ScratchText {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ScratchText {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl std::fmt::Write for ScratchText {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

/// A word being read out of a text: cleared and grown in place for every word.
#[derive(Debug, Default)]
pub(crate) struct ScratchWord(pub String);

impl std::ops::Deref for ScratchWord {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ScratchWord {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl std::fmt::Write for ScratchWord {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

/// The room a search for the entities on cycles works in, kept from file to file.
#[derive(Debug, Default)]
pub(crate) struct CycleRoom {
    /// The number of each entity met.
    pub numbers: hashbrown::HashMap<Value, EntityNumber>,
    /// The entities, by number.
    pub values: Vec<Value>,
    /// The edges, by the numbers of the entities they join.
    pub pairs: Vec<EdgeEnds>,
    /// Where the edges leaving each entity begin in `leaving`, one more than there are entities.
    pub starts: EdgePlaces,
    /// Where the next edge of each entity is put while they are sorted by their source.
    pub filled: EdgePlaces,
    /// The entity each edge leads to, grouped by its source.
    pub leaving: EntityNumbers,
    /// When each entity was first met by the walk.
    pub order: MeetingOrder,
    /// The earliest meeting any entity below it on the walk can reach.
    pub low: MeetingOrder,
    /// Whether each entity is on the stack of the walk.
    pub open: OnStack,
    /// The entities of the components being found.
    pub stack: EntityNumbers,
    /// Whether each entity is on a cycle.
    pub found: OnCycle,
    /// The entities on the walk, each with the next edge to try.
    pub walk: Vec<WalkStep>,
    /// The entities of the component just found.
    pub component: EntityNumbers,
    /// The entities on cycles, in order.
    pub ordered: Vec<Value>,
}

/// Which kinds a test asked of their names is about: the vocabulary of kinds it asked of, the test, and the hash of the text it is
/// against.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KindTest {
    /// The names of the kinds of the file it was asked of: a set of kinds is only as good as the numbering of the language that made it.
    pub vocabulary: KindSum,
    /// How the name is compared, by its place among the tests that can name a kind.
    pub test: NamingTest,
    /// The exact literal text the name is compared with.
    pub text: Box<str>,
}

/// The kinds of a language that have one name: as a set to ask of, and as a list to walk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindSet {
    /// A bit for each kind, by its number.
    pub bits: KindMask,
    /// The kinds, in order.
    pub kinds: Vec<KindBits>,
}

/// The places of the elements of a tree that a filter leaves to be looked at.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Candidates(pub Vec<u32>);

/// Comparisons of a filter, by their place among the filter's comparisons.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ComparisonNumbers(pub Vec<usize>);

/// Which elements of a tree a domain admits, by their place.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Marks(pub Vec<bool>);

/// An element of a tree, by its place among the elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementIndex(pub usize);

/// A text of a file, by the number of its name; the elements with no text have the slot after the last name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextSlot(pub usize);

/// A test, by its place among the four that can name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NamingPlace(pub usize);

/// Whether the sets of kinds have decided the comparisons that name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindsBy {
    /// The sets decide them.
    Sets,
    /// Nothing decides them before the rows are read.
    Nothing,
}

/// What a row asks of a name or a file: the hash of the name's text, or the number of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GlobSubject {
    /// Whether it is a name or a file.
    pub tag: SubjectTag,
    /// The hash of the name's text, or the number of the file.
    pub hash: SubjectHash,
}

/// Whether the subject of a glob is a name or a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubjectTag {
    /// A name.
    Name,
    /// A file.
    File,
}

/// Where a comparison's decision about a name is remembered, and the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoPlace {
    /// The place in the memo.
    pub place: MemoSlot,
    /// The name.
    pub symbol: Symbol,
}

/// Stretches of a text or of a list: where each group begins and ends in a list, or where each captured text is in a text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stretches(pub Vec<std::ops::Range<usize>>);

/// What a comparison decided is remembered by the comparison and the hash of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct VerdictKey {
    /// The comparison.
    pub comparison: ComparisonSlot,
    /// The hash of the text it decided about.
    pub text: TextHash,
}

/// A comparison of a filter, by its place among the filter's comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComparisonSlot(pub usize);

/// The number of a kind set among the sets of a language's kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KindSetPlace(pub usize);

/// For each text of a file, whether the comparisons hold for it: zero not decided, one yes, two no.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct TextVerdicts(pub Vec<u8>);

/// Where the elements with each text of a file begin among the elements ordered by text.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct TextStarts(pub Vec<u32>);

/// Places of elements of a tree, in some order.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ElementPlaces(pub Vec<u32>);

/// The file and the number of elements the elements were grouped by text for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextedFile {
    /// The file.
    pub file: FileId,
    /// How many elements it has.
    pub elements: TreeSize,
}

/// How many elements a tree has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TreeSize(pub usize);

/// What a capture read from a path: where in the captured values what its slots matched begins, and how many ways it matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Captured {
    /// Where in the captured values it begins.
    pub begin: CapturedPlace,
    /// How many ways it matched.
    pub ways: WayCount,
}

/// A place among the captured values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CapturedPlace(pub usize);

/// How many ways a capture matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WayCount(pub usize);

/// An entity met by a search for cycles, by the order it was met in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EntityNumber(pub usize);

/// Entities, by number.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct EntityNumbers(pub Vec<usize>);

/// An edge of a search for cycles, by its place among the edges leaving an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EdgePlace(pub usize);

/// Places among the edges of a search for cycles.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct EdgePlaces(pub Vec<usize>);

/// The entities an edge joins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EdgeEnds {
    /// Where the edge starts.
    pub from: EntityNumber,
    /// Where it leads.
    pub to: EntityNumber,
}

/// An entity on the walk of a search for cycles, with the next edge to try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkStep {
    /// The entity.
    pub entity: EntityNumber,
    /// The next edge to try.
    pub edge: EdgePlace,
}

/// When each entity was met by the walk, or the earliest meeting reachable.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MeetingOrder(pub Vec<usize>);

/// Whether each entity is on the stack of the walk.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct OnStack(pub Vec<bool>);

/// Whether each entity is on a cycle.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct OnCycle(pub Vec<bool>);

/// A bit for each kind of a set, by the number of the kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindMask(pub Vec<u64>);

/// How a name is compared, by its place among the tests that can name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NamingTest(pub u8);

/// The hash of a name's text, or the number of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SubjectHash(pub u64);

/// A place in the memo of what a comparison decided about a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoSlot(pub usize);

/// For which file the elements were grouped by their text, when they were.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TextGrouping {
    /// No file has been grouped yet.
    #[default]
    Unmade,
    /// The elements of this file were grouped.
    Made(TextedFile),
}

/// A heavy operator group, constructed only when an operation first accesses it.
/// Its box and every payload are owned by the enclosing rule invocation.
#[derive(Debug, Default)]
pub(crate) struct LazyScratch<T>(pub std::cell::OnceCell<Box<T>>);

impl<T: Default> std::ops::Deref for LazyScratch<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.get_or_init(Box::default)
    }
}

impl<T: Default> std::ops::DerefMut for LazyScratch<T> {
    #[expect(
        clippy::expect_used,
        reason = "get_or_init installs the group before this exclusive mutable read; absence would violate OnceCell's contract"
    )]
    fn deref_mut(&mut self) -> &mut T {
        let _initialized = self.0.get_or_init(Box::default);
        self.0
            .get_mut()
            .expect("operator scratch group was initialized")
    }
}

/// Cheap row buffers and lazy heavy groups owned only by the current rule.
#[derive(Debug, Default)]
pub(crate) struct OperatorScratch {
    /// The cells of a row that a comparison reads, by their place in the row.
    pub needed: ColumnPlaces,

    /// A row read from the columns of a relation.
    pub row: Vec<Value>,

    /// A second row, for the operators that read two at once.
    pub other: Vec<Value>,
    /// Lazily constructed join work for this invocation.
    pub join_scratch: LazyScratch<JoinScratch>,
    /// Lazily constructed groups work for this invocation.
    pub groups_scratch: LazyScratch<GroupsScratch>,
    /// Lazily constructed walk work for this invocation.
    pub walk_scratch: LazyScratch<WalkScratch>,
    /// Lazily constructed capture work for this invocation.
    pub capture_scratch: LazyScratch<CaptureScratch>,
    /// Lazily constructed select work for this invocation.
    pub select_scratch: LazyScratch<SelectScratch>,
    /// Lazily constructed cycle work for this invocation.
    pub cycle_scratch: LazyScratch<CycleScratch>,
}

/// Mutable join work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct JoinScratch {
    /// The rows already kept, found by what they hold.
    pub seen: hashbrown::HashTable<RowIndex>,

    /// The cells of a row that say which rows it is alike.
    pub key: Vec<Value>,

    /// The rows of the table at `right`, in the order of their keys.
    pub order: KeyIndex,

    /// An input read whole, for the operators that look rows up in it.
    pub right: Table,

    /// A second input read whole, and the rows of it in the order of their keys.
    pub domain: Table,

    /// The rows of `domain` in the order of their keys.
    pub domain_order: KeyIndex,
}

/// Mutable groups work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct GroupsScratch {
    /// What a group has counted, group by group.
    pub counts: crate::domain::operating::types::GroupCounts,
    /// The values already counted for a group, by the group.
    pub pairs: hashbrown::HashSet<crate::domain::operating::types::GroupValue>,
    /// The hash of each value of a group, by the group.
    pub hashed: Vec<crate::domain::operating::types::GroupHash>,

    /// The groups a count found.
    pub groups: Vec<Group>,

    /// The hashes of the values of a group, to digest the distinct ones.
    pub hashes: Hashes,
}

/// Mutable walk work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct WalkScratch {
    /// The room a walk along edges works in.
    pub walk: Walk,

    /// Compact graph visitation/frontier state, reused without provider references.
    pub graph_walk: crate::domain::stored_graph::types::GraphWalk,

    /// Prepared adjacency ranges of the ordered DFS operator.
    pub dfs_graph: crate::domain::table_reach::types::OrderedGraph,

    /// Iterative ordered DFS frames and exact visited values.
    pub dfs_walk: crate::domain::table_reach::types::OrderedWalk,

    /// Which elements of a tree a domain admits, by their place.
    pub marks: Marks,

    /// For each element of a tree, its nearest ancestor a walk admits, once it is known.
    pub nearest: ElementPlaces,

    /// The elements whose nearest ancestor is being found, from the one asked about up.
    pub climbing: ElementPlaces,
}

/// Mutable capture work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct CaptureScratch {
    /// Where each group begins and ends in `members`, or where each captured text is in `text`.
    pub places: Stretches,

    /// Text being made: a path read for a capture, the name of a group.
    pub text: ScratchText,

    /// The word being read out of a text.
    pub word: ScratchWord,

    /// For each path a capture has read, where in `captured` what its slots matched begins, and how many ways it matched.
    pub captures: hashbrown::HashMap<Value, Captured>,

    /// What the slots matched, for every path of `captures`, a way after a way.
    pub captured: Vec<Value>,
}

/// Mutable select work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct SelectScratch {
    /// What a comparison decided about a text, by the comparison and the hash of the text: the same text
    /// is decided once for the run, not once for each file that has it.
    pub verdicts: hashbrown::HashMap<VerdictKey, Verdict>,

    /// Fixed glob operands made ready once for each comparison, for this run's bindings.
    pub compiled_globs:
        hashbrown::HashMap<ComparisonSlot, crate::domain::globbing::types::CompiledGlobs>,

    /// Compiled regular expressions reused across rows, bounded by the matching service.
    pub compiled_regexes: Patterns,

    /// The kinds of a language by the name they are given: a name is the same kind in every file.
    pub kind_names: hashbrown::HashMap<KindTest, KindSetPlace>,

    /// The kinds each name stands for, as the sets `kind_names` numbers.
    pub kind_sets: Vec<std::sync::Arc<KindSet>>,

    /// The comparisons of a filter that name a kind: the set of kinds that satisfy each.
    pub kind_tests: ComparisonNumbers,

    /// The comparisons of a filter that ask of the text of an element something fixed.
    pub gate: ComparisonNumbers,

    /// For each text of the file, whether those comparisons hold for it: zero not decided, one yes, two no.
    pub text_ok: TextVerdicts,

    /// The places of the elements of a tree that a filter on their kind leaves to be looked at.
    pub candidates: Candidates,

    /// Where the elements with each text of the file begin in `text_order`, one more than there are texts, for the file `text_file`.
    pub text_starts: TextStarts,

    /// The places of the elements, grouped by their text and in order within a text.
    pub text_order: ElementPlaces,

    /// The file and the number of elements `text_starts` and `text_order` were made for.
    pub text_file: TextGrouping,

    /// The comparisons of a filter that the kind sets and the text gate have not decided already.
    pub rest: ComparisonNumbers,
}

/// Mutable cycle work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct CycleScratch {
    /// The room a search for cycles works in.
    pub cycles: CycleRoom,
}

/// Compiled patterns, indexed by their exact source text.
#[derive(Debug, Default)]
pub(crate) struct Patterns(
    pub  hashbrown::HashMap<
        crate::domain::table_texts::types::RegexPattern,
        regex_automata::meta::Regex,
    >,
);
