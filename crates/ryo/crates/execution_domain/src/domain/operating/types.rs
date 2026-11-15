//! The types of the `operating` concept: what an operator reads, writes and fails with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::files::types::SourceView;

/// An application-owned, file-scoped semantic demand port. Operators borrow it;
/// they never own a provider session or its cache.
pub trait SemanticLookup: std::fmt::Debug {
    /// Resolve demanded keys in order; the current application deduplicates each batch.
    ///
    /// # Errors
    ///
    /// The implementation's [`OperatingError`]: a request it does not admit, a language without semantics, or an
    /// answer batch whose length differs from the requests'.
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        source: SourceView<'_>,
    ) -> Result<Vec<RenderOutcome>, OperatingError>;
}

/// One of the inputs of a step, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputPlace(pub usize);

/// Whether a comparison asks of the kind of an element something fixed, which the sets of kinds decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindNaming {
    /// It names a kind.
    Names,
    /// It does not.
    Silent,
}

/// Whether the steps run before the files were read, or after: after, the text of the workers is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterTheFiles {
    /// While the files are read.
    Before,
    /// After the files were read.
    After,
}

use ryo_values::domain::rows::types::{RowCount, Table};

/// Profile-only semantic observations. No execution or ownership policy is reachable.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug)]
pub enum ProgressCounter {
    /// Rows read from an input.
    InputRows,
    /// Predicate evaluations.
    PredicateEvaluations,
    /// Probes of a key index.
    IndexProbes,
    /// Rows an index probe examined.
    IndexRowsExamined,
    /// Rows an equality matched.
    EqualityMatches,
    /// Candidate pairs a join considered.
    JoinCandidatePairs,
}
/// How much one profile observation adds to its counter: the bare count inside is named here.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug)]
pub struct ProgressAmount(pub u64);
/// Receives the counts a profiled run observes.
#[cfg(feature = "diagnostic-row-progress")]
pub trait ProgressObserver: Sync {
    /// Adds `amount` to what `counter` has counted.
    fn observe(&self, counter: ProgressCounter, amount: ProgressAmount);
}
/// The observer of a run that profiles nothing.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Debug, Clone, Copy)]
pub struct IgnoredProgress;
#[cfg(feature = "diagnostic-row-progress")]
impl ProgressObserver for IgnoredProgress {
    fn observe(&self, _counter: ProgressCounter, _amount: ProgressAmount) {}
}

/// Borrowed portable primitive meanings; the application supplies the implementation.
/// No outbound adapter, cache owner or scheduler is reachable from a primitive.
pub trait PrimitiveSemantics: Sync {
    /// The observer the evaluation reports its row progress to.
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> &dyn ProgressObserver {
        &IgnoredProgress
    }

    /// The injected operators the primitive evaluates through.
    ///
    /// # Errors
    ///
    /// The [`PortableOperatorFault`](ryo_operators::domain::contract::types::PortableOperatorFault) the operator
    /// answers the call with.
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >;
}
