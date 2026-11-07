//! The engine's side of one evaluation: the lease, the downstream sink and the prepared inputs behind the
//! operators' driver ports. Every kind's row meaning is the operators'; each strategy the engine keeps (an
//! index probe, native tree candidates, its walks over trees, partitions and graphs) implements one port.
use super::failure::{FailureLatch, LatchState};
use super::types::{FoundRows, PreparedInputs, PrimitiveLease, PrimitiveResources, RowSink};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::errors::ContractError;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::cycles::types::CycleDriver;
use ryo_operators::domain::equality_join::types::JoinDriver;
use ryo_operators::domain::evaluation::types::{
    AxisDriver, ClosureDriver, DfsDriver, EvaluationInput, EvaluationViews, GroupSummary,
    InvocationDriver, RightColumns, RowRead, TextFound, VertexPlace,
};
use ryo_operators::domain::ordered_match::types::OrderedMatchDriver;
use ryo_operators::domain::render_batch::types::RenderKind;
use ryo_operators::domain::select::types::PredicateOutcome;
use ryo_operators::domain::set_rows::errors::SetRowsError;
use ryo_operators::domain::set_rows::types::{
    FirstOccurrence, RightMembership, RightPopulation, SetRowDriver,
};
use ryo_operators::domain::source_matches::types::TextRowDriver;
use ryo_relations::domain::relations::types::{Count, Integer, NodeId, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The lease, sink and inputs of one evaluation, with the cursor state of the engine's strategies.
pub(crate) struct LeaseDriver<'a, 'f> {
    pub(super) lease: &'a mut PrimitiveLease<'f>,
    pub(super) sink: &'a mut dyn RowSink,
    pub(super) operation: &'a Operation,
    pub(super) inputs: PreparedInputs<'a>,
    pub(super) resources: &'f dyn PrimitiveResources,
    pub(super) failure: FailureLatch,
    pub(super) join: super::join::JoinCursor<'f>,
    pub(super) axis: super::axis::AxisCursor<'f>,
    pub(super) graph: super::graph::GraphCursor<'f>,
    pub(super) rendered: Vec<ryo_language::domain::semantics::types::RenderOutcome>,
    pub(super) left_buffer: Vec<Value>,
    pub(super) right_buffer: Vec<Value>,
}

impl<'a, 'f> LeaseDriver<'a, 'f> {
    /// Keeps the first engine error, and answers every later call as interrupted.
    pub(super) fn hold<T>(&mut self, result: Result<T, OperatingError>) -> Result<T, Fault> {
        result.map_err(|error| {
            self.failure.record(error);
            Fault::Interrupted
        })
    }

    /// Whether an earlier engine error interrupted the evaluation.
    pub(super) fn interrupted(&self) -> bool {
        self.failure.failed() == LatchState::Failed
    }

    /// The scheduled left row of a row-mode evaluation.
    pub(super) fn left(&self) -> &'a [Value] {
        match self.inputs {
            PreparedInputs::Row { row, .. } => row,
            PreparedInputs::Views(_) | PreparedInputs::RequestSource(_) => &[],
        }
    }

    /// One population of a population-mode evaluation.
    fn borrowed(
        &self,
        input: InputNumber,
    ) -> Option<crate::domain::table_model::types::RepresentationInput<'a>> {
        match self.inputs {
            PreparedInputs::Views(views) => views.get(input.0).copied(),
            PreparedInputs::RequestSource(source) if input.0 == 0 => Some(*source),
            PreparedInputs::RequestSource(_) | PreparedInputs::Row { .. } => None,
        }
    }
}

/// Evaluates one operation through the operators' one meaning of its kind: the engine lends its views, its
/// scratch and this driver, and keeps the first engine error ahead of any portable answer.
pub(crate) fn evaluate(
    operation: &Operation,
    inputs: PreparedInputs<'_>,
    lease: &mut PrimitiveLease<'_>,
    sink: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    let semantics = lease.semantics();
    let resources = lease.views.resources;
    let views = EvaluationViews {
        source: ryo_operators::domain::source_matches::types::LineBytes(lease.views.source.0),
        bindings: lease.views.bindings,
        regex: lease.views.regex,
        checked: lease.views.validated_source.map(|checked| {
            ryo_operators::domain::evaluation::types::CheckedSource {
                base: checked.base,
                text: checked.text,
                line: checked.line,
            }
        }),
        appended: resources.appended(),
        layers: resources.rank_layers(),
        cached_reach: lease.views.cached_reach.map(|reach| {
            ryo_operators::domain::evaluation::types::CachedReach {
                request: reach.request,
                destinations: reach.destinations,
            }
        }),
        cached_axis: lease.views.cached_axis.map(|axis| axis.destinations),
    };
    let input = match inputs {
        PreparedInputs::Row { row, .. } => EvaluationInput::Row(row),
        PreparedInputs::Views(_) | PreparedInputs::RequestSource(_) => EvaluationInput::Populations,
    };
    let mut scratch = std::mem::take(&mut lease.workspace.evaluation);
    let positions = std::mem::take(&mut lease.workspace.positions);
    let mut driver = LeaseDriver {
        lease,
        sink,
        operation,
        inputs,
        resources,
        failure: FailureLatch::default(),
        join: super::join::JoinCursor::new(resources, operation, positions),
        axis: super::axis::AxisCursor::default(),
        graph: super::graph::GraphCursor::default(),
        rendered: Vec::new(),
        left_buffer: Vec::new(),
        right_buffer: Vec::new(),
    };
    let answer = semantics.operators(
        ryo_operators::inbound::use_cases::operators::types::OperatorCall::Evaluate {
            operation,
            input,
            views,
            driver: &mut driver,
            scratch: &mut scratch,
        },
    );
    let latched = driver.failure.take();
    let refusal = match answer {
        Err(Fault::MissingCapture) if latched.is_none() => missing_capture(operation),
        Err(Fault::UnsupportedRender) if latched.is_none() => {
            Some(unsupported_render(&driver.rendered))
        }
        _ => None,
    };
    let LeaseDriver { lease, join, .. } = driver;
    lease.workspace.positions = join.give_back();
    lease.workspace.evaluation = scratch;
    if let Some(refusal) = refusal {
        return Err(refusal);
    }
    super::types::written_result(answer, latched)
}

/// A named capture the matcher does not have is an invalid regular expression of the operation.
fn missing_capture(operation: &Operation) -> Option<OperatingError> {
    let Operation::SourceMatches { capture, .. } = operation else {
        return None;
    };
    Some(OperatingError::InvalidRegex {
        reason: format!(
            "missing named capture `{}`",
            capture.as_ref().map(|name| &*name.0).unwrap_or_default()
        ),
    })
}

/// A provider's unsupported answer invalidates the consuming rule, with the provider's reason.
fn unsupported_render(
    answers: &[ryo_language::domain::semantics::types::RenderOutcome],
) -> OperatingError {
    use ryo_language::domain::semantics::types::RenderOutcome;
    let reason = answers.iter().find_map(|answer| match answer {
        RenderOutcome::Unsupported(reason) => Some(reason.0),
        RenderOutcome::Known(_) | RenderOutcome::NotApplicable => None,
    });
    OperatingError::SemanticIncomplete {
        reason: format!(
            "selected language cannot render this syntax (reason {})",
            reason.map_or_else(String::new, |reason| reason.to_string())
        ),
    }
}

impl InvocationDriver for LeaseDriver<'_, '_> {
    fn emit(&mut self, row: &[Value]) -> Result<(), Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let result = self.sink.row(self.lease, row);
        self.hold(result)
    }
    fn holds(&mut self, row: &[Value]) -> Result<PredicateOutcome, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let verdict = self.lease.holds(row);
        self.hold(verdict).map(|verdict| {
            if verdict == Verdict::Fails {
                PredicateOutcome::Fails
            } else {
                PredicateOutcome::Holds
            }
        })
    }
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let added = self.lease.distinct().map(|state| {
            crate::domain::table_dedup::service::first_time(&mut state.rows, &mut state.seen, row)
                == crate::domain::table_model::types::Addition::Added
        });
        self.hold(added).map(FirstOccurrence::from)
    }
    fn found(&mut self, input: InputNumber, rows: Count) {
        self.lease
            .found(input, FoundRows(u64::try_from(rows.0).unwrap_or(u64::MAX)));
    }
    fn text(&mut self, value: Value, target: &mut String) -> TextFound {
        let texts = Texts {
            symbols: self.lease.derived.as_ref(),
            source: self.lease.views.source,
            paths: self.lease.views.paths,
            arenas: self.lease.views.arenas,
        };
        match datum(value, &texts) {
            Datum::Text(text) => {
                target.push_str(text.0);
                TextFound::Text
            }
            _ => TextFound::NotText,
        }
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
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
    fn population(&mut self, input: InputNumber) -> Result<RowCount, Fault> {
        let population = self
            .borrowed(input)
            .map(|rows| rows.count())
            .ok_or(OperatingError::NotRun);
        self.hold(population)
    }
    fn population_row(
        &mut self,
        input: InputNumber,
        at: RowIndex,
        row: &mut Vec<Value>,
    ) -> RowRead {
        let Some(rows) = self.borrowed(input) else {
            return RowRead::Missing;
        };
        match rows.read(at, &mut self.left_buffer) {
            Some(values) => {
                row.clear();
                row.extend_from_slice(values);
                RowRead::Read
            }
            None => RowRead::Missing,
        }
    }
    fn groups(&mut self) -> Result<Count, Fault> {
        let groups = self
            .resources
            .groups()
            .map(|groups| Count(groups.len()))
            .ok_or(OperatingError::NotRun);
        self.hold(groups)
    }
    fn group(&self, at: VertexPlace) -> Option<GroupSummary> {
        self.resources
            .groups()?
            .get(at.0)
            .map(|group| GroupSummary {
                first: group.first,
                counted: Count(group.counted.0),
                digest: Integer(group.digest.0),
            })
    }
    fn render(&mut self, nodes: &[NodeId]) -> Result<(), Fault> {
        let Operation::RenderSyntax { context, .. } = self.operation else {
            return self.hold(Err(OperatingError::WrongOperation));
        };
        let requests: Vec<ryo_language::domain::semantics::types::RenderRequest> = nodes
            .iter()
            .map(
                |node| ryo_language::domain::semantics::types::RenderRequest {
                    node: *node,
                    context: *context,
                },
            )
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
    fn rendered(&self, at: RowIndex, text: &mut String) -> RenderKind {
        use ryo_language::domain::semantics::types::RenderOutcome;
        match self.rendered.get(at.0) {
            Some(RenderOutcome::Known(known)) => {
                text.push_str(&known.0);
                RenderKind::Known
            }
            Some(RenderOutcome::NotApplicable) | None => RenderKind::NotApplicable,
            Some(RenderOutcome::Unsupported(_)) => RenderKind::Unsupported,
        }
    }
    fn right_columns(&self) -> RightColumns {
        if self.join.native().is_some() {
            RightColumns::Physical
        } else {
            RightColumns::Logical
        }
    }
    fn join(&mut self) -> &mut dyn JoinDriver {
        self
    }
    fn set_rows(&mut self) -> &mut dyn SetRowDriver {
        self
    }
    fn text_rows(&mut self) -> &mut dyn TextRowDriver {
        self
    }
    fn ordered_match(&mut self) -> Result<&mut dyn OrderedMatchDriver, Fault> {
        let checked = match self.inputs {
            PreparedInputs::Views(inputs) => inputs
                .first()
                .ok_or(OperatingError::MissingInput { position: 1 })
                .and_then(|_| {
                    inputs
                        .get(1)
                        .ok_or(OperatingError::MissingInput { position: 2 })
                })
                .and_then(|_| self.resources.ordered())
                .and_then(|ordered| ordered.ok_or(OperatingError::WrongBinding)),
            PreparedInputs::Row { .. } | PreparedInputs::RequestSource(_) => {
                Err(OperatingError::WrongOperation)
            }
        };
        self.graph.ordered = Some(self.hold(checked)?);
        Ok(self)
    }
    fn cycles(&mut self) -> Result<&dyn CycleDriver, Fault> {
        let graph = self
            .resources
            .cycles()
            .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
        let graph = self.hold(graph)?;
        self.graph.cycles = Some(super::graph::Adjacency(graph));
        self.graph
            .cycles
            .as_ref()
            .map(|graph| graph as &dyn CycleDriver)
            .ok_or(Fault::WrongBinding)
    }
    fn closure(&mut self) -> &mut dyn ClosureDriver {
        self
    }
    fn axis(&mut self) -> &mut dyn AxisDriver {
        self
    }
    fn dfs(&mut self) -> &mut dyn DfsDriver {
        self
    }
}

impl SetRowDriver for LeaseDriver<'_, '_> {
    fn right_empty(&mut self) -> Result<RightPopulation, SetRowsError> {
        if self.interrupted() {
            return Err(SetRowsError::Interrupted);
        }
        let empty = self
            .resources
            .index()
            .map(|index| RightPopulation::from(index.rows.count().0 == 0))
            .ok_or(OperatingError::WrongBinding);
        self.hold(empty).map_err(|_| SetRowsError::Interrupted)
    }
    fn contains_right(&mut self, key: &[Value]) -> Result<RightMembership, SetRowsError> {
        if self.interrupted() {
            return Err(SetRowsError::Interrupted);
        }
        let contained = self
            .resources
            .index()
            .map(|index| index.contains(key))
            .ok_or(OperatingError::WrongBinding);
        self.hold(contained).map_err(|_| SetRowsError::Interrupted)
    }
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, SetRowsError> {
        InvocationDriver::first_time(self, row).map_err(|_| SetRowsError::Interrupted)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), SetRowsError> {
        InvocationDriver::emit(self, row).map_err(|_| SetRowsError::Interrupted)
    }
}

impl TextRowDriver for LeaseDriver<'_, '_> {
    fn intern(&mut self, text: CellText<'_>) -> Value {
        Value::Symbol(query_intern_derived(
            &mut self.lease.derived,
            ByteSlice(self.lease.views.source.0),
            NameText(text.0),
        ))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        InvocationDriver::emit(self, row).map_err(|_| ContractError::Interrupted)
    }
}

impl OrderedMatchDriver for LeaseDriver<'_, '_> {
    fn left_len(&self) -> RowCount {
        RowCount(self.graph.ordered.map_or(0, |ordered| ordered.left.len()))
    }
    fn right_len(&self) -> RowCount {
        RowCount(self.graph.ordered.map_or(0, |ordered| ordered.right.len()))
    }
    fn compare_keys(&self, left: RowIndex, right: RowIndex) -> std::cmp::Ordering {
        self.graph
            .ordered
            .map_or(std::cmp::Ordering::Equal, |ordered| {
                ordered.left[left.0].keys.cmp(&ordered.right[right.0].keys)
            })
    }
    fn right_order_before_or_equal(
        &self,
        left: RowIndex,
        right: RowIndex,
    ) -> ryo_operators::domain::ordered_match::types::KeyOrder {
        use ryo_operators::domain::ordered_match::types::KeyOrder;
        match self.graph.ordered {
            Some(ordered) if ordered.right[right.0].order <= ordered.left[left.0].order => {
                KeyOrder::BeforeOrEqual
            }
            Some(_) | None => KeyOrder::After,
        }
    }
    fn visit_pair(
        &mut self,
        left: RowIndex,
        right: RowIndex,
        project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let (Some(ordered), PreparedInputs::Views(inputs)) = (self.graph.ordered, self.inputs)
        else {
            return self.hold(Err(OperatingError::WrongBinding));
        };
        let (Some(left_rows), Some(right_rows)) = (inputs.first(), inputs.get(1)) else {
            return self.hold(Err(OperatingError::ShapeMismatch));
        };
        let (left_buffer, right_buffer) = (&mut self.left_buffer, &mut self.right_buffer);
        let pair = left_rows
            .read(ordered.left[left.0].row, left_buffer)
            .ok_or(OperatingError::ShapeMismatch)
            .and_then(|left| {
                right_rows
                    .read(ordered.right[right.0].row, right_buffer)
                    .ok_or(OperatingError::ShapeMismatch)
                    .map(|right| project(left, right))
            });
        self.hold(pair)?
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        InvocationDriver::emit(self, row).map_err(|_| ContractError::Interrupted)
    }
}
