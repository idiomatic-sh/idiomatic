//! The `LeaseDriver` value: the engine's side of one evaluation behind the operators' driver ports. It holds the
//! shared row delivery and the strategy values (join candidates, graph walks and lookups, the axis walk), and
//! passes the row delivery into each strategy a port reaches; every kind's row meaning is the operators'.

use super::super::axis_walk::value::AxisWalk;
use super::super::graph_lookups::value::GraphLookups;
use super::super::graph_walks::value::GraphWalks;
use super::super::join_candidates::value::JoinCandidates;
use super::super::row_delivery::value::RowDelivery;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::{PrimitiveLease, RowSink};
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_operators::domain::axis::types::{
    AxisAdmission, AxisCandidates, AxisDriver, AxisEvaluation,
};
use ryo_operators::domain::closure::types::PartitionPlace;
use ryo_operators::domain::closure::types::{ClosureDriver, ClosureEvaluation};
use ryo_operators::domain::contract::errors::ContractError;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::{
    FirstOccurrence, GroupSummary, PredicateOutcome, RenderKind, RightColumns, RowDriver, RowRead,
    TextFound, TextScratch, VertexPlace,
};
use ryo_operators::domain::cycles::types::{CycleDriver, CyclesEvaluation};
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::domain::equality_join::types::{Advance, JoinDriver, JoinEvaluation};
use ryo_operators::domain::ordered_dfs::types::{DfsDriver, DfsEvaluation, OrderedEdge};
use ryo_operators::domain::ordered_match::types::{
    KeyOrder, OrderedMatchDriver, OrderedMatchEvaluation,
};
use ryo_operators::domain::set_rows::errors::SetRowsError;
use ryo_operators::domain::set_rows::types::{
    RightMembership, RightPopulation, SetRowDriver, SetRowsEvaluation,
};
use ryo_operators::domain::source_matches::types::{TextRowDriver, TextRowsEvaluation};
use ryo_relations::domain::relations::types::{Count, NodeId, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, InputNumber};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The row delivery of one evaluation and the strategies its ports reach.
pub(crate) struct LeaseDriver<'a, 'f> {
    rows: RowDelivery<'a, 'f>,
    join: JoinCandidates<'a, 'f>,
    walks: GraphWalks<'f>,
    lookups: GraphLookups<'f>,
    axis: AxisWalk<'f>,
}

impl<'a, 'f> LeaseDriver<'a, 'f> {
    /// The driver of one evaluation of `operation` over `inputs`; it borrows the workspace's positions buffer
    /// until [`LeaseDriver::finish`] hands it back.
    pub(crate) fn new(
        lease: &'a mut PrimitiveLease<'f>,
        sink: &'a mut dyn RowSink,
        operation: &'a Operation,
        inputs: PreparedInputs<'a>,
    ) -> Self {
        let resources = lease.views.resources;
        let positions = std::mem::take(&mut lease.workspace.positions);
        let left = match inputs {
            PreparedInputs::Row { row, .. } => row,
            PreparedInputs::Views(_) | PreparedInputs::RequestSource(_) => &[],
        };
        Self {
            rows: RowDelivery::new(lease, sink, operation, inputs),
            join: JoinCandidates::new(resources, operation, left, positions),
            walks: GraphWalks::default(),
            lookups: GraphLookups::default(),
            axis: AxisWalk::default(),
        }
    }

    /// The engine error kept, taken out of the latch.
    pub(crate) fn take_failure(&mut self) -> Option<OperatingError> {
        self.rows.take_failure()
    }

    /// The answers of the last render.
    pub(crate) fn rendered_answers(&self) -> &[RenderOutcome] {
        self.rows.rendered_answers()
    }

    /// Ends the evaluation: the positions buffer goes back to the workspace and the lease to the caller.
    pub(crate) fn finish(self) -> &'a mut PrimitiveLease<'f> {
        let lease = self.rows.into_lease();
        lease.workspace.positions = self.join.give_back();
        lease
    }
}

impl RowDriver for LeaseDriver<'_, '_> {
    fn emit(&mut self, row: &[Value]) -> Result<(), Fault> {
        self.rows.emit(row)
    }
    fn holds(&mut self, row: &[Value]) -> Result<PredicateOutcome, Fault> {
        self.rows.holds(row)
    }
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, Fault> {
        self.rows.first_time(row)
    }
    fn found(&mut self, input: InputNumber, rows: Count) {
        self.rows.found(input, rows);
    }
    fn text(&mut self, value: Value, target: &mut TextScratch) -> TextFound {
        self.rows.text(value, target)
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        self.rows.mint(text)
    }
    fn population(&mut self, input: InputNumber) -> Result<RowCount, Fault> {
        self.rows.population(input)
    }
    fn population_row(
        &mut self,
        input: InputNumber,
        at: RowIndex,
        row: &mut Vec<Value>,
    ) -> RowRead {
        self.rows.population_row(input, at, row)
    }
    fn groups(&mut self) -> Result<Count, Fault> {
        self.rows.groups()
    }
    fn group(&self, at: VertexPlace) -> Option<GroupSummary> {
        self.rows.group(at)
    }
    fn render(&mut self, nodes: &[NodeId]) -> Result<(), Fault> {
        self.rows.render(nodes)
    }
    fn rendered(&self, at: RowIndex, text: &mut TextScratch) -> RenderKind {
        self.rows.rendered(at, text)
    }
    fn right_columns(&self) -> RightColumns {
        self.join.right_columns()
    }
}

impl TextRowDriver for LeaseDriver<'_, '_> {
    fn intern(&mut self, text: CellText<'_>) -> Value {
        self.rows.intern(text)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        self.rows.emit(row).map_err(|_| ContractError::Interrupted)
    }
}

impl TextRowsEvaluation for LeaseDriver<'_, '_> {
    fn text_rows(&mut self) -> &mut dyn TextRowDriver {
        self
    }
}

impl JoinDriver for LeaseDriver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<Advance, EqualityJoinError> {
        self.join.next(&mut self.rows, key)
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        self.join.right_cell(&self.rows, column)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), EqualityJoinError> {
        self.rows
            .emit(row)
            .map_err(|_| EqualityJoinError::Interrupted)
    }
}

impl JoinEvaluation for LeaseDriver<'_, '_> {
    fn join(&mut self) -> &mut dyn JoinDriver {
        self
    }
}

impl SetRowsEvaluation for LeaseDriver<'_, '_> {
    fn set_rows(&mut self) -> &mut dyn SetRowDriver {
        self
    }
}

impl SetRowDriver for LeaseDriver<'_, '_> {
    fn right_empty(&mut self) -> Result<RightPopulation, SetRowsError> {
        self.join.right_empty(&mut self.rows)
    }
    fn contains_right(&mut self, key: &[Value]) -> Result<RightMembership, SetRowsError> {
        self.join.contains_right(&mut self.rows, key)
    }
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, SetRowsError> {
        self.rows
            .first_time(row)
            .map_err(|_| SetRowsError::Interrupted)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), SetRowsError> {
        self.rows.emit(row).map_err(|_| SetRowsError::Interrupted)
    }
}

impl ClosureDriver for LeaseDriver<'_, '_> {
    fn partitions(&mut self) -> Result<Count, Fault> {
        self.walks.partitions(&mut self.rows)
    }
    fn partition_key(
        &mut self,
        partition: PartitionPlace,
        key: &mut Vec<Value>,
    ) -> Result<(), Fault> {
        self.walks.partition_key(&mut self.rows, partition, key)
    }
    fn sources(&self, partition: PartitionPlace) -> Count {
        self.walks.sources(partition)
    }
    fn source(
        &mut self,
        partition: PartitionPlace,
        at: VertexPlace,
    ) -> Result<Option<Value>, Fault> {
        self.walks.source(&mut self.rows, partition, at)
    }
    fn reach(&mut self, partition: PartitionPlace, source: Value) -> Result<Count, Fault> {
        self.walks.reach(&mut self.rows, partition, source)
    }
    fn reached(&self, at: VertexPlace) -> Option<Value> {
        GraphWalks::reached(&self.rows, at)
    }
}

impl DfsDriver for LeaseDriver<'_, '_> {
    fn roots(&mut self) -> Result<RowCount, Fault> {
        self.walks.roots(&mut self.rows)
    }
    fn root(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        self.walks.root(at, row)
    }
    fn root_columns(&self, columns: &mut Vec<ColumnIndex>) {
        self.walks.root_columns(columns);
    }
    fn begin(&mut self, partition: &[Value], root: Value) -> Result<(), Fault> {
        self.walks.begin(&mut self.rows, partition, root)
    }
    fn next(&mut self, partition: &[Value]) -> Result<Option<OrderedEdge>, Fault> {
        self.walks.next(&mut self.rows, partition)
    }
    fn edge(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        self.walks.edge(at, row)
    }
}

impl ClosureEvaluation for LeaseDriver<'_, '_> {
    fn closure(&mut self) -> &mut dyn ClosureDriver {
        self
    }
}

impl DfsEvaluation for LeaseDriver<'_, '_> {
    fn dfs(&mut self) -> &mut dyn DfsDriver {
        self
    }
}

impl CyclesEvaluation for LeaseDriver<'_, '_> {
    fn cycles(&mut self) -> Result<&dyn CycleDriver, Fault> {
        self.lookups.cycles(&mut self.rows)
    }
}

impl OrderedMatchEvaluation for LeaseDriver<'_, '_> {
    fn ordered_match(&mut self) -> Result<&mut dyn OrderedMatchDriver, Fault> {
        self.lookups.ordered_match(&mut self.rows)?;
        Ok(self)
    }
}

impl OrderedMatchDriver for LeaseDriver<'_, '_> {
    fn left_len(&self) -> RowCount {
        self.lookups.left_len()
    }
    fn right_len(&self) -> RowCount {
        self.lookups.right_len()
    }
    fn compare_keys(&self, left: RowIndex, right: RowIndex) -> std::cmp::Ordering {
        self.lookups.compare_keys(left, right)
    }
    fn right_order_before_or_equal(&self, left: RowIndex, right: RowIndex) -> KeyOrder {
        self.lookups.right_order_before_or_equal(left, right)
    }
    fn visit_pair(
        &mut self,
        left: RowIndex,
        right: RowIndex,
        project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        self.lookups
            .visit_pair(&mut self.rows, left, right, project)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        self.rows.emit(row).map_err(|_| ContractError::Interrupted)
    }
}

impl AxisDriver for LeaseDriver<'_, '_> {
    fn walk(
        &mut self,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, Fault> {
        self.axis.walk(&mut self.rows, direction, row, id)
    }
    fn admits(&mut self, at: VertexPlace) -> Result<AxisAdmission, Fault> {
        self.axis.admits(&mut self.rows, at)
    }
    fn candidate(&self, at: VertexPlace) -> Option<Value> {
        self.axis.candidate(&self.rows, at)
    }
    fn element_cell(&self, at: VertexPlace, column: ColumnIndex) -> Option<Value> {
        self.axis.element_cell(&self.rows, at, column)
    }
}

impl AxisEvaluation for LeaseDriver<'_, '_> {
    fn axis(&mut self) -> &mut dyn AxisDriver {
        self
    }
}
