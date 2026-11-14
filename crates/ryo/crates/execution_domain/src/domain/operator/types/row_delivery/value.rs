//! The `RowDelivery` value: the row side every port of one evaluation shares (the lease, the downstream sink
//! and the failure latch), with the operation, the inputs and the rendered answers the operators' row port reads.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::types::{FailureLatch, LatchState};
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::{
    FoundRows, PrimitiveLease, PrimitiveResources, PrimitiveWorkspace, RowSink,
};
use crate::domain::table_model::types::RepresentationInput;
use ryo_operators::domain::table_texts::service::datum;
use ryo_operators::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::{
    FirstOccurrence, GroupSummary, PredicateOutcome, RenderKind, RowRead, TextFound, TextScratch,
    VertexPlace,
};
use ryo_relations::domain::relations::types::{Count, Integer, NodeId, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The lease and sink of one evaluation, the first engine error it kept, and what its row port reads.
pub(crate) struct RowDelivery<'a, 'f> {
    lease: &'a mut PrimitiveLease<'f>,
    sink: &'a mut dyn RowSink,
    failure: FailureLatch,
    operation: &'a Operation,
    inputs: PreparedInputs<'a>,
    rendered: Vec<RenderOutcome>,
    buffer: Vec<Value>,
}

impl<'a, 'f> RowDelivery<'a, 'f> {
    /// The row side of one evaluation of `operation` over `inputs`, with no error kept yet.
    pub(crate) fn new(
        lease: &'a mut PrimitiveLease<'f>,
        sink: &'a mut dyn RowSink,
        operation: &'a Operation,
        inputs: PreparedInputs<'a>,
    ) -> Self {
        Self {
            lease,
            sink,
            failure: FailureLatch::default(),
            operation,
            inputs,
            rendered: Vec::new(),
            buffer: Vec::new(),
        }
    }

    /// Keeps the first engine error, and answers every later call as interrupted.
    pub(crate) fn hold<T>(&mut self, result: Result<T, OperatingError>) -> Result<T, Fault> {
        result.map_err(|error| {
            crate::domain::operator::contract::failure::service::record(&mut self.failure, error);
            Fault::Interrupted
        })
    }

    /// Whether an earlier engine error interrupted the evaluation: the latch is `Failed` once one was kept.
    pub(crate) fn interrupted(&self) -> LatchState {
        crate::domain::operator::contract::failure::service::failed(&self.failure)
    }

    /// The engine error kept, taken out of the latch.
    pub(crate) fn take_failure(&mut self) -> Option<OperatingError> {
        crate::domain::operator::contract::failure::service::take(&mut self.failure)
    }

    /// The prepared inputs of the evaluation.
    pub(crate) fn inputs(&self) -> PreparedInputs<'a> {
        self.inputs
    }

    /// The resources the invocation acquired.
    pub(crate) fn resources(&self) -> &'f dyn PrimitiveResources {
        self.lease.views.resources
    }

    /// The invocation's workspace, for the walks' work.
    pub(crate) fn workspace(&mut self) -> &mut PrimitiveWorkspace {
        self.lease.workspace
    }

    /// The invocation's workspace, read.
    pub(crate) fn workspace_view(&self) -> &PrimitiveWorkspace {
        self.lease.workspace
    }

    /// The answers of the last render.
    pub(crate) fn rendered_answers(&self) -> &[RenderOutcome] {
        &self.rendered
    }

    /// The lease, handed back when the evaluation ends.
    pub(crate) fn into_lease(self) -> &'a mut PrimitiveLease<'f> {
        self.lease
    }

    /// The bound predicate's verdict on a row.
    pub(crate) fn verdict(&mut self, row: &[Value]) -> Result<PredicateOutcome, OperatingError> {
        crate::domain::operator::contract::lease::service::holds(self.lease, row)
    }

    /// Counts one candidate a join found at its right input.
    pub(crate) fn candidate_found(&self) {
        crate::domain::operator::contract::lease::service::found(
            self.lease,
            InputNumber(1),
            FoundRows(1),
        );
        #[cfg(feature = "diagnostic-row-progress")]
        crate::domain::operator::contract::lease::service::progress(
            self.lease,
            crate::domain::operating::types::ProgressCounter::JoinCandidatePairs,
            crate::domain::operating::types::ProgressAmount(1),
        );
    }

    /// One population of a population-mode evaluation.
    pub(crate) fn borrowed(&self, input: InputNumber) -> Option<RepresentationInput<'a>> {
        match self.inputs {
            PreparedInputs::Views(views) => views.get(input.0).copied(),
            PreparedInputs::RequestSource(source) if input.0 == 0 => Some(*source),
            PreparedInputs::RequestSource(_) | PreparedInputs::Row { .. } => None,
        }
    }

    /// Delivers a row downstream.
    pub(crate) fn emit(&mut self, row: &[Value]) -> Result<(), Fault> {
        if self.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let result = self.sink.row(self.lease, row);
        self.hold(result)
    }

    /// Whether the bound predicate holds for a row.
    pub(crate) fn holds(&mut self, row: &[Value]) -> Result<PredicateOutcome, Fault> {
        if self.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let verdict = self.verdict(row);
        self.hold(verdict).map(|verdict| {
            if verdict == PredicateOutcome::Fails {
                PredicateOutcome::Fails
            } else {
                PredicateOutcome::Holds
            }
        })
    }

    /// Whether a row comes for the first time.
    pub(crate) fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, Fault> {
        if self.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let added =
            crate::domain::operator::contract::lease::service::distinct(self.lease).map(|state| {
                crate::domain::table_dedup::service::first_time(
                    &mut state.rows,
                    &mut state.seen,
                    row,
                ) == crate::domain::table_model::types::Addition::Added
            });
        self.hold(added).map(FirstOccurrence::from)
    }

    /// Counts the rows an input was found to hold.
    pub(crate) fn found(&self, input: InputNumber, rows: Count) {
        crate::domain::operator::contract::lease::service::found(
            self.lease,
            input,
            FoundRows(u64::try_from(rows.0).unwrap_or(u64::MAX)),
        );
    }

    /// The text of a value, appended to `target`.
    pub(crate) fn text(&self, value: Value, target: &mut TextScratch) -> TextFound {
        let texts = Texts {
            symbols: self.lease.derived.as_ref(),
            source: self.lease.views.source,
            paths: self.lease.views.paths,
            arenas: self.lease.views.arenas,
        };
        match datum(value, &texts) {
            Datum::Text(text) => {
                target.0.push_str(text.0);
                TextFound::Text
            }
            Datum::Unlocated
            | Datum::Number(_)
            | Datum::Flag(_)
            | Datum::Numbers(_)
            | Datum::Texts(_) => TextFound::NotText,
        }
    }

    /// A derived name's value, in the phase the evaluation runs in.
    pub(crate) fn mint(&mut self, text: CellText<'_>) -> Value {
        let made = query_intern_derived(
            &mut self.lease.derived,
            ByteSlice(self.lease.views.source.0),
            NameText(text.0),
        );
        crate::domain::operating::service::minted(
            self.lease.derived.as_ref(),
            made,
            self.lease.views.phase,
        )
    }

    /// A derived name, as a symbol.
    pub(crate) fn intern(&mut self, text: CellText<'_>) -> Value {
        Value::Symbol(query_intern_derived(
            &mut self.lease.derived,
            ByteSlice(self.lease.views.source.0),
            NameText(text.0),
        ))
    }

    /// How many rows an input population holds.
    pub(crate) fn population(&mut self, input: InputNumber) -> Result<RowCount, Fault> {
        let population = self
            .borrowed(input)
            .map(|rows| rows.count())
            .ok_or(OperatingError::NotRun);
        self.hold(population)
    }

    /// One row of an input population, into `row`.
    pub(crate) fn population_row(
        &mut self,
        input: InputNumber,
        at: RowIndex,
        row: &mut Vec<Value>,
    ) -> RowRead {
        let Some(rows) = self.borrowed(input) else {
            return RowRead::Missing;
        };
        match rows.read(at, &mut self.buffer) {
            Some(values) => {
                row.clear();
                row.extend_from_slice(values);
                RowRead::Read
            }
            None => RowRead::Missing,
        }
    }

    /// How many groups the input holds.
    pub(crate) fn groups(&mut self) -> Result<Count, Fault> {
        let groups = self
            .resources()
            .groups()
            .map(|groups| Count(groups.len()))
            .ok_or(OperatingError::NotRun);
        self.hold(groups)
    }

    /// One group's summary.
    pub(crate) fn group(&self, at: VertexPlace) -> Option<GroupSummary> {
        self.resources()
            .groups()?
            .get(at.0)
            .map(|group| GroupSummary {
                first: group.first,
                counted: Count(group.counted.0),
                digest: Integer(group.digest.0),
            })
    }

    /// Renders the syntax of nodes through the semantic session, keeping the answers.
    pub(crate) fn render(&mut self, nodes: &[NodeId]) -> Result<(), Fault> {
        let Operation::RenderSyntax { context, .. } = self.operation else {
            return self.hold(Err(OperatingError::WrongOperation));
        };
        let requests: Vec<RenderRequest> = nodes
            .iter()
            .map(|node| RenderRequest {
                node: *node,
                context: *context,
            })
            .collect();
        let answers = self
            .lease
            .views
            .semantic
            .ok_or_else(|| OperatingError::SemanticIncomplete {
                reason: crate::domain::operator::constants::NO_SEMANTIC_SESSION.into(),
            })
            .and_then(|lookup| lookup.render_batch(&requests, self.lease.views.source))
            .and_then(|answers| {
                if answers.len() == requests.len() {
                    Ok(answers)
                } else {
                    Err(OperatingError::SemanticBatchShape)
                }
            });
        self.rendered = self.hold(answers)?;
        Ok(())
    }

    /// One rendered answer's kind, its text appended to `text`.
    pub(crate) fn rendered(&self, at: RowIndex, text: &mut TextScratch) -> RenderKind {
        match self.rendered.get(at.0) {
            Some(RenderOutcome::Known(known)) => {
                text.0.push_str(&known.0);
                RenderKind::Known
            }
            Some(RenderOutcome::NotApplicable) | None => RenderKind::NotApplicable,
            Some(RenderOutcome::Unsupported(_)) => RenderKind::Unsupported,
        }
    }
}
