//! Pure preflight and reporting decisions over request data.
use super::types::*;
use ryo_execution_domain::domain::features::types::StepWork;
use ryo_execution_domain::domain::kind_check::types::{AskedKind, KindRefusal};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindWord, OperatorDescription};
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::run_metadata::types::Feature;
use ryo_values::domain::runs::types::{RuleId, RuleNumber};
use ryo_values::domain::stored_facts::types::ReadInPart;

// Canonical inbound signature vocabulary from private pure concepts.
pub use ryo_execution_domain::domain::coverage::types::CoverageScope;
pub use ryo_execution_domain::domain::coverage::types::Covered;
pub use ryo_execution_domain::domain::coverage::types::FileObservationReadiness;
pub use ryo_execution_domain::domain::coverage::types::FilePartial;
pub use ryo_execution_domain::domain::coverage::types::Lacking;
pub use ryo_execution_domain::domain::coverage::types::LackingKind;
pub use ryo_execution_domain::domain::coverage::types::Message;
pub use ryo_execution_domain::domain::coverage::types::Partial;
pub use ryo_execution_domain::domain::coverage::types::PartialPlace;
pub use ryo_execution_domain::domain::coverage::types::Partials;
pub use ryo_execution_domain::domain::coverage::types::Recovery;
pub use ryo_execution_domain::domain::coverage::types::RuleIndex;
pub use ryo_execution_domain::domain::coverage::types::Unknown;
pub use ryo_execution_domain::domain::features::types::FeatureRows;
pub use ryo_execution_domain::domain::kind_check::types::KindOrigin;
pub use ryo_execution_domain::domain::kind_check::types::UnknownKind;
pub use ryo_execution_domain::domain::operator::types::RowsFed;
pub use ryo_execution_domain::domain::placement::types::GlobOwner;
pub use ryo_execution_domain::domain::placement::types::GlobSide;
pub use ryo_execution_domain::domain::placement::types::Governing;
pub use ryo_execution_domain::domain::placement::types::Mask;
pub use ryo_execution_domain::domain::placement::types::PlacedScopes;
pub use ryo_execution_domain::domain::placement::types::RuleScope;
pub use ryo_execution_domain::domain::placement::types::ScopePlacement;
pub use ryo_execution_domain::domain::trust::types::Bearing;
pub use ryo_execution_domain::domain::trust::types::Blocking;
pub use ryo_execution_domain::domain::trust::types::Needs;
pub use ryo_execution_domain::domain::trust::types::RelationRead;

/// Decisions that inspect request metadata without owning its execution.
pub trait MetadataUseCase {
    fn rule_id(&self, ids: &[RuleId], rule: RuleIndex) -> RuleId;
    fn kind_refusal(
        &self,
        plan: &Plan,
        vocabularies: &KindVocabularies,
        matching: &dyn ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase,
    ) -> Option<KindRefusal>;
    fn kinds_asked(&self, plan: &Plan, bound: &[ParameterBinding]) -> Vec<AskedKind>;
    fn first_unknown(
        &self,
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        vocabularies: &KindVocabularies,
        matching: &dyn ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase,
    ) -> Option<UnknownKind>;
    fn unknown_kind_message(&self, word: &KindWord, vocabularies: &KindVocabularies)
    -> KindRefusal;

    fn decide(&self, scopes: &[RuleScope]) -> ScopePlacement;
    fn place(&self, placed: &PlacedScopes, path: &FilePath, mask: &mut Mask, hits: &mut Hits);
    fn governs(&self, mask: &Mask, rule: RuleNumber) -> Governing;
    fn governed(&self, mask: &Mask) -> Governing;
    fn unplace(&self, mask: &mut Mask, rule: RuleNumber);

    fn feature_catalog(&self) -> Vec<Feature>;
    fn work_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        fed: &[Vec<RowsFed>],
    ) -> Vec<Vec<StepWork>>;
    fn fed_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        fed: &[Vec<RowsFed>],
    ) -> Vec<FeatureRows>;

    fn partial_of(
        &self,
        said: &[ReadInPart],
        recovery: Recovery,
        wanted: &[Relation],
    ) -> Vec<Partial>;
    fn unknown_message(&self, partial: &Partial, scope: CoverageScope<'_>) -> Message;
    fn affected_ordinals(&self, request: AffectedFiles<'_>)
    -> Result<FileOrdinals, OperatingError>;

    fn scoped_needs(&self, plan: &Plan) -> Needs;
    fn observed_reads(&self, plan: &Plan) -> Vec<RelationRead>;
    fn matters(&self, read: &RelationRead, partial: &Partial) -> Bearing;
    fn blocked(&self, needs: &[RelationRead], partial: &[Partial]) -> Blocking;
    fn project_unknowns(
        &self,
        partials: &[Partial],
        named: &[FilePath],
        needs: &[Needs],
        ids: &[RuleId],
    ) -> Vec<Unknown>;

    fn operators(&self) -> &'static [QueryOperator];
    fn describe(&self, operator: QueryOperator) -> OperatorDescription;
}
