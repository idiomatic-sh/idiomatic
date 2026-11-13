//! The decisions over request metadata the use cases take: the kernel's own, called here where an application service may name its
//! implementation, so a use case holds none.

use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;

use crate::domain::workers::types::RunDecisions;
use ryo_execution::inbound::use_cases::metadata::interface::{
    Bearing, Blocking, CoverageScope, Governing, Mask, Message, Partial, PlacedScopes, Recovery,
    RelationRead, RuleIndex,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::runs::types::{RuleId, RuleNumber};
use ryo_values::domain::stored_facts::types::ReadInPart;

/// The kernel's decisions over request metadata, for a use case to call.
#[must_use]
pub(crate) fn kernel_metadata() -> impl MetadataUseCase {
    MetadataUseCaseImpl
}

/// Any metadata use case answers the domain's decisions over a run's rules: the application hands it down, so the domain names
/// no use case.
impl<Metadata: MetadataUseCase> RunDecisions for Metadata {
    fn rule_id(&self, ids: &[RuleId], rule: RuleIndex) -> RuleId {
        MetadataUseCase::rule_id(self, ids, rule)
    }
    fn place(&self, placed: &PlacedScopes, path: &FilePath, mask: &mut Mask, hits: &mut Hits) {
        MetadataUseCase::place(self, placed, path, mask, hits);
    }
    fn governs(&self, mask: &Mask, rule: RuleNumber) -> Governing {
        MetadataUseCase::governs(self, mask, rule)
    }
    fn governed(&self, mask: &Mask) -> Governing {
        MetadataUseCase::governed(self, mask)
    }
    fn partial_of(
        &self,
        said: &[ReadInPart],
        recovery: Recovery,
        wanted: &[Relation],
    ) -> Vec<Partial> {
        MetadataUseCase::partial_of(self, said, recovery, wanted).0
    }
    fn matters(&self, read: &RelationRead, partial: &Partial) -> Bearing {
        MetadataUseCase::matters(self, read, partial)
    }
    fn unknown_message(&self, partial: &Partial, scope: CoverageScope<'_>) -> Message {
        MetadataUseCase::unknown_message(self, partial, scope)
    }
    fn blocked(&self, needs: &[RelationRead], partial: &[Partial]) -> Blocking {
        MetadataUseCase::blocked(self, needs, partial)
    }
}
