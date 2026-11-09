//! The lease's own operations: the capabilities a primitive reaches through its borrowed context.
use super::types::{
    DistinctWorkspace, FoundRows, PrimitiveLease, PrimitiveViews, PrimitiveWorkspace,
    WorkCounters, WorkTotals,
};
use crate::domain::operating::types::PrimitiveSemantics;
use super::views::PredicateProgram;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::Value;

impl WorkCounters {
    /// Adds the rows an input was found to hold.
    pub fn found(&self, input: ryo_values::domain::plan_parts::types::InputNumber, rows: FoundRows) {
        if let Some(total) = self.0.get(input.0) {
            total.set(total.get().saturating_add(rows.0));
        }
    }
    /// The totals counted so far, resetting the counters.
    pub fn take(&self) -> WorkTotals {
        WorkTotals(std::array::from_fn(|at| FoundRows(self.0[at].replace(0))))
    }
}

impl Default for PrimitiveWorkspace {
    fn default() -> Self {
        Self {
            evaluation: Default::default(),
            axis: Some(Default::default()),
            positions: Default::default(),
            dfs: Some(Default::default()),
            closure_walk: Some(Default::default()),
            distinct: Some(Default::default()),
        }
    }
}

impl<'a> PrimitiveLease<'a> {
    pub(crate) fn distinct(&mut self) -> Result<&mut DistinctWorkspace, OperatingError> {
        let state = self
            .workspace
            .distinct
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        state.schema.as_ref().map_err(Clone::clone)?;
        Ok(state)
    }

    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) fn progress(
        &self,
        counter: crate::domain::operating::types::ProgressCounter,
        amount: crate::domain::operating::types::ProgressAmount,
    ) {
        self.semantics.progress().observe(counter, amount);
    }

    /// Copy the request-lived semantic port before borrowing this lease for row delivery.
    pub(crate) fn semantics(&self) -> &'a dyn PrimitiveSemantics {
        self.semantics
    }

    pub(super) fn new(
        views: PrimitiveViews<'a>,
        workspace: &'a mut PrimitiveWorkspace,
        derived: QuerySymbols<'a>,
        predicate: &'a mut dyn PredicateProgram,
        semantics: &'a dyn PrimitiveSemantics,
        work: &'a WorkCounters,
    ) -> Self {
        Self {
            views,
            workspace,
            derived,
            predicate,
            semantics,
            work,
        }
    }
    pub(crate) fn found(&self, input: ryo_values::domain::plan_parts::types::InputNumber, rows: FoundRows) {
        self.work.found(input, rows);
    }
    /// Only the bound semantic program can be requested by a primitive.
    pub(crate) fn holds(
        &mut self,
        row: &[Value],
    ) -> Result<crate::domain::table_model::types::Verdict, OperatingError> {
        self.predicate.holds(row, self.derived.as_ref())
    }
}
