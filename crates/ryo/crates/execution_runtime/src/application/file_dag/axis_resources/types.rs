//! The types of the `axis_resources` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::view::service::AxisResource;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::membership::types::MembershipView;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    operator::contract::resources::AxisResources,
    table_cells::types::{TreeForm, TreeRows},
    table_model::service::check_columns,
    tree_walk::service::entity_of,
};
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

#[derive(Clone, Copy)]
pub(crate) enum AxisMode<'f> {
    Empty,
    Native {
        tree: TreeRows<'f>,
        among: Option<TreeRows<'f>>,
    },
    Indexed {
        from: [ColumnIndex; 1],
        to: ColumnIndex,
    },
}
