//! Pure preflight and reporting decisions over request data.
use super::types::*;
use ryo_execution_domain::domain::kind_check::types::KindRefusal;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindWord, OperatorDescription};
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::QueryOperator;
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
pub use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;

/// Decisions that inspect request metadata without owning its execution.
pub trait MetadataUseCase {
    /// The identity of the rule at `rule` among `ids`.
    fn rule_id(&self, ids: &[RuleId], rule: RuleIndex) -> RuleId;
    /// The refusal for a kind the plan names that no vocabulary knows, when it names one.
    fn kind_refusal(
        &self,
        plan: &Plan,
        vocabularies: &KindVocabularies,
        matching: &impl ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase,
    ) -> KindRefusalFound;
    /// The words the plan compares the kind of a node or a token with, once its parameters are bound.
    fn kinds_asked(&self, plan: &Plan, bound: &[ParameterBinding]) -> AskedKinds;
    /// The first word one of the plans writes for a kind that no linked language has a kind for.
    fn first_unknown(
        &self,
        plans: &[&Plan],
        bindings: PlanBindings<'_>,
        vocabularies: &KindVocabularies,
        matching: &impl ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase,
    ) -> UnknownKindFound;
    /// What is said of a word that names no kind: the word and the languages that were looked in.
    fn unknown_kind_message(&self, word: &KindWord, vocabularies: &KindVocabularies)
    -> KindRefusal;

    /// Decides the scopes of the rules: the globs in one list, whose each is whose, and the rules that include nothing.
    fn decide(&self, scopes: &[RuleScope]) -> ScopePlacement;
    /// Says which rules govern the file at `path`, filling `mask` and `hits`, which the worker owns and reuses.
    fn place(&self, placed: &PlacedScopes, path: &FilePath, mask: &mut Mask, hits: &mut Hits);
    /// Whether the rule governs the file the mask was made for.
    fn governs(&self, mask: &Mask, rule: RuleNumber) -> Governing;
    /// Whether any rule governs the file the mask was made for.
    fn governed(&self, mask: &Mask) -> Governing;
    /// Clears one rule from the file's mask without changing its path scope.
    fn unplace(&self, mask: &mut Mask, rule: RuleNumber);

    /// The features of the language a run is shown to exercise, in catalog order.
    fn feature_catalog(&self) -> FeatureCatalog;
    /// How many rows each step of each rule was fed, summed over its inputs.
    fn work_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        fed: FedRows<'_>,
    ) -> StepWorks;
    /// How many rows each feature of the language was fed over a run, rule by rule.
    fn fed_over(
        &self,
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        fed: FedRows<'_>,
    ) -> FeatureRowsByRule;

    /// The relations of a file that are read in part.
    fn partial_of(
        &self,
        said: &[ReadInPart],
        recovery: Recovery,
        wanted: &[Relation],
    ) -> PartialReads;
    /// What an operator that trusts an absence of rows says it could not decide, for a relation read in part.
    fn unknown_message(&self, partial: &Partial, scope: CoverageScope<'_>) -> Message;
    /// The path that names the repository as a whole where an unknown names a file: where a rule that decided nothing anywhere
    /// in the repository is unknown.
    fn repository(&self) -> FilePath;
    /// The ordinals of the files a partial project publication affects, sorted and once each: every file when a cause has no
    /// file.
    ///
    /// # Errors
    ///
    /// `ObservationIncomplete` when no file is affected, the request declares no file, or an affected file is not one of them.
    fn affected_ordinals(&self, request: AffectedFiles<'_>)
    -> Result<FileOrdinals, OperatingError>;

    /// The reads of the relations the steps of a plan trust the absence of rows of, by where the steps run.
    fn scoped_needs(&self, plan: &Plan) -> Needs;
    /// All physical reads needed to certify the demanded observations of a plan.
    fn observed_reads(&self, plan: &Plan) -> ObservedReads;
    /// Whether a partly read relation could change what a read of it makes.
    fn matters(&self, read: &RelationRead, partial: &Partial) -> Bearing;
    /// Whether any of the reads `needs` is of a relation read in part in the file.
    fn blocked(&self, needs: &[RelationRead], partial: &[Partial]) -> Blocking;
    /// What the rules could not decide because a relation of the project they trust the absence of rows of is read in part.
    fn project_unknowns(
        &self,
        partials: &[Partial],
        named: &[FilePath],
        needs: &[Needs],
        ids: &[RuleId],
    ) -> ProjectUnknowns;

    /// Every query operator.
    fn operators(&self) -> &'static [QueryOperator];
    /// What an operator is, what it takes and an example.
    fn describe(&self, operator: QueryOperator) -> OperatorDescription;
}
