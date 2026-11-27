//! The types of the `table_scratch` concept: the invocation-owned rooms operators work in, plain
//! data with public fields and derives, and no constructor or inherent impl.

use super::constants::SCRATCH_GROUP_INITIALIZED;

use crate::domain::contract::types::PredicateOutcome;
use ryo_language::domain::elements::types::KindSum;
use ryo_relations::domain::relations::types::{KindBits, TextHash};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Which kinds a test asked of their names is about: the vocabulary of kinds it asked of, the test, and the hash of the text it is
/// against.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KindTest {
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

/// A test, by its place among the four that can name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamingPlace(pub usize);




/// What a comparison decided is remembered by the comparison and the hash of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VerdictKey {
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
pub struct KindSetPlace(pub usize);

/// A bit for each kind of a set, by the number of the kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KindMask(pub Vec<u64>);

/// How a name is compared, by its place among the tests that can name a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NamingTest(pub u8);


/// A place in the memo of what a comparison decided about a name.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::table_scratch::types::MemoSlot;
/// assert_eq!(MemoSlot(3).0, 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoSlot(pub usize);

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
    pub verdicts: hashbrown::HashMap<VerdictKey, PredicateOutcome>,

    /// Fixed glob operands made ready once for each comparison, for this run's bindings.
    pub compiled_globs: hashbrown::HashMap<
        ComparisonSlot,
        ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs,
    >,

    /// Compiled regular expressions reused across rows, bounded by the matching service.
    pub compiled_regexes: Patterns,

    /// The kinds of a language by the name they are given: a name is the same kind in every file.
    pub kind_names: hashbrown::HashMap<KindTest, KindSetPlace>,

    /// The kinds each name stands for, as the sets `kind_names` numbers.
    pub kind_sets: Vec<std::sync::Arc<KindSet>>,
}

/// Compiled patterns, indexed by their exact source text.
#[derive(Debug, Default)]
pub struct Patterns(
    pub  hashbrown::HashMap<
        crate::domain::table_texts::types::RegexPattern,
        ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
    >,
);

/// What a comparison decided about a name of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Memo {
    /// Not decided yet.
    Unknown,
    /// The comparison fails for the name.
    Fails,
    /// The comparison holds for the name.
    Holds,
}

/// What each comparison of one filter decided about each name of the open file, by its place, name-major: every place is
/// undecided but those decided since the filter's run began, which its next run forgets, one by one. Its room grows to the
/// places decided, not to the file's names: a filter that decides no name by its memo keeps none.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::table_scratch::types::MemoTable;
/// let memo = MemoTable::default();
/// assert_eq!(format!("{memo:?}"), "MemoTable { decided: [], written: [] }");
/// ```
#[derive(Debug, Default)]
pub struct MemoTable {
    /// What was decided at each place; a place past its end is undecided.
    pub decided: Vec<Memo>,
    /// The places decided since the run began.
    pub written: Vec<MemoSlot>,
}

/// Whether a value, a kind or an entity, is in a set of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetMembership {
    /// It is in the set.
    In,
    /// It is not.
    Out,
}
