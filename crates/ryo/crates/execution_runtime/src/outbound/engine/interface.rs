//! What the file scan asks of the engine: a rule set loaded from plans or from a pack, its pack's bytes, one file scanned, and the
//! cross-file pass. The host reads nothing else of a rule set, and no rule, language or linter is named here.

use ryo_engine::domain::findings::types::{FoundRows, FrontierSource, ScanCounters};
use ryo_engine::domain::packs::types::{EncodedPack, PackDigest};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::project::errors::ProjectPassError;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_language::domain::facts::types::{FileFacts, ProjectFacts};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What every `engine` adapter implements.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::SelectedRules;
/// use ryo_engine::inbound::use_cases::load::types::LoadSource;
/// use ryo_execution_runtime::outbound::engine::interface::EnginePort;
/// use ryo_values::domain::descriptions::types::KindVocabulary;
///
/// fn accepts_nothing(port: &dyn EnginePort, kinds: &KindVocabulary) -> bool {
///     port.load(LoadSource::Plans(SelectedRules(&[])), kinds)
///         .is_ok_and(|programs| port.accepted(&programs).is_empty())
/// }
/// ```
pub trait EnginePort: Send + Sync + std::fmt::Debug {
    /// The rule set of `source`: the selected rules lowered against `kinds`, or a pack read back.
    ///
    /// # Errors
    ///
    /// The first rule the engine cannot lower or that is over its budget; for a pack, why its bytes are not this rule set's.
    fn load(&self, source: LoadSource<'_>, kinds: &KindVocabulary) -> Result<Programs, LoadError>;

    /// The rules a rule set runs, in rule order: the ones `load` accepted.
    fn accepted(&self, programs: &Programs) -> Vec<SelectedRuleOrdinal>;

    /// The bytes of a rule set, written for `digest`: what the host's store keeps and `load` reads back.
    fn encoded(&self, programs: &Programs, digest: PackDigest) -> EncodedPack;

    /// A worker's per-file state for a rule set.
    fn state_for(&self, programs: &Programs) -> ScanScratch;

    /// Runs the rule set on one lent file, handing every found row to `found`.
    ///
    /// # Errors
    ///
    /// The fault of a rule whose program read a relation the language refused or filled in part.
    fn scan(
        &self,
        programs: &Programs,
        file: FileFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError>;

    /// Runs the rule set's cross-file programs once, after every file, over the frontier rows the files handed.
    ///
    /// # Errors
    ///
    /// The fault of a rule whose program read a relation the project refused.
    fn project(
        &self,
        programs: &Programs,
        frontier: &dyn FrontierSource,
        project: ProjectFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ProjectPassError>;
}
