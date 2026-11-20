//! The decisions over request metadata the use cases take: the kernel's own, called here where an application service may name its
//! implementation, so a use case holds none; and which rule governs which file, decided once for a run.

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

/// Which rule governs which file: the scopes of the rules decided once, and their globs compiled.
///
/// # Errors
///
/// The rule whose glob is not one, with why; no rule for a pattern that names none.
pub(crate) fn placement(
    matching: &dyn ryo_execution::outbound::matching::interface::MatchingPort,
    rules: &[ryo_values::domain::rule_requests::types::RuleSpec],
    ids: &[RuleId],
) -> Result<PlacedScopes, ryo_values::domain::rule_requests::errors::RuleRequestsError> {
    use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
    let placement = kernel_metadata().decide(&crate::domain::run_rules::service::scopes_of(rules));
    let matcher = matching.compile(&placement.globs).map_err(|error| {
        let (owner, reason) = match error {
            MatchingError::BadGlob { glob, reason } => {
                let glob = ryo_values::domain::files::types::Glob(glob.into());
                let owner = crate::domain::run_rules::service::owner_of(&placement, &glob, ids);
                (owner.cloned().unwrap_or_default(), reason)
            }
            MatchingError::InvalidRegex { reason } => (RuleId::default(), reason),
        };
        ryo_values::domain::rule_requests::service::failed(&owner, &reason)
    })?;
    Ok(PlacedScopes { placement, matcher })
}
