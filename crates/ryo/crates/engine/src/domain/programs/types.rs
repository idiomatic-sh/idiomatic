//! The types of the `programs` concept: a rule set's programs and the instruction set they are written in. Everything but
//! `Programs` itself is the crate's own: the lowering writes it, the packs encode it, the executor runs it.
use crate::domain::code::types::ProgramCode;
use crate::domain::steps::types::StepRegex;
use ryo_language::domain::facts::types::BaseWords;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_plans::types::NodeDigests;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The programs of a rule set: opaque; built only by `load`, encoded by `packs::service`.
///
/// ```compile_fail,E0603
/// let forged = ryo_engine::domain::programs::types::Programs(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(accepted(&Programs::default()).is_empty());
/// ```
#[derive(Debug, Default)]
pub struct Programs(pub(crate) ProgramsEngine);

/// The private body of `Programs`: what a scan runs, after every load-time pass.
#[derive(Debug, Default)]
pub(crate) struct ProgramsEngine {
    /// The programs, encoded.
    pub(crate) code: ProgramCode,
    /// The constant pool: the language's fixed names, then every text a program compares with, as the run's base words.
    pub(crate) words: BaseWords,
    /// The regular expressions the programs match, by number, as their sources.
    pub(crate) regexes: Vec<StepRegex>,
    /// Every relation some program reads: what a language builds its model for (`RelationNeeds`).
    pub(crate) reads: Vec<Relation>,
    /// For each rule of the code, in the code's rule order: the digests of its own plan's steps, by which a unit's plan step is
    /// found for that rule (`service::unit_step`). A unit is named by a merged node, which is no step of any one plan.
    pub(crate) steps: Vec<RuleSteps>,
}

/// One rule's plan as a work notice needs it: where each node is in the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleSteps {
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// The step the plan emits: the rule's own unit.
    pub(crate) emitted: StepId,
    /// The digest of each step of the plan, in step order.
    pub(crate) nodes: NodeDigests,
}
