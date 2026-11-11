//! The source, endpoints and rows that the graphs cases and their nested parts' cases share.
use crate::application::file_dag::derived::owner::graphs::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use std::sync::Arc;

pub(super) const SOURCE: Input = Input::Relation(Relation::Function);

pub(super) const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];

pub(super) fn rows(columns: &[FactColumn]) -> Rows<'_> {
    Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns,
    })
}
