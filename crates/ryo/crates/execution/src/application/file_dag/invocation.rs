//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
use super::derived::owner::acquisition::admission::service::AcquiredView;
mod binding;
mod constants;
pub(in crate::application::file_dag) mod native;
use native::service::NativeJoinLease;
mod groups;
pub(in crate::application::file_dag) mod index;
mod prepared_use;
pub(in crate::application::file_dag) mod service;
mod views;
use index::IndexResource;
use prepared_use::service::prepared;
#[cfg(test)]
mod observation;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::{
    join::NativeJoin,
    resources::ResourceProgram,
    types::OperatorContract,
    views::{IndexedRows},
};
use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_groups::types::Group;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_derived::domain::closure_adjacency::types::ClosurePartition;
use ryo_derived::domain::cycle_adjacency::types::CycleGraph;
use ryo_derived::domain::ordered::types::OrderedMatching;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, InputNumber, LiteralText},
    plans::types::Operation,
};

fn input<'f>(
    inputs: &[RepresentationInput<'f>],
    at: InputNumber,
) -> Result<RepresentationInput<'f>, OperatingError> {
    inputs
        .get(at.0)
        .copied()
        .ok_or(OperatingError::MissingInput { position: at.0 + 1 })
}

/// Resources are prepared once, after the scheduler's static/empty/first-row gates.
/// Regex resources are acquired separately after the checked physical/text row gates.
pub(crate) struct InvocationResources<'f> {
    pub(crate) cached_graph: Option<super::derived::owner::cached_graph::types::CachedGraph>,
    pub(crate) inputs: InvocationInputs<'f>,
}

/// The invocation binds these acquired handles once. Primitives see only the
/// immutable reader implemented for this value, never the owner or scheduler.
pub(crate) struct InvocationInputs<'f> {
    pub(super) index: Option<IndexResource<'f>>,
    pub(crate) appended: Option<Vec<Value>>,
    pub(crate) rank_layers: Option<Vec<Vec<LiteralText>>>,
    pub(crate) native_join: Option<NativeJoin<'f>>,
    pub(crate) native_storage: Option<AcquiredView>,
    pub(crate) cycles: Option<AcquiredView>,
    pub(crate) closure: Vec<super::derived::owner::acquisition::admission::service::AcquiredView>,
    pub(crate) partitioned_closure: Option<super::closure_partitions::PartitionedClosure<'f>>,
    pub(crate) ordered: Option<AcquiredView>,
    pub(crate) dfs: Option<AcquiredView>,
    pub(crate) axis: Option<super::axis_resources::view::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
}
impl<'f> InvocationResources<'f> {
    /// Validate required Join resource metadata without constructing a payload.
    /// Availability, geometry and static output checks remain at their existing
    /// owner boundaries; an optional unsupported native lookup still declines.
    pub(in crate::application::file_dag) fn validate_row_metadata(
        contract: &OperatorContract,
        inputs: &[RepresentationInput<'_>],
    ) -> Result<(), OperatingError> {
        use ryo_derived::domain::request::types::DerivedKind;
        let ResourceProgram::Join(join) = &contract.resources else {
            return Ok(());
        };
        let right = input(inputs, InputNumber(1))?;
        let validate = |kind| {
            let request = prepared(contract, kind)?;
            crate::domain::derived_views::service::validate(kind, &request.plan)
        };
        if join.reach.is_some() {
            validate(DerivedKind::PartitionOrder)?;
            validate(DerivedKind::ClosureAdjacency)
        } else if let RepresentationInput::Tree(tree) = right {
            validate(DerivedKind::NativeLookup)?;
            let native = prepared(contract, DerivedKind::NativeLookup)?;
            let form = match tree.form {
                crate::domain::table_cells::types::TreeForm::Elements => {
                    ryo_derived::domain::native::types::NativeForm::Elements
                }
                crate::domain::table_cells::types::TreeForm::Edges => {
                    ryo_derived::domain::native::types::NativeForm::Edges
                }
            };
            if ryo_derived::domain::dispatch::service::native_finding(&native.plan.semantic, form)
                .map_err(crate::domain::operator::contract::derived::portable_fault)?
                .is_some()
            {
                return Ok(());
            }
            if join.has_filter == crate::domain::operator::contract::resources::Filtering::Filtered || !join.view.is_empty() {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        } else {
            // The ordinary values driver has no native filter/view protocol.
            if join.has_filter == crate::domain::operator::contract::resources::Filtering::Filtered || !join.view.is_empty() {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        }
    }
}

fn partitioned<'f>(
    contract: &OperatorContract,
    input: RepresentationInput<'f>,
    admitted: &super::workspace::InputAdmissions<'_, 'f>,
    context: &mut OperatorContext<'_>,
) -> Result<super::closure_partitions::PartitionedClosure<'f>, OperatingError> {
    use ryo_derived::domain::request::types::DerivedKind;
    let order = prepared(contract, DerivedKind::PartitionOrder)?;
    let closure = prepared(contract, DerivedKind::ClosureAdjacency)?;
    admitted.partitioned(order, closure, input, context)
}

use ryo_values::domain::rows::types::Table;

impl crate::domain::operator::contract::views::ClosureParts
    for Vec<super::derived::owner::acquisition::admission::service::AcquiredView>
{
    fn len(&self) -> crate::domain::operator::contract::views::PartitionCount {
        crate::domain::operator::contract::views::PartitionCount(Vec::len(self))
    }
    fn get(
        &self,
        at: crate::domain::operator::contract::views::PartitionIndex,
    ) -> Result<&ryo_derived::domain::closure_adjacency::types::ClosurePartition, OperatingError> {
        let view = self
            .as_slice()
            .get(at.0)
            .ok_or(OperatingError::ShapeMismatch)?
            .view()?;
        match view {
            ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(part) => Ok(part),
            _ => Err(OperatingError::WrongOperation),
        }
    }
}

impl InvocationResources<'_> {
    pub(in crate::application::file_dag) fn native_join_lease(
        &self,
    ) -> Result<Option<NativeJoinLease<'_>>, OperatingError> {
        match (
            self.inputs.native_join.as_ref(),
            self.inputs.native_storage.as_ref(),
        ) {
            (None, None) => Ok(None),
            (Some(native), Some(acquired)) => Ok(Some(NativeJoinLease::new(*native, acquired))),
            _ => Err(OperatingError::WrongOperation),
        }
    }
}
