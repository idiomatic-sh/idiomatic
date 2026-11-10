//! The request-level observation schedule: which rules produce what others read, how their bound inputs resolve, and the
//! plans linked beneath them.
use ryo_values::domain::plans::types::Plan;

/// A linked request owns its remapped plans and typed producer references. Cached plans are untouched.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::types::LinkedObservationPlans;
/// let linked = LinkedObservationPlans { plans: Vec::new(), bindings: vec![Vec::new()] };
/// assert_eq!(linked.bindings.len(), 1);
/// ```
#[derive(Debug)]
pub struct LinkedObservationPlans {
    /// The plans, with their observation references remapped.
    pub plans: Vec<Plan>,
    /// The resolved producer bindings of each rule.
    pub bindings: Vec<Vec<ResolvedObservationBinding>>,
}

/// Ordinary requests need no named-root traversal allocation.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
/// let schedule = RuleSchedule::Dependencies {
///     order: vec![RuleIndex(1), RuleIndex(0)],
///     batches: vec![vec![RuleIndex(1)], vec![RuleIndex(0)]],
/// };
/// assert_ne!(schedule, RuleSchedule::Independent);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleSchedule {
    /// Preserve the existing independent original-index traversal.
    Independent,
    /// Original indices traversed once in dependency order and ready batches.
    Dependencies {
        /// The rules in dependency order.
        order: Vec<crate::domain::coverage::types::RuleIndex>,
        /// The rules of each ready batch, in order.
        batches: Vec<Vec<crate::domain::coverage::types::RuleIndex>>,
    },
}

/// A producer identity resolved once during request compilation.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer;
/// use ryo_values::domain::records::types::ObservationIndex;
/// let producer = ResolvedObservationProducer::Output { rule: RuleIndex(0), output: ObservationIndex(1) };
/// assert!(matches!(producer, ResolvedObservationProducer::Output { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedObservationProducer {
    /// An actual named root in a compiled producer plan.
    Output {
        /// The producing rule.
        rule: crate::domain::coverage::types::RuleIndex,
        /// The named output of the producer.
        output: ryo_values::domain::records::types::ObservationIndex,
    },
    /// Explicit unavailable selected query, never a fake empty root.
    NoQuery {
        /// The rule the selected query names.
        rule: ryo_values::domain::runs::types::RuleId,
    },
}

/// A resolved consumer input in its stable local slot.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::observation_schedule::types::{ResolvedObservationBinding, ResolvedObservationProducer};
/// use ryo_values::domain::observations::types::{BoundObservationInput, ObservationBindingName};
/// use ryo_values::domain::plan_parts::types::{ObservationBindingId, StepScope};
/// use ryo_values::domain::records::types::ObservationIndex;
/// let binding = ResolvedObservationBinding {
///     input: ObservationBindingId(0),
///     schema: BoundObservationInput {
///         name: ObservationBindingName("calls".into()),
///         columns: Vec::new(),
///         scope: StepScope::PerFile,
///     },
///     producers: vec![ResolvedObservationProducer::Output { rule: RuleIndex(0), output: ObservationIndex(0) }],
/// };
/// assert_eq!(binding.producers.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedObservationBinding {
    /// Local compiled input identity.
    pub input: ryo_values::domain::plan_parts::types::ObservationBindingId,
    /// The actual inferred schema and phase.
    pub schema: ryo_values::domain::observations::types::BoundObservationInput,
    /// Actual typed producers; no runtime name lookup remains.
    pub producers: Vec<ResolvedObservationProducer>,
}

/// Compile-time descriptors and their resolved graph references travel together.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::types::PreparedObservationInputs;
/// let prepared = PreparedObservationInputs { schemas: Vec::new(), bindings: Vec::new() };
/// assert_eq!(prepared.schemas.len(), prepared.bindings.len());
/// ```
#[derive(Debug)]
pub struct PreparedObservationInputs {
    /// Compiler declarations in original local input order.
    pub schemas: Vec<ryo_values::domain::observations::types::BoundObservationInput>,
    /// Corresponding resolved graph roots.
    pub bindings: Vec<ResolvedObservationBinding>,
}

/// The resolved request dependency contract borrowed by the existing file/cross graph owners.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::observation_schedule::types::ObservationRequest;
/// use ryo_values::domain::records::types::ObservationReadinessLayout;
/// use ryo_values::domain::stored_facts::types::Digest;
/// let bound = ObservationRequest::Bound { layout: ObservationReadinessLayout(Digest([0; 16])), bindings: &[] };
/// assert!(matches!(bound, ObservationRequest::Bound { .. }));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum ObservationRequest<'a> {
    /// No bound named roots: keep independent scheduling and no observation state.
    Independent,
    /// Actual resolved bindings and original-index topology, never a second executor.
    Bound {
        /// Where each rule's observations are made ready.
        layout: ryo_values::domain::records::types::ObservationReadinessLayout,
        /// The resolved bindings of each rule.
        bindings: &'a [Vec<ResolvedObservationBinding>],
    },
}

/// A persisted named root uses an existing frontier bucket, not another row store.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::records::types::ObservationIndex;
/// let root = ObservedFrontierRoot { producer: RuleIndex(0), output: ObservationIndex(0), frontier: InputNumber(2) };
/// assert_eq!(root.frontier, InputNumber(2));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ObservedFrontierRoot {
    /// The producing rule.
    pub producer: crate::domain::coverage::types::RuleIndex,
    /// The producer's named output.
    pub output: ryo_values::domain::records::types::ObservationIndex,
    /// The frontier the root is persisted in.
    pub frontier: ryo_values::domain::plan_parts::types::InputNumber,
}

/// How many bound inputs a plan declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingCount(pub usize);

/// The steps of one plan that linking keeps, by the step's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WantedSteps(pub Vec<bool>);

/// The named outputs of one plan that a consumer reads, by the output's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExportedOutputs(pub Vec<bool>);

/// The named outputs of one plan that a cross-file step reads, by the output's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EscapingOutputs(pub Vec<bool>);

/// The bound inputs of one rule that a kept step reads, by the input's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UsedInputs(pub Vec<bool>);

/// A step of one rule that linking has still to follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PendingStep {
    /// The rule the step belongs to.
    pub rule: crate::domain::coverage::types::RuleIndex,
    /// The step to follow.
    pub step: ryo_values::domain::plan_parts::types::StepId,
}
