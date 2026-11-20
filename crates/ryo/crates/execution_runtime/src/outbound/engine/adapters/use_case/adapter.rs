//! The `engine` port, implemented by the `engine` crate's use cases and the readers of its programs: every call passes through.

use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::findings::types::{FoundRows, FrontierSource, RuleColumns, ScanCounters};
use ryo_engine::domain::packs::service::encoded;
use ryo_engine::domain::packs::types::{EncodedPack, PackDigest};
use ryo_engine::domain::programs::service::{accepted, columns, scratch_for};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::interface::LoadUseCase;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::project::errors::ProjectPassError;
use ryo_engine::inbound::use_cases::project::interface::ProjectUseCase;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_engine::inbound::use_cases::scan::interface::ScanUseCase;
use ryo_language::domain::facts::types::{FileFacts, ProjectFacts};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The `use_case` `engine` adapter.
#[derive(Debug)]
pub(crate) struct EngineUseCaseAdapter<L, S, P> {
    /// The `load` use case.
    loader: L,
    /// The `scan` use case.
    scanner: S,
    /// The `project` use case.
    projector: P,
}

impl<L, S, P> EngineUseCaseAdapter<L, S, P> {
    /// The adapter over the use cases the factory built.
    pub(crate) const fn new(loader: L, scanner: S, projector: P) -> Self {
        Self {
            loader,
            scanner,
            projector,
        }
    }
}

impl<L, S, P> EnginePort for EngineUseCaseAdapter<L, S, P>
where
    L: LoadUseCase + std::fmt::Debug,
    S: ScanUseCase + std::fmt::Debug,
    P: ProjectUseCase + std::fmt::Debug,
{
    fn load(&self, source: LoadSource<'_>, kinds: &KindVocabulary) -> Result<Programs, LoadError> {
        self.loader.load(source, kinds)
    }

    fn accepted(&self, programs: &Programs) -> Vec<SelectedRuleOrdinal> {
        accepted(programs)
    }

    fn columns(&self, programs: &Programs) -> Vec<RuleColumns> {
        columns(programs)
    }

    fn encoded(&self, programs: &Programs, digest: PackDigest) -> EncodedPack {
        encoded(programs, digest)
    }

    fn state_for(&self, programs: &Programs) -> ScanScratch {
        scratch_for(programs)
    }

    fn scan(
        &self,
        programs: &Programs,
        file: FileFacts<'_>,
        _number: FileId,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError> {
        // The engine's `scan` takes no file number yet: a lent file does not say which file of the run it is.
        self.scanner.scan(programs, file, scratch, found)
    }

    fn project(
        &self,
        programs: &Programs,
        frontier: &dyn FrontierSource,
        project: ProjectFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ProjectPassError> {
        self.projector
            .project(programs, frontier, project, scratch, found)
    }
}
