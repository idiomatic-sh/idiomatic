//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
mod binding;
mod constants;
mod groups;
pub(crate) mod index;
mod prepared_use;
pub(crate) mod service;
mod views;
use index::IndexResource;
use prepared_use::service::prepared;
#[cfg(test)]
mod observation;
use ryo_derived::domain::cycle_adjacency::types::CycleGraph;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_derived::domain::ordered::types::OrderedMatching;
use ryo_execution_domain::domain::operator::contract::{
    resources::ResourceProgram, types::OperatorContract, views::NativeJoin,
};
use ryo_execution_domain::domain::table_groups::types::Group;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_reach::borrowed::IndexedRows;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError, operator::types::OperatorContext,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{InputNumber, LiteralText};

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
    pub(crate) cached_graph: Option<ryo_execution_store::application::file_dag::derived::owner::cached_graph::service::CachedGraph>,
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
    pub(crate) closure: AcquiredPartitions,
    pub(crate) partitioned_closure: Option<
        ryo_execution_store::application::file_dag::closure_partitions::PartitionedClosure<'f>,
    >,
    pub(crate) ordered: Option<AcquiredView>,
    pub(crate) dfs: Option<AcquiredView>,
    pub(crate) axis: Option<super::axis_resources::view::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
    /// The file's profile, the observer the lent indexes report their probes to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) progress:
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
}
impl<'f> InvocationResources<'f> {
    /// Validate required Join resource metadata without constructing a payload.
    /// Availability, geometry and static output checks remain at their existing
    /// owner boundaries; an optional unsupported native lookup still declines.
    pub(crate) fn validate_row_metadata(
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
            ryo_execution_domain::domain::derived_views::service::validate(kind, &request.plan)
        };
        if join.reach.is_some() {
            validate(DerivedKind::PartitionOrder)?;
            validate(DerivedKind::ClosureAdjacency)
        } else if let RepresentationInput::Tree(tree) = right {
            validate(DerivedKind::NativeLookup)?;
            let native = prepared(contract, DerivedKind::NativeLookup)?;
            let form = match tree.form {
                ryo_execution_domain::domain::table_cells::types::TreeForm::Elements => {
                    ryo_derived::domain::native::types::NativeForm::Elements
                }
                ryo_execution_domain::domain::table_cells::types::TreeForm::Edges => {
                    ryo_derived::domain::native::types::NativeForm::Edges
                }
            };
            if ryo_derived::domain::dispatch::service::native_finding(&native.plan.semantic, form)
                .map_err(ryo_execution_domain::domain::operating::service::derived_fault)?
                .is_some()
            {
                return Ok(());
            }
            if join.has_filter
                == ryo_execution_domain::domain::operator::contract::resources::Filtering::Filtered
                || !join.view.is_empty()
            {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        } else {
            // The ordinary values driver has no native filter/view protocol.
            if join.has_filter
                == ryo_execution_domain::domain::operator::contract::resources::Filtering::Filtered
                || !join.view.is_empty()
            {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        }
    }
}

fn partitioned<'f>(
    contract: &OperatorContract,
    input: RepresentationInput<'f>,
    admitted: &ryo_execution_store::application::file_dag::workspace::InputAdmissions<'_, 'f>,
    context: &mut OperatorContext<'_>,
) -> Result<
    ryo_execution_store::application::file_dag::closure_partitions::PartitionedClosure<'f>,
    OperatingError,
> {
    use ryo_derived::domain::request::types::DerivedKind;
    let order = prepared(contract, DerivedKind::PartitionOrder)?;
    let closure = prepared(contract, DerivedKind::ClosureAdjacency)?;
    admitted.partitioned(order, closure, input, context)
}

/// The closure partitions an invocation acquired, in partition order: the views the store leased, held as one
/// list so the engine can lend them to the domain's partition access (a foreign list cannot implement it).
#[derive(Debug, Default)]
pub(crate) struct AcquiredPartitions(Vec<AcquiredView>);
impl AcquiredPartitions {
    /// Appends the next partition's view.
    pub(crate) fn push(&mut self, view: AcquiredView) {
        self.0.push(view);
    }
}

impl ryo_execution_domain::domain::operator::contract::views::ClosureParts for AcquiredPartitions {
    fn len(&self) -> ryo_execution_domain::domain::operator::contract::views::PartitionCount {
        ryo_execution_domain::domain::operator::contract::views::PartitionCount(self.0.len())
    }
    fn get(
        &self,
        at: ryo_execution_domain::domain::operator::contract::views::PartitionIndex,
    ) -> Result<&ryo_derived::domain::closure_adjacency::types::ClosurePartition, OperatingError>
    {
        let view = self
            .0
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
