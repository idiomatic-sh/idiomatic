//! The prepared program a caller holds: opaque, never minted by a caller.
use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::Reuse;
use std::sync::Arc;
/// The implementation layers are unavailable to callers. Each handle holds its members in one crate-private parts
/// struct, and the workspace denies `private_interfaces`: widening `parts` to `pub` exposes a crate-private type, so the
/// runtime's own build refuses it. The same lint holds `RequestExecution::open_project`, whose return type is crate-private.
///
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::execution::service::CommonOperatorEngine;
/// ```
/// The program compiler (`compile`) lives in `ryo_execution_domain`; `idiomatic.arch/library-surface` keeps it, the store
/// and the runtime's application from every crate outside the engine crates.
/// Public prepared handles can be accepted and inspected through the inbound contract.
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::{
///     interface::ProgramPreparedExecutionPort, types::PreparedExecutionProgram,
/// };
/// fn count(program: &PreparedExecutionProgram) -> usize { program.rule_count().0 }
/// ```
pub struct PreparedExecutionProgram {
    pub(crate) parts: ProgramParts,
}
/// The members of a prepared program, reachable only inside the runtime.
pub(crate) struct ProgramParts {
    pub(crate) kernels:
        Arc<dyn crate::application::file_dag::executor::service::PrimitiveExecutor + Send>,
    pub(crate) derived: crate::application::file_dag::derived::program::service::ProgramDerived,
    pub(crate) inner:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    /// What names the content of packed words, for every request of the program.
    pub(crate) digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
}
/// Whether a rule is wanted by the request, directly or through a named observation.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::Demand;
/// assert_ne!(Demand::Demanded, Demand::Undemanded);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Demand {
    Demanded,
    Undemanded,
}
/// Whether a rule declares or consumes named observations.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::ObservationContract;
/// assert_ne!(ObservationContract::Declared, ObservationContract::Absent);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationContract {
    Declared,
    Absent,
}
/// The readiness layout of the named observations and the frontiers that feed them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::NamedLayout;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::records::types::ObservationIndex;
/// use ryo_values::domain::records::types::ObservationReadinessLayout;
/// use ryo_values::domain::stored_facts::types::Digest;
/// let frontiers = [ObservedFrontierRoot { producer: RuleIndex(0), output: ObservationIndex(0), frontier: InputNumber(0) }];
/// let named = NamedLayout { layout: ObservationReadinessLayout(Digest([0; 16])), frontiers: &frontiers };
/// assert_eq!(named.frontiers.len(), 1);
/// ```
pub struct NamedLayout<'a> {
    pub layout: ObservationReadinessLayout,
    pub frontiers: &'a [ObservedFrontierRoot],
}
/// The rules a changed selection reaches and the producer frontiers they read.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::ReplayClosure;
/// use ryo_values::domain::stored_facts::types::Bucket;
/// use ryo_values::domain::stores::types::Reuse;
/// let closure = ReplayClosure { reuse: Reuse::Whole, buckets: vec![Bucket(0)] };
/// assert_eq!(closure, ReplayClosure { reuse: Reuse::Whole, buckets: vec![Bucket(0)] });
/// ```
#[derive(Debug, PartialEq)]
pub struct ReplayClosure {
    pub reuse: Reuse,
    pub buckets: Vec<Bucket>,
}

/// The order the program runs its rules in: the dependency order when rules depend on others, otherwise each rule by
/// its index.
#[derive(Clone, Debug)]
pub enum RuleOrder<'a> {
    /// Rules that depend on others, in dependency order.
    Dependencies(std::slice::Iter<'a, ryo_execution_domain::domain::coverage::types::RuleIndex>),
    /// Independent rules, by index: the next to run and the end.
    Independent {
        /// The next rule to run.
        next: ryo_execution_domain::domain::coverage::types::RuleIndex,
        /// One past the last rule.
        end: ryo_execution_domain::domain::coverage::types::RuleIndex,
    },
}

impl Iterator for RuleOrder<'_> {
    type Item = ryo_execution_domain::domain::coverage::types::RuleIndex;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Dependencies(order) => order.next().copied(),
            Self::Independent { next, end } => (next.0 < end.0).then(|| {
                let rule = *next;
                next.0 += 1;
                rule
            }),
        }
    }
}

/// The batches the observation rules run in, when the program has observation rules.
#[derive(Clone, Copy, Debug)]
pub struct RuleBatches<'a>(
    pub Option<&'a [Vec<ryo_execution_domain::domain::coverage::types::RuleIndex>]>,
);

/// The demand of each rule, by rule index.
#[derive(Clone, Debug, Default)]
pub struct Demands(pub Vec<Demand>);

/// The named layout of the program's observations, when it declares any.
pub struct ObservationLayout<'a>(pub Option<NamedLayout<'a>>);

/// The relations a rule's observation reads.
#[derive(Clone, Debug, Default)]
pub struct RelationReads(pub Vec<ryo_execution_domain::domain::trust::types::RelationRead>);

/// The observation bindings each frontier input of a rule depends on, by input.
#[derive(Clone, Copy, Debug)]
pub struct FrontierDependencies<'a>(
    pub &'a [Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>],
);

impl<'a> std::ops::Deref for RuleBatches<'a> {
    type Target = Option<&'a [Vec<ryo_execution_domain::domain::coverage::types::RuleIndex>]>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for Demands {
    type Target = Vec<Demand>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Demands {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for Demands {
    type Item = Demand;
    type IntoIter = std::vec::IntoIter<Demand>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Demands {
    type Item = &'a Demand;
    type IntoIter = std::slice::Iter<'a, Demand>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> std::ops::Deref for ObservationLayout<'a> {
    type Target = Option<NamedLayout<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for RelationReads {
    type Target = Vec<ryo_execution_domain::domain::trust::types::RelationRead>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RelationReads {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RelationReads {
    type Item = ryo_execution_domain::domain::trust::types::RelationRead;
    type IntoIter = std::vec::IntoIter<ryo_execution_domain::domain::trust::types::RelationRead>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RelationReads {
    type Item = &'a ryo_execution_domain::domain::trust::types::RelationRead;
    type IntoIter = std::slice::Iter<'a, ryo_execution_domain::domain::trust::types::RelationRead>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> std::ops::Deref for FrontierDependencies<'a> {
    type Target = &'a [Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
