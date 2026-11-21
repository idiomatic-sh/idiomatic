//! The types of the `pack_load` operation: what a load and the harness are asked, and what a load made of the selected rules.

use crate::outbound::digest::interface::DigestPort;
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::statistics::types::MatchedStatistics;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, LanguageName};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredPack;

/// What a load is asked: the selected rules' plans and what each binds its parameters to, the statistics the rules are
/// lowered with, what the run was given as one digest, the kinds of the language whose files are scanned, and the packs the
/// store kept.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::LoadRequest;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new(), fields: Vec::new(), fixed: Vec::new() };
/// let request = LoadRequest { plans: &[], bindings: &[], statistics: ByteSlice(&[]), given: Digest([0; 16]), kinds: &kinds, stored: &[] };
/// assert!(request.stored.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct LoadRequest<'a> {
    /// The plan of each selected rule, in rule order.
    pub plans: &'a [&'a Plan],
    /// What each selected rule binds its plan's parameters to, in rule order: the host's bound values, which the engine lowers
    /// as constants. A rule past the end binds nothing.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// The statistics the rules are lowered with, as the bytes of the file that holds them: none, no statistics. They are part
    /// of what the run was given: `given` is made of them too, so a pack is kept under them.
    pub statistics: ByteSlice<'a>,
    /// What the run was given, as one digest: its rules as they were written, their fragments, what the host declared, which
    /// were selected and the statistics they are lowered with. Made before any compile.
    pub given: Digest,
    /// The kinds of the linked language the rules are lowered against.
    pub kinds: &'a KindVocabulary,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
}

/// What reading a rule set back from the store is asked: what the run was given as one digest, the kinds of the language whose
/// files are scanned, and the packs the store kept. No plan: nothing is compiled before it is asked.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::KeptRequest;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new(), fields: Vec::new(), fixed: Vec::new() };
/// let request = KeptRequest { given: Digest([0; 16]), kinds: &kinds, stored: &[] };
/// assert!(request.stored.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct KeptRequest<'a> {
    /// What the run was given, as one digest.
    pub given: Digest,
    /// The kinds of the linked language the rule set scans the files of.
    pub kinds: &'a KindVocabulary,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
}

/// The version of the engine that writes a pack: part of what the pack is kept under.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::EngineVersion;
///
/// assert_ne!(EngineVersion("0.2.0"), EngineVersion("0.3.0"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineVersion<'a>(pub &'a str);

/// What the edge composed for the build branch's `RYO_ENGINE` harness: the new engine, the digest its packs are keyed by, every
/// linked language's kinds, and the language the request names when it names one.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::Harness;
/// use ryo_values::domain::descriptions::types::KindVocabularies;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let vocabularies = KindVocabularies(Vec::new());
/// let harness = Harness { engine: &engine, digest: &digest, vocabularies: &vocabularies, named: None };
/// assert!(harness.named.is_none());
/// ```
#[derive(Clone, Copy)]
pub struct Harness<'a> {
    /// The new engine.
    pub engine: &'a dyn EnginePort,
    /// What a rule set's pack key is digested with.
    pub digest: &'a dyn DigestPort,
    /// The kinds of every linked language, in the order they were linked.
    pub vocabularies: &'a KindVocabularies,
    /// The language the request says reads every file, when it names one.
    pub named: Option<&'a LanguageName>,
}

impl std::fmt::Debug for Harness<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// What the harness is asked before the files of one request: the selected rules, the statistics they are lowered with,
/// what the run was given as one digest, and the packs the store kept.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::HarnessRequest;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = HarnessRequest { plans: &[], bindings: &[], statistics: ByteSlice(&[]), given: Digest([0; 16]), stored: &[] };
/// assert!(request.stored.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct HarnessRequest<'a> {
    /// The plan of each selected rule, in rule order.
    pub plans: &'a [&'a Plan],
    /// What each selected rule binds its plan's parameters to, in rule order.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// The statistics the rules are lowered with, as the bytes of the file that holds them: none, no statistics.
    pub statistics: ByteSlice<'a>,
    /// What the run was given, as one digest.
    pub given: Digest,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
}

/// What the harness is asked of the store before anything is compiled: what the run was given as one digest, how many rules it
/// selects, and the packs the store kept.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::ReadBackRequest;
/// use ryo_relations::domain::relations::types::Count;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = ReadBackRequest { given: Digest([0; 16]), selected: Count(0), stored: &[] };
/// assert_eq!(request.selected, Count(0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ReadBackRequest<'a> {
    /// What the run was given, as one digest.
    pub given: Digest,
    /// How many rules the request selects: what the harness reports the rule set's rules of.
    pub selected: Count,
    /// The packs the store kept.
    pub stored: &'a [StoredPack],
}

/// What the new engine's `load` made of the selected rules: the rule set, where it came from, and the pack the store keeps for it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::{HarnessLoad, LoadedFrom};
///
/// assert_eq!(HarnessLoad::default().from, LoadedFrom::default());
/// ```
#[derive(Debug, Default)]
pub struct HarnessLoad {
    /// The language whose kinds the rules were lowered against: the rule set scans that language's files.
    pub language: LanguageName,
    /// The programs of the accepted rules.
    pub programs: Programs,
    /// The accepted rules, in rule order.
    pub accepted: Vec<SelectedRuleOrdinal>,
    /// Whether the rules were lowered, or read back from the store's pack.
    pub from: LoadedFrom,
    /// What the statistics the rules were lowered with name of the programs: `Absent` for a load given none, or bytes that
    /// do not read, and for a pack read back.
    pub matched: MatchedStatistics,
    /// The pack of the rule set, under its key: what the store keeps for the next run.
    pub pack: StoredPack,
}

/// Where a rule set came from.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::{LoadedFrom, Refusals};
///
/// assert_ne!(LoadedFrom::Lowering(Refusals::default()), LoadedFrom::Pack);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadedFrom {
    /// Each selected rule was lowered: the rules refused, by why.
    Lowering(Refusals),
    /// The store's pack of the same digest was read back: nothing was lowered.
    Pack,
}

impl Default for LoadedFrom {
    fn default() -> Self {
        Self::Lowering(Refusals::default())
    }
}

/// The rules a lowering refused, by why.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::types::Refusals;
/// use ryo_relations::domain::relations::types::Count;
///
/// assert_eq!(Refusals::default().past_width, Count(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Refusals {
    /// The rules with a step the engine cannot evaluate.
    pub unlowerable: Count,
    /// The rules with a count past the width the engine's code gives it.
    pub past_width: Count,
}
