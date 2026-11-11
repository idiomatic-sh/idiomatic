//! The types of the `driver` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::failure::types::FailureLatch;
use super::super::primitive::types::PreparedInputs;
use super::super::types::{PrimitiveLease, PrimitiveResources, RowSink};
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
use ryo_operators::domain::ordered_match::types::{OrderedMatchDriver, OrderedMatchEvaluation};
use ryo_operators::domain::set_rows::errors::SetRowsError;
use ryo_operators::domain::set_rows::types::{
    RightMembership, RightPopulation, SetRowDriver, SetRowsEvaluation,
};
use ryo_operators::domain::source_matches::types::{TextRowDriver, TextRowsEvaluation};
use ryo_relations::domain::relations::types::{Count, NodeId, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The lease, sink and inputs of one evaluation, with the cursor state of the engine's strategies.
pub(crate) struct LeaseDriver<'a, 'f> {
    pub(crate) lease: &'a mut PrimitiveLease<'f>,
    pub(crate) sink: &'a mut dyn RowSink,
    pub(crate) operation: &'a Operation,
    pub(crate) inputs: PreparedInputs<'a>,
    pub(crate) resources: &'f dyn PrimitiveResources,
    pub(crate) failure: FailureLatch,
    pub(crate) join: super::super::views::types::JoinCursor<'f>,
    pub(crate) axis: super::super::views::types::AxisCursor<'f>,
    pub(crate) graph: super::super::views::types::GraphCursor<'f>,
    pub(crate) rendered: Vec<ryo_language::domain::semantics::types::RenderOutcome>,
    pub(crate) left_buffer: Vec<Value>,
    pub(crate) right_buffer: Vec<Value>,
}

impl RowDriver for LeaseDriver<'_, '_> {
    fn emit(&mut self, row: &[Value]) -> Result<(), Fault> {
        super::super::driver::service::emit(self, row)
    }
    fn holds(&mut self, row: &[Value]) -> Result<PredicateOutcome, Fault> {
        super::super::driver::service::holds(self, row)
    }
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, Fault> {
        super::super::driver::service::first_time(self, row)
    }
    fn found(&mut self, input: InputNumber, rows: Count) {
        super::super::driver::service::found(self, input, rows)
    }
    fn text(&mut self, value: Value, target: &mut TextScratch) -> TextFound {
        super::super::driver::service::text(self, value, target)
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        super::super::driver::service::mint(self, text)
    }
    fn population(&mut self, input: InputNumber) -> Result<RowCount, Fault> {
        super::super::driver::service::population(self, input)
    }
    fn population_row(
        &mut self,
        input: InputNumber,
        at: RowIndex,
        row: &mut Vec<Value>,
    ) -> RowRead {
        super::super::driver::service::population_row(self, input, at, row)
    }
    fn groups(&mut self) -> Result<Count, Fault> {
        super::super::driver::service::groups(self)
    }
    fn group(&self, at: VertexPlace) -> Option<GroupSummary> {
        super::super::driver::service::group(self, at)
    }
    fn render(&mut self, nodes: &[NodeId]) -> Result<(), Fault> {
        super::super::driver::service::render(self, nodes)
    }
    fn rendered(&self, at: RowIndex, text: &mut TextScratch) -> RenderKind {
        super::super::driver::service::rendered(self, at, text)
    }
    fn right_columns(&self) -> RightColumns {
        super::super::driver::service::right_columns(self)
    }
}

impl TextRowDriver for LeaseDriver<'_, '_> {
    fn intern(&mut self, text: CellText<'_>) -> Value {
        super::super::driver::service::intern(self, text)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        RowDriver::emit(self, row).map_err(|_| ContractError::Interrupted)
    }
}

impl TextRowsEvaluation for LeaseDriver<'_, '_> {
    fn text_rows(&mut self) -> &mut dyn TextRowDriver {
        self
    }
}

impl JoinDriver for LeaseDriver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<Advance, EqualityJoinError> {
        super::super::candidates::service::next(self, key)
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        super::super::candidates::service::right_cell(self, column)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), EqualityJoinError> {
        RowDriver::emit(self, row).map_err(|_| EqualityJoinError::Interrupted)
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
        super::super::candidates::service::right_empty(self)
    }
    fn contains_right(&mut self, key: &[Value]) -> Result<RightMembership, SetRowsError> {
        super::super::candidates::service::contains_right(self, key)
    }
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, SetRowsError> {
        RowDriver::first_time(self, row).map_err(|_| SetRowsError::Interrupted)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), SetRowsError> {
        RowDriver::emit(self, row).map_err(|_| SetRowsError::Interrupted)
    }
}

impl ClosureDriver for LeaseDriver<'_, '_> {
    fn partitions(&mut self) -> Result<Count, Fault> {
        super::super::graph::service::partitions(self)
    }
    fn partition_key(
        &mut self,
        partition: PartitionPlace,
        key: &mut Vec<Value>,
    ) -> Result<(), Fault> {
        super::super::graph::service::partition_key(self, partition, key)
    }
    fn sources(&self, partition: PartitionPlace) -> Count {
        super::super::graph::service::sources(self, partition)
    }
    fn source(
        &mut self,
        partition: PartitionPlace,
        at: VertexPlace,
    ) -> Result<Option<Value>, Fault> {
        super::super::graph::service::source(self, partition, at)
    }
    fn reach(&mut self, partition: PartitionPlace, source: Value) -> Result<Count, Fault> {
        super::super::graph::service::reach(self, partition, source)
    }
    fn reached(&self, at: VertexPlace) -> Option<Value> {
        super::super::graph::service::reached(self, at)
    }
}

impl DfsDriver for LeaseDriver<'_, '_> {
    fn roots(&mut self) -> Result<RowCount, Fault> {
        super::super::graph::service::roots(self)
    }
    fn root(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        super::super::graph::service::root(self, at, row)
    }
    fn root_columns(&self, columns: &mut Vec<ColumnIndex>) {
        super::super::graph::service::root_columns(self, columns)
    }
    fn begin(&mut self, partition: &[Value], root: Value) -> Result<(), Fault> {
        super::super::graph::service::begin(self, partition, root)
    }
    fn next(&mut self, partition: &[Value]) -> Result<Option<OrderedEdge>, Fault> {
        super::super::graph::service::next(self, partition)
    }
    fn edge(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        super::super::graph::service::edge(self, at, row)
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
        super::super::graph::service::cycles(self)
    }
}

impl OrderedMatchEvaluation for LeaseDriver<'_, '_> {
    fn ordered_match(&mut self) -> Result<&mut dyn OrderedMatchDriver, Fault> {
        super::super::graph::service::ordered_match(self)
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
        super::super::graph::service::compare_keys(self, left, right)
    }
    fn right_order_before_or_equal(
        &self,
        left: RowIndex,
        right: RowIndex,
    ) -> ryo_operators::domain::ordered_match::types::KeyOrder {
        super::super::graph::service::right_order_before_or_equal(self, left, right)
    }
    fn visit_pair(
        &mut self,
        left: RowIndex,
        right: RowIndex,
        project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        super::super::graph::service::visit_pair(self, left, right, project)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        RowDriver::emit(self, row).map_err(|_| ContractError::Interrupted)
    }
}

impl AxisDriver for LeaseDriver<'_, '_> {
    fn walk(
        &mut self,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, Fault> {
        super::super::walk::service::walk(self, direction, row, id)
    }
    fn admits(&mut self, at: VertexPlace) -> Result<AxisAdmission, Fault> {
        super::super::walk::service::admits(self, at)
    }
    fn candidate(&self, at: VertexPlace) -> Option<Value> {
        super::super::walk::service::candidate(self, at)
    }
    fn element_cell(&self, at: VertexPlace, column: ColumnIndex) -> Option<Value> {
        super::super::walk::service::element_cell(self, at, column)
    }
}

impl AxisEvaluation for LeaseDriver<'_, '_> {
    fn axis(&mut self) -> &mut dyn AxisDriver {
        self
    }
}
