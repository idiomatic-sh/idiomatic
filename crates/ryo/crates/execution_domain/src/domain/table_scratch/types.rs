//! The types of the `table_scratch` concept: the invocation-owned rooms operators work in, plain
//! data with public fields and derives, and no constructor or inherent impl.

use super::constants::SCRATCH_GROUP_INITIALIZED;
use crate::domain::table_cells::types::ColumnPlaces;

use crate::domain::table_model::types::Verdict;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_language::domain::elements::types::KindSum;
use ryo_relations::domain::relations::types::{KindBits, TextHash, Value};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Which kinds a test asked of their names is about: the vocabulary of kinds it asked of, the test, and the hash of the text it is
/// against.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KindTest {
    /// The names of the kinds of the file it was asked of: a set of kinds is only as good as the numbering of the language that made it.
    pub vocabulary: KindSum,
    /// How the name is compared, by its place among the tests that can name a kind.
    pub test: NamingTest,
    /// The exact literal text the name is compared with.
    pub text: LiteralText,
}

/// The kinds of a language that have one name: as a set to ask of, and as a list to walk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KindSet {
    /// A bit for each kind, by its number.
    pub bits: KindMask,
    /// The kinds, in order.
    pub kinds: Vec<KindBits>,
}

/// Comparisons of a filter, by their place among the filter's comparisons.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ComparisonNumbers(pub Vec<usize>);

/// A test, by its place among the four that can name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NamingPlace(pub usize);

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
pub struct ComparisonSlot(pub usize);

/// The number of a kind set among the sets of a language's kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KindSetPlace(pub usize);

/// A bit for each kind of a set, by the number of the kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KindMask(pub Vec<u64>);

/// How a name is compared, by its place among the tests that can name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NamingTest(pub u8);

/// The hash of a name's text, or the number of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SubjectHash(pub u64);

/// A place in the memo of what a comparison decided about a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoSlot(pub usize);

/// A heavy operator group, constructed only when an operation first accesses it.
/// Its box and every payload are owned by the enclosing rule invocation.
#[derive(Debug, Default)]
pub struct LazyScratch<T>(pub std::cell::OnceCell<Box<T>>);

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
        self.0.get_mut().expect(SCRATCH_GROUP_INITIALIZED)
    }
}

/// Cheap row buffers and lazy heavy groups owned only by the current rule.
#[derive(Debug, Default)]
pub struct OperatorScratch {
    /// Lazily constructed select work for this invocation.
    pub select_scratch: LazyScratch<SelectScratch>,
}

/// Mutable select work, allocated only by operators that use it.
#[derive(Debug, Default)]
pub struct SelectScratch {
    /// What a comparison decided about a text, by the comparison and the hash of the text: the same text
    /// is decided once for the run, not once for each file that has it.
    pub(crate) verdicts: hashbrown::HashMap<VerdictKey, Verdict>,

    /// Fixed glob operands made ready once for each comparison, for this run's bindings.
    pub compiled_globs: hashbrown::HashMap<
        ComparisonSlot,
        ryo_matching::inbound::use_cases::matching::compiled::CompiledTextGlobs,
    >,

    /// Compiled regular expressions reused across rows, bounded by the matching service.
    pub compiled_regexes: Patterns,

    /// The kinds of a language by the name they are given: a name is the same kind in every file.
    pub(crate) kind_names: hashbrown::HashMap<KindTest, KindSetPlace>,

    /// The kinds each name stands for, as the sets `kind_names` numbers.
    pub(crate) kind_sets: Vec<std::sync::Arc<KindSet>>,

    /// The comparisons of a filter that name a kind: the set of kinds that satisfy each.
    pub(crate) kind_tests: ComparisonNumbers,

}

/// Compiled patterns, indexed by their exact source text.
#[derive(Debug, Default)]
pub struct Patterns(
    pub  hashbrown::HashMap<
        crate::domain::table_texts::types::RegexPattern,
        ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
    >,
);

use ryo_values::domain::rows::types::{RowIndex, Table};
