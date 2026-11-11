//! The support of the `index` cases.
use super::super::super::types::ByteLimit;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::derived::owner::fixtures::graph::GraphFixture;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(super) fn new(wanted: &[Relation], partials: &[Partial], limit: ByteLimit) -> GraphFixture {
    GraphFixture::new(wanted, partials, limit.0)
}

pub(super) fn acquire(
    owner: &GraphFixture,
    relation: Relation,
    rows: ColumnsView<'_>,
    on: &[ColumnIndex],
) -> Result<AcquiredView, ryo_execution_domain::domain::operating::errors::OperatingError> {
    owner.index(relation, &Rows::Stored(rows), on)
}

pub(super) fn equality(view: &AcquiredView) -> &KeyIndex {
    let DerivedView::Equality(index) = view.view().expect("valid equality payload") else {
        panic!("equality payload")
    };
    index
}
