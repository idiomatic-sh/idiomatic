//! The types of the `lowering` concept: the rules it lowers and what a refusal names.

use super::constants::{
    CANNOT_BE_EVALUATED, PAST_REGISTERS, PAST_SITES, PAST_TABLE_COLUMNS, PAST_VOCABULARY,
    PAST_WORD_OPERANDS, PAST_WORDS, STEP,
};
use crate::domain::lowered::types::LoweredPack;
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::{NodeDigests, Plan};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// One selected rule: the plan its query compiled to, what its parameters are bound to, and the digest of each of its plan's
/// steps. The engine hashes nothing: the host computes the digests (`ryo_values::..::query_plans::service::node_digests`, with
/// its own hash) and hands them in as data.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::fixtures::{digests_of, named};
/// use ryo_engine::domain::lowering::types::SelectedRule;
/// use ryo_values::domain::query_bindings::types::RuleBindings;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let plan = named("one tree rule");
/// let digests = digests_of(&plan, RuleBindings(&[]));
/// let rule = SelectedRule { rule: SelectedRuleOrdinal(0), plan: &plan, bindings: RuleBindings(&[]), digests: &digests };
/// assert_eq!(rule.digests.0.len(), rule.plan.steps.len());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SelectedRule<'a> {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// Its compiled plan.
    pub plan: &'a Plan,
    /// What the host bound each of the plan's parameters to, in the plan's order: a parameter is a constant from the lowering on.
    pub bindings: RuleBindings<'a>,
    /// The digest of each step of the plan under those bindings, in step order: one for each step. A definition, a made
    /// table and the rule's own unit are named by the digest of the step whose rows they are.
    pub digests: &'a NodeDigests,
}

/// The selected rules, in rule order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::SelectedRules;
///
/// assert!(SelectedRules(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SelectedRules<'a>(pub &'a [SelectedRule<'a>]);

/// What the host declares for a run beside the plans: the rows of each relation it declares (a plan's `Hosted` step reads
/// the one at its place) and the producers of each bound observation a plan reads. The engine lowers both as rows of the
/// file it scans: a declared relation's rows are a constant table (a row naming a file is a row of that file alone), and a
/// bound observation is a definition whose branches are its producers' observed steps.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::LoadGiven;
///
/// let given = LoadGiven::default();
/// assert!(given.hosted.is_empty() && given.observed.is_empty());
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadGiven<'a> {
    /// The relations the host declares, with their rows, in the order a plan's `HostedNumber` counts them.
    pub hosted: &'a [ryo_values::domain::hosted::types::HostRelation],
    /// The producers of each bound observation of a selected rule.
    pub observed: &'a [ObservedBinding],
}

/// One bound observation of a selected rule: the rule, the binding its plan reads, the digest the host names the
/// observation's definition by (the engine hashes nothing), and the producers whose observed steps are its rows.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::{BoundProducer, ObservedBinding};
/// use ryo_values::domain::plan_parts::types::{ObservationBindingId, StepId};
/// use ryo_values::domain::query_plans::types::NodeDigest;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let bound = ObservedBinding {
///     consumer: SelectedRuleOrdinal(1),
///     binding: ObservationBindingId(0),
///     node: NodeDigest([7; 16]),
///     producers: vec![BoundProducer { rule: SelectedRuleOrdinal(0), step: StepId(3) }],
/// };
/// assert_eq!(bound.producers.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedBinding {
    /// The rule whose plan reads the observation.
    pub consumer: SelectedRuleOrdinal,
    /// The binding, by its place among the plan's bound inputs.
    pub binding: ryo_values::domain::plan_parts::types::ObservationBindingId,
    /// The digest of the observation's definition.
    pub node: ryo_values::domain::query_plans::types::NodeDigest,
    /// The producers, in the order the host lists them.
    pub producers: Vec<BoundProducer>,
}

/// A producer of a bound observation: a selected rule and the step of its plan whose rows it hands over.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::BoundProducer;
/// use ryo_values::domain::plan_parts::types::StepId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let producer = BoundProducer { rule: SelectedRuleOrdinal(0), step: StepId(3) };
/// assert_eq!(producer.step, StepId(3));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundProducer {
    /// The producing rule.
    pub rule: SelectedRuleOrdinal,
    /// The observed step of its plan.
    pub step: ryo_values::domain::plan_parts::types::StepId,
}

/// A step of a rule's plan, by its place in the plan's steps.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::PlanNode;
///
/// assert_ne!(PlanNode(0), PlanNode(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanNode(pub u32);

/// A rule the load did not accept.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::{LoadRefusal, PlanNode, RefusedRule};
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let refused = RefusedRule { rule: SelectedRuleOrdinal(1), refusal: LoadRefusal::Unlowerable(PlanNode(0)) };
/// assert_eq!(refused.rule, SelectedRuleOrdinal(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefusedRule {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// Why.
    pub refusal: LoadRefusal,
}

/// Why a rule is not loaded: it cannot be evaluated, or it cannot be written. Never its cost, and never its size.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::{LoadRefusal, PastWidth, PlanNode};
///
/// assert_eq!(LoadRefusal::Unlowerable(PlanNode(0)).to_string(), "step PlanNode(0) cannot be evaluated");
/// assert_eq!(LoadRefusal::Budget(PastWidth::Registers).to_string(), "more than 4,294,967,295 registers in the pack");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadRefusal {
    /// A step that cannot be evaluated, by its place in the rule's plan.
    Unlowerable(PlanNode),
    /// A count of the rule's programs, or of the pack as it stood when the rule was written, does not fit the width the code
    /// gives it.
    Budget(PastWidth),
}

/// What was counted past its width. The first three are counted in 16 bits (65,535), the others in 32.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::PastWidth;
///
/// assert_eq!(PastWidth::TableColumns.to_string(), "more than 65,535 columns in one table");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PastWidth {
    /// The operands of one word: the values of a match, the bodies of an `Or`, the slots of a `Fork`, the columns of a row.
    WordOperands,
    /// The columns of one table or definition.
    TableColumns,
    /// The kinds or the fields of the language's vocabulary.
    Vocabulary,
    /// The registers of the pack.
    Registers,
    /// The words of the pack, and the slots of its pool.
    Words,
    /// The sites of one of the pack's tables: memos, needs, indexes, points, constants.
    Sites,
}

impl std::fmt::Display for LoadRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unlowerable(step) => write!(formatter, "{STEP} {step:?} {CANNOT_BE_EVALUATED}"),
            Self::Budget(width) => width.fmt(formatter),
        }
    }
}

impl std::fmt::Display for PastWidth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::WordOperands => PAST_WORD_OPERANDS,
            Self::TableColumns => PAST_TABLE_COLUMNS,
            Self::Vocabulary => PAST_VOCABULARY,
            Self::Registers => PAST_REGISTERS,
            Self::Words => PAST_WORDS,
            Self::Sites => PAST_SITES,
        })
    }
}

/// What the lowering answers: the pack of the rules it accepted, and each rule it refused.
#[derive(Debug, Default)]
pub(crate) struct LoweredRules {
    /// The pack of the accepted rules.
    pub(crate) pack: LoweredPack,
    /// The refused rules, in rule order.
    pub(crate) refused: Vec<RefusedRule>,
}
