//! The rows, source and endpoints that the graph lifetime's case groups share.
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::*;
use fixtures::graph::GraphFixture;
use ryo_execution_domain::domain::{
    stored_graph::types::{GraphWalk, Orientation},
    table_model::types::Rows,
};
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::hash::BuildHasher;

pub(super) fn number(value: Integer) -> Value {
    Value::Number(value)
}

pub(super) fn rows(columns: &[FactColumn]) -> Rows<'_> {
    Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns,
    })
}

pub(super) const SOURCE: Input = Input::Relation(Relation::Function);

pub(super) const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];
