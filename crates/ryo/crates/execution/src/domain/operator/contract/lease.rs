//! The lease's own operations: the capabilities a primitive reaches through its borrowed context.
use super::types::{
    DistinctWorkspace, FoundRows, PrimitiveLease, PrimitiveSemantics, PrimitiveViews, PrimitiveWorkspace,
    WorkCounters, WorkTotals, portable_fault,
};
use super::views::PredicateProgram;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::Value;

impl WorkCounters {
    pub(crate) fn found(&self, input: ryo_values::domain::plan_parts::types::InputNumber, rows: FoundRows) {
        if let Some(total) = self.0.get(input.0) {
            total.set(total.get().saturating_add(rows.0));
        }
    }
    pub(crate) fn take(&self) -> WorkTotals {
        WorkTotals(std::array::from_fn(|at| FoundRows(self.0[at].replace(0))))
    }
    pub(crate) fn snapshot(&self) -> WorkTotals {
        WorkTotals(std::array::from_fn(|at| FoundRows(self.0[at].get())))
    }
}

impl Default for PrimitiveWorkspace {
    fn default() -> Self {
        Self {
            row: Default::default(),
            other: Default::default(),
            text: Default::default(),
            word: Default::default(),
            stretches: Default::default(),
            captured: Default::default(),
            axis: Some(Default::default()),
            positions: Default::default(),
            dfs: Some(Default::default()),
            cycles: Some(Default::default()),
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
        counter: crate::domain::operator::contract::views::ProgressCounter,
        amount: crate::domain::operator::contract::views::ProgressAmount,
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
    pub(crate) fn project(
        &self,
        row: &[Value],
        columns: &[ryo_values::domain::plan_parts::types::ColumnIndex],
        projected: &mut Vec<Value>,
    ) -> Result<(), OperatingError> {
        match self.semantics.operators(
            ryo_operators::inbound::use_cases::operators::types::OperatorCall::Project {
                row,
                columns,
                target: projected,
            },
        ) {
            Ok(
                ryo_operators::inbound::use_cases::operators::types::OperatorAnswer::Written,
            ) => Ok(()),
            Err(error) => Err(portable_fault(error)),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn append(
        &self,
        row: &[Value],
        appended: &[Value],
        joined: &mut Vec<Value>,
    ) -> Result<(), OperatingError> {
        match self.semantics.operators(
            ryo_operators::inbound::use_cases::operators::types::OperatorCall::Append {
                row,
                appended,
                target: joined,
            },
        ) {
            Ok(
                ryo_operators::inbound::use_cases::operators::types::OperatorAnswer::Written,
            ) => Ok(()),
            Err(error) => Err(portable_fault(error)),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn span_bounds(
        &self,
        row: &[Value],
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Result<[Value; 2], OperatingError> {
        use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        match self
            .semantics
            .operators(OperatorCall::SpanBounds { row, column })
        {
            Ok(OperatorAnswer::SpanBounds(bounds)) => Ok(bounds),
            Err(error) => Err(portable_fault(error)),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn span_between(
        &self,
        row: &[Value],
        start: ryo_values::domain::plan_parts::types::ColumnIndex,
        end: ryo_values::domain::plan_parts::types::ColumnIndex,
        source_len: ryo_operators::domain::contract::types::SourceByteLength,
    ) -> Result<Value, OperatingError> {
        use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        match self.semantics.operators(OperatorCall::SpanBetween {
            row,
            start,
            end,
            source_len,
        }) {
            Ok(OperatorAnswer::SpanBetween(span)) => Ok(span),
            Err(error) => Err(portable_fault(error)),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    /// Only the bound semantic program can be requested by a primitive.
    pub(crate) fn holds(
        &mut self,
        row: &[Value],
    ) -> Result<crate::domain::table_model::types::Verdict, OperatingError> {
        self.predicate.holds(row, self.derived.as_ref())
    }
}
