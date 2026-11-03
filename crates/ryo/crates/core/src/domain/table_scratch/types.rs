//! The types of the `table_scratch` concept: the invocation-owned rooms operators work in, plain
//! data with public fields and derives, and no constructor or inherent impl.

use crate::domain::table_cells::types::{ColumnPlaces, RowIndex};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_model::types::{KeyIndex, Table};
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

/// An entity met by a search for cycles, by the order it was met in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EntityNumber(pub usize);

/// An edge of a search for cycles, by its place among the edges leaving an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EdgePlace(pub usize);

/// An entity on the walk of a search for cycles, with the next edge to try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkStep {
    /// The entity.
    pub entity: EntityNumber,
    /// The next edge to try.
    pub edge: EdgePlace,
}

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
    /// Measurements copied from real engine-owned invocation resources in tests.
    #[cfg(test)]
    pub observation: crate::domain::operating::types::ExecutionObservationSlot,
    /// The cells of a row that a comparison reads, by their place in the row.
    pub needed: ColumnPlaces,

    /// A row read from the columns of a relation.
    pub row: Vec<Value>,

    /// A second row, for the operators that read two at once.
    pub other: Vec<Value>,
    /// Lazily constructed join work for this invocation.
    pub join_scratch: LazyScratch<JoinScratch>,
    /// Lazily constructed walk work for this invocation.
    pub walk_scratch: LazyScratch<WalkScratch>,
    /// Lazily constructed select work for this invocation.
    pub select_scratch: LazyScratch<SelectScratch>,
}

/// Mutable join work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct JoinScratch {
    /// The rows already kept, found by what they hold.
    pub seen: hashbrown::HashTable<RowIndex>,

    /// Reused by the engine's prepared Join lookup across rows in this invocation.
    pub key: Vec<Value>,
}

/// Mutable walk work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub(crate) struct WalkScratch {
    /// Which elements of a tree a domain admits, by their place.
    pub marks: Marks,

    /// For each element of a tree, its nearest ancestor a walk admits, once it is known.
    pub nearest: ElementPlaces,

    /// The elements whose nearest ancestor is being found, from the one asked about up.
    pub climbing: ElementPlaces,
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

/// Compiled patterns, indexed by their exact source text.
#[derive(Debug, Default)]
pub(crate) struct Patterns(
    pub  hashbrown::HashMap<
        crate::domain::table_texts::types::RegexPattern,
        regex_automata::meta::Regex,
    >,
);
