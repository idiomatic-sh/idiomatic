//! Inbound composition of private, pure execution metadata concepts.
use super::interface::MetadataUseCase;
use super::interface::*;
use super::types::*;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindWord, OperatorDescription};
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::plans::types::{Feature, Plan, QueryOperator};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::runs::types::{RuleId, RuleNumber};
use ryo_values::domain::stored_facts::types::ReadInPart;

/// Stateless access to pure decisions. No program, owner, or permit is held here.
pub struct MetadataUseCaseImpl;

impl MetadataUseCase for MetadataUseCaseImpl {
    fn rule_id(&self, ids: &[RuleId], rule: RuleIndex) -> RuleId {
        crate::domain::rules::service::id_at(ids, rule)
    }
    fn kind_refusal(&self, plan: &Plan, vocabularies: &KindVocabularies) -> Option<KindRefusal> {
        crate::domain::kind_check::service::refusal(plan, vocabularies)
    }
    fn kinds_asked(&self, plan: &Plan, bound: &[ParameterBinding]) -> Vec<AskedKind> {
        crate::domain::kind_check::service::kinds_asked(plan, bound)
    }
    fn first_unknown(
        &self,
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        vocabularies: &KindVocabularies,
    ) -> Option<UnknownKind> {
        crate::domain::kind_check::service::first_unknown(plans, bindings, vocabularies)
    }
    fn unknown_kind_message(
        &self,
        word: &KindWord,
        vocabularies: &KindVocabularies,
    ) -> KindRefusal {
        crate::domain::kind_check::service::unknown_message(word, vocabularies)
    }
    fn decide(&self, scopes: &[RuleScope]) -> ScopePlacement {
        crate::domain::placement::service::decide(scopes)
    }
    fn place(&self, placed: &PlacedScopes, path: &FilePath, mask: &mut Mask, hits: &mut Hits) {
        crate::domain::placement::service::place(placed, path, mask, hits);
    }
    fn governs(&self, mask: &Mask, rule: RuleNumber) -> Governing {
        crate::domain::placement::service::governs(mask, rule)
    }
    fn governed(&self, mask: &Mask) -> Governing {
        crate::domain::placement::service::governed(mask)
    }
    fn unplace(&self, mask: &mut Mask, rule: RuleNumber) {
        crate::domain::placement::service::unplace(mask, rule);
    }
    fn feature_catalog(&self) -> Vec<Feature> {
        crate::domain::features::service::catalog()
    }
    fn work_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleMetadata],
        fed: &[Vec<RowsFed>],
    ) -> Vec<Vec<StepWork>> {
        crate::domain::features::service::work_over(metadata, fed)
    }
    fn fed_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleMetadata],
        fed: &[Vec<RowsFed>],
    ) -> Vec<FeatureRows> {
        crate::domain::features::service::fed_over(metadata, fed)
    }
    fn partial_of(
        &self,
        said: &[ReadInPart],
        recovery: Recovery,
        wanted: &[Relation],
    ) -> Vec<Partial> {
        crate::domain::coverage::service::partial_of(said, recovery, wanted)
    }
    fn unknown_message(&self, partial: &Partial, scope: CoverageScope<'_>) -> Message {
        crate::domain::coverage::service::unknown_message(partial, scope)
    }
    fn affected_ordinals(
        &self,
        request: AffectedFiles<'_>,
    ) -> Result<FileOrdinals, OperatingError> {
        crate::domain::coverage::service::affected_ordinals(request.affected, request.file_count.0)
            .map(FileOrdinals)
    }
    fn scoped_needs(&self, plan: &Plan) -> Needs {
        crate::domain::trust::service::scoped_needs(plan)
    }
    fn observed_reads(&self, plan: &Plan) -> Vec<RelationRead> {
        crate::domain::trust::service::observed_reads(plan)
    }
    fn matters(&self, read: &RelationRead, partial: &Partial) -> Bearing {
        crate::domain::trust::service::matters(read, partial)
    }
    fn blocked(&self, needs: &[RelationRead], partial: &[Partial]) -> Blocking {
        crate::domain::trust::service::blocked(needs, partial)
    }
    fn project_unknowns(
        &self,
        partials: &[Partial],
        named: &[FilePath],
        needs: &[Needs],
        ids: &[RuleId],
    ) -> Vec<Unknown> {
        crate::domain::trust::service::project_unknowns(partials, named, needs, ids)
    }
    fn operators(&self) -> &'static [QueryOperator] {
        &crate::domain::operator::constants::OPERATORS
    }
    fn describe(&self, operator: QueryOperator) -> OperatorDescription {
        crate::domain::operator::constants::describe(operator)
    }
}
