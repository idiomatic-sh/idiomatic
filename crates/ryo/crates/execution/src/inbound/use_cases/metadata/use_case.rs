//! Inbound composition of private, pure execution metadata concepts.
use super::interface::MetadataUseCase;
use super::interface::*;
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

/// Stateless access to pure decisions. No program, owner, or permit is held here.
pub struct MetadataUseCaseImpl;

impl MetadataUseCase for MetadataUseCaseImpl {
    fn rule_id(&self, ids: &[RuleId], rule: RuleIndex) -> RuleId {
        ryo_execution_domain::domain::rules::service::id_at(ids, rule)
    }
    fn kind_refusal(
        &self,
        plan: &Plan,
        vocabularies: &KindVocabularies,
        matching: &impl ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase,
    ) -> Option<KindRefusal> {
        ryo_execution_domain::domain::kind_check::service::refusal(
            plan,
            vocabularies,
            ryo_execution_domain::domain::matching::types::PatternMatcher(matching),
        )
    }
    fn kinds_asked(&self, plan: &Plan, bound: &[ParameterBinding]) -> Vec<AskedKind> {
        ryo_execution_domain::domain::kind_check::service::kinds_asked(plan, bound)
    }
    fn first_unknown(
        &self,
        plans: &[Plan],
        bindings: &[Vec<ParameterBinding>],
        vocabularies: &KindVocabularies,
        matching: &impl ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase,
    ) -> Option<UnknownKind> {
        ryo_execution_domain::domain::kind_check::service::first_unknown(
            plans,
            bindings,
            vocabularies,
            ryo_execution_domain::domain::matching::types::PatternMatcher(matching),
        )
    }
    fn unknown_kind_message(
        &self,
        word: &KindWord,
        vocabularies: &KindVocabularies,
    ) -> KindRefusal {
        ryo_execution_domain::domain::kind_check::service::unknown_message(word, vocabularies)
    }
    fn decide(&self, scopes: &[RuleScope]) -> ScopePlacement {
        ryo_execution_domain::domain::placement::service::decide(scopes)
    }
    fn place(&self, placed: &PlacedScopes, path: &FilePath, mask: &mut Mask, hits: &mut Hits) {
        ryo_execution_domain::domain::placement::service::place(placed, path, mask, hits);
    }
    fn governs(&self, mask: &Mask, rule: RuleNumber) -> Governing {
        ryo_execution_domain::domain::placement::service::governs(mask, rule)
    }
    fn governed(&self, mask: &Mask) -> Governing {
        ryo_execution_domain::domain::placement::service::governed(mask)
    }
    fn unplace(&self, mask: &mut Mask, rule: RuleNumber) {
        ryo_execution_domain::domain::placement::service::unplace(mask, rule);
    }
    fn feature_catalog(&self) -> Vec<Feature> {
        ryo_execution_domain::domain::features::service::catalog()
    }
    fn work_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        fed: &[Vec<RowsFed>],
    ) -> Vec<Vec<StepWork>> {
        ryo_execution_domain::domain::features::service::work_over(metadata, fed)
    }
    fn fed_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        fed: &[Vec<RowsFed>],
    ) -> Vec<FeatureRows> {
        ryo_execution_domain::domain::features::service::fed_over(metadata, fed)
    }
    fn partial_of(
        &self,
        said: &[ReadInPart],
        recovery: Recovery,
        wanted: &[Relation],
    ) -> Vec<Partial> {
        ryo_execution_domain::domain::coverage::service::partial_of(said, recovery, wanted)
    }
    fn unknown_message(&self, partial: &Partial, scope: CoverageScope<'_>) -> Message {
        ryo_execution_domain::domain::coverage::service::unknown_message(partial, scope)
    }
    fn affected_ordinals(
        &self,
        request: AffectedFiles<'_>,
    ) -> Result<FileOrdinals, OperatingError> {
        ryo_execution_domain::domain::coverage::service::affected_ordinals(
            request.affected,
            ryo_execution_domain::domain::coverage::types::DeclaredFiles(request.file_count.0),
        )
        .map(|ordinals| FileOrdinals(ordinals.into_iter().map(|ordinal| ordinal.0).collect()))
    }
    fn scoped_needs(&self, plan: &Plan) -> Needs {
        ryo_execution_domain::domain::trust::service::scoped_needs(plan)
    }
    fn observed_reads(&self, plan: &Plan) -> Vec<RelationRead> {
        ryo_execution_domain::domain::trust::service::observed_reads(plan)
    }
    fn matters(&self, read: &RelationRead, partial: &Partial) -> Bearing {
        ryo_execution_domain::domain::trust::service::matters(read, partial)
    }
    fn blocked(&self, needs: &[RelationRead], partial: &[Partial]) -> Blocking {
        ryo_execution_domain::domain::trust::service::blocked(needs, partial)
    }
    fn project_unknowns(
        &self,
        partials: &[Partial],
        named: &[FilePath],
        needs: &[Needs],
        ids: &[RuleId],
    ) -> Vec<Unknown> {
        ryo_execution_domain::domain::trust::service::project_unknowns(partials, named, needs, ids)
    }
    fn operators(&self) -> &'static [QueryOperator] {
        &ryo_execution_domain::domain::operator::constants::OPERATORS
    }
    fn describe(&self, operator: QueryOperator) -> OperatorDescription {
        ryo_execution_domain::domain::operator::constants::describe(operator)
    }
}
