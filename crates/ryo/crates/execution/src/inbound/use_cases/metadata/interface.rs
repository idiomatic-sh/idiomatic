//! Pure preflight and reporting decisions over request data.
use super::types::*;
use crate::domain::operating::errors::OperatingError;
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
pub use crate::domain::coverage::types::CoverageScope;
pub use crate::domain::coverage::types::Covered;
pub use crate::domain::coverage::types::FileObservationReadiness;
pub use crate::domain::coverage::types::FilePartial;
pub use crate::domain::coverage::types::Lacking;
pub use crate::domain::coverage::types::LackingKind;
pub use crate::domain::coverage::types::Message;
pub use crate::domain::coverage::types::Partial;
pub use crate::domain::coverage::types::Partials;
pub use crate::domain::coverage::types::Recovery;
pub use crate::domain::coverage::types::RuleIndex;
pub use crate::domain::coverage::types::Unknown;
pub use crate::domain::features::types::FeatureRows;
pub use crate::domain::features::types::StepWork;
pub use crate::domain::kind_check::types::AskedKind;
pub use crate::domain::kind_check::types::KindMatch;
pub use crate::domain::kind_check::types::KindRefusal;
pub use crate::domain::kind_check::types::LanguageList;
pub use crate::domain::kind_check::types::RulePlace;
pub use crate::domain::kind_check::types::UnknownKind;
pub use crate::domain::operator::types::RowsFed;
pub use crate::domain::placement::types::GlobOwner;
pub use crate::domain::placement::types::GlobSide;
pub use crate::domain::placement::types::Governing;
pub use crate::domain::placement::types::Mask;
pub use crate::domain::placement::types::PlacedScopes;
pub use crate::domain::placement::types::RuleScope;
pub use crate::domain::placement::types::ScopePlacement;
pub use crate::domain::trust::types::Bearing;
pub use crate::domain::trust::types::Blocking;
pub use crate::domain::trust::types::Needs;
pub use crate::domain::trust::types::RelationRead;

/// Decisions that inspect request metadata without owning its execution.
pub trait MetadataUseCase {
    fn rule_id(&self, ids: &[RuleId], rule: RuleIndex) -> RuleId;
    fn kind_refusal(&self, plan: &Plan, vocabularies: &KindVocabularies) -> Option<KindRefusal>;
    fn kinds_asked(&self, plan: &Plan, bound: &[ParameterBinding]) -> Vec<AskedKind>;
    fn first_unknown(
        &self,
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        vocabularies: &KindVocabularies,
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
