//! The test support of the `evaluation` concept: an engine driver held in memory, which lends scripted axis,
//! closure, ordered depth-first and population resources and keeps every row it is given.
use crate::domain::axis::types::{AxisAdmission, AxisCandidates, AxisDriver, AxisEvaluation};
use crate::domain::closure::types::{ClosureDriver, ClosureEvaluation};
use crate::domain::contract::types::{FirstOccurrence, GroupSummary, Occurrences, PortableOperatorFault, PredicateOutcome, RenderKind, RightColumns, RowDriver, RowRead, TextFound, TextScratch, VertexPlace};
use crate::domain::closure::types::PartitionPlace;
use crate::domain::cycles::types::{CycleDriver, CyclesEvaluation};
use crate::domain::equality_join::types::{JoinDriver, JoinEvaluation};
use crate::domain::ordered_dfs::types::{DfsDriver, DfsEvaluation, OrderedEdge};
use crate::domain::ordered_match::types::{OrderedMatchDriver, OrderedMatchEvaluation};
use crate::domain::set_rows::types::{SetRowDriver, SetRowsEvaluation};
use crate::domain::source_matches::types::{TextRowDriver, TextRowsEvaluation};
use ryo_relations::domain::relations::types::{Count, Integer, NodeId, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, InputNumber};
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};
use std::ops::ControlFlow;

/// A number cell, the one kind of value the scripted resources hold.
pub(crate) fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}

/// The rows emitted, written out: the cells of a row by spaces, the rows by `|`, a cell that is no number
/// as `?`.
pub(crate) fn written(rows: &[Vec<Value>]) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Value::Number(Integer(value)) => value.to_string(),
                    _ => "?".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// A scripted engine: what each resource answers, and the rows the evaluation emitted.
#[derive(Debug, Default)]
pub(crate) struct FakeDriver {
    /// Every row emitted, in order.
    pub emitted: Vec<Vec<Value>>,
    /// The candidates an axis walk keeps, in walk order.
    pub candidates: Vec<Value>,
    /// The rows of the candidate elements; none means the candidates are entities.
    pub elements: Vec<Vec<Value>>,
    /// Which candidates the axis admits; a candidate past the list is admitted.
    pub admitted: Vec<AxisAdmission>,
    /// Each closure partition: its key and its sources.
    pub partitions: Vec<(Vec<Value>, Vec<Value>)>,
    /// What each closure source reaches, in walk order.
    pub reaches: Vec<(Value, Vec<Value>)>,
    /// What the last closure walk reached.
    pub reached: Vec<Value>,
    /// The root rows of an ordered depth-first graph, in sorted order.
    pub roots: Vec<Vec<Value>>,
    /// The partition keys and canonical root of the graph's root rows.
    pub root_columns: Vec<ColumnIndex>,
    /// The edge rows of the graph.
    pub edges: Vec<Vec<Value>>,
    /// The edges each visit reaches, one list for each visit begun, in order.
    pub visits: Vec<Vec<OrderedEdge>>,
    /// How many visits were begun.
    pub begun: usize,
    /// The edges the current visit has still to reach, last first.
    pub visit: Vec<OrderedEdge>,
    /// The rows of the request's own population.
    pub population: Vec<Vec<Value>>,
}

impl RowDriver for FakeDriver {
    fn emit(&mut self, row: &[Value]) -> Result<(), PortableOperatorFault> {
        self.emitted.push(row.to_vec());
        Ok(())
    }
    fn holds(&mut self, _row: &[Value]) -> Result<PredicateOutcome, PortableOperatorFault> {
        Ok(PredicateOutcome::Holds)
    }
    fn first_time(&mut self, _row: &[Value]) -> Result<FirstOccurrence, PortableOperatorFault> {
        Ok(FirstOccurrence::First)
    }
    fn found(&mut self, _input: InputNumber, _rows: Count) {}
    fn text(&mut self, _value: Value, _target: &mut TextScratch) -> TextFound {
        TextFound::NotText
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        number(i64::try_from(text.0.len()).unwrap_or(i64::MAX))
    }
    fn population(&mut self, _input: InputNumber) -> Result<RowCount, PortableOperatorFault> {
        Ok(RowCount(self.population.len()))
    }
    fn population_row(
        &mut self,
        _input: InputNumber,
        at: RowIndex,
        row: &mut Vec<Value>,
    ) -> RowRead {
        copy(&self.population, at.0, row)
    }
    fn groups(&mut self) -> Result<Count, PortableOperatorFault> {
        Err(PortableOperatorFault::NotRun)
    }
    fn group(&self, _at: VertexPlace) -> Option<GroupSummary> {
        None
    }
    fn render(&mut self, _nodes: &[NodeId]) -> Result<(), PortableOperatorFault> {
        Err(PortableOperatorFault::NotRun)
    }
    fn rendered(&self, _at: RowIndex, _text: &mut TextScratch) -> RenderKind {
        RenderKind::NotApplicable
    }
    fn right_columns(&self) -> RightColumns {
        RightColumns::Logical
    }
}

impl JoinEvaluation for FakeDriver {
    fn join(&mut self) -> &mut dyn JoinDriver {
        panic!("the scripted engine lends no join candidates")
    }
}

impl SetRowsEvaluation for FakeDriver {
    fn set_rows(&mut self) -> &mut dyn SetRowDriver {
        panic!("the scripted engine lends no set rows")
    }
}

impl TextRowsEvaluation for FakeDriver {
    fn text_rows(&mut self) -> &mut dyn TextRowDriver {
        panic!("the scripted engine lends no occurrence rows")
    }
}

impl OrderedMatchEvaluation for FakeDriver {
    fn ordered_match(&mut self) -> Result<&mut dyn OrderedMatchDriver, PortableOperatorFault> {
        Err(PortableOperatorFault::NotRun)
    }
}

impl CyclesEvaluation for FakeDriver {
    fn cycles(&mut self) -> Result<&dyn CycleDriver, PortableOperatorFault> {
        Err(PortableOperatorFault::NotRun)
    }
}

impl ClosureEvaluation for FakeDriver {
    fn closure(&mut self) -> &mut dyn ClosureDriver {
        self
    }
}

impl AxisEvaluation for FakeDriver {
    fn axis(&mut self) -> &mut dyn AxisDriver {
        self
    }
}

impl DfsEvaluation for FakeDriver {
    fn dfs(&mut self) -> &mut dyn DfsDriver {
        self
    }
}

impl AxisDriver for FakeDriver {
    fn walk(
        &mut self,
        _direction: Direction,
        _row: &[Value],
        _id: ColumnIndex,
    ) -> Result<AxisCandidates, PortableOperatorFault> {
        let count = Count(self.candidates.len());
        Ok(if self.elements.is_empty() {
            AxisCandidates::Entities(count)
        } else {
            AxisCandidates::Elements(count)
        })
    }
    fn admits(&mut self, at: VertexPlace) -> Result<AxisAdmission, PortableOperatorFault> {
        Ok(self
            .admitted
            .get(at.0)
            .copied()
            .unwrap_or(AxisAdmission::In))
    }
    fn candidate(&self, at: VertexPlace) -> Option<Value> {
        self.candidates.get(at.0).copied()
    }
    fn element_cell(&self, at: VertexPlace, column: ColumnIndex) -> Option<Value> {
        let column = usize::try_from(column.0).ok()?;
        self.elements.get(at.0)?.get(column).copied()
    }
}

impl ClosureDriver for FakeDriver {
    fn partitions(&mut self) -> Result<Count, PortableOperatorFault> {
        Ok(Count(self.partitions.len()))
    }
    fn partition_key(
        &mut self,
        partition: PartitionPlace,
        key: &mut Vec<Value>,
    ) -> Result<(), PortableOperatorFault> {
        let (found, _) = self
            .partitions
            .get(partition.0)
            .ok_or(PortableOperatorFault::ShapeMismatch)?;
        key.clear();
        key.extend_from_slice(found);
        Ok(())
    }
    fn sources(&self, partition: PartitionPlace) -> Count {
        Count(
            self.partitions
                .get(partition.0)
                .map_or(0, |(_, sources)| sources.len()),
        )
    }
    fn source(
        &mut self,
        partition: PartitionPlace,
        at: VertexPlace,
    ) -> Result<Option<Value>, PortableOperatorFault> {
        Ok(self
            .partitions
            .get(partition.0)
            .and_then(|(_, sources)| sources.get(at.0).copied()))
    }
    fn reach(
        &mut self,
        _partition: PartitionPlace,
        source: Value,
    ) -> Result<Count, PortableOperatorFault> {
        self.reached = self
            .reaches
            .iter()
            .find(|(start, _)| *start == source)
            .map(|(_, reached)| reached.clone())
            .unwrap_or_default();
        Ok(Count(self.reached.len()))
    }
    fn reached(&self, at: VertexPlace) -> Option<Value> {
        self.reached.get(at.0).copied()
    }
}

impl DfsDriver for FakeDriver {
    fn roots(&mut self) -> Result<RowCount, PortableOperatorFault> {
        Ok(RowCount(self.roots.len()))
    }
    fn root(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        copy(&self.roots, at.0, row)
    }
    fn root_columns(&self, columns: &mut Vec<ColumnIndex>) {
        columns.clone_from(&self.root_columns);
    }
    fn begin(&mut self, _partition: &[Value], _root: Value) -> Result<(), PortableOperatorFault> {
        self.visit = self.visits.get(self.begun).cloned().unwrap_or_default();
        self.visit.reverse();
        self.begun += 1;
        Ok(())
    }
    fn next(&mut self, _partition: &[Value]) -> Result<Option<OrderedEdge>, PortableOperatorFault> {
        Ok(self.visit.pop())
    }
    fn edge(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        copy(&self.edges, at.0, row)
    }
}

impl Occurrences for FakeDriver {
    fn matches(
        &self,
        _regex: &ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        _text: ryo_matching::inbound::use_cases::matching::types::SearchText<'_>,
        _each: &mut dyn FnMut(ryo_matching::inbound::use_cases::matching::types::TextSpan) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn captures(
        &self,
        _regex: &ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        _text: ryo_matching::inbound::use_cases::matching::types::SearchText<'_>,
        _group: ryo_matching::inbound::use_cases::matching::types::GroupName<'_>,
        _each: &mut dyn FnMut(
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
        ) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, PortableOperatorFault> {
        Ok(ControlFlow::Continue(()))
    }
}

/// Copies the row at `at` of `rows` into `row`.
fn copy(rows: &[Vec<Value>], at: usize, row: &mut Vec<Value>) -> RowRead {
    match rows.get(at) {
        Some(found) => {
            row.clone_from(found);
            RowRead::Read
        }
        None => RowRead::Missing,
    }
}
