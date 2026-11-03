//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
use super::{
    axis_resources::prepare_axis, derived::view::AcquiredView, resources::appended_values,
};
mod groups;
mod index;
mod views;
use groups::groups;
pub(super) use index::IndexResource;
#[cfg(test)]
mod observation;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::{
    graph::{ClosurePartition, CycleGraph, OrderedMatching, PreparedDfs},
    join::NativeJoin,
    resources::{IndexColumns, JoinResources, ResourceProgram},
    types::{EvaluationMode, OperatorContract},
    views::{BorrowedRows, IndexedRows},
};
use crate::domain::table_groups::types::Group;
use crate::domain::table_model::types::{KeyIndex, ParameterBinding, Table};
use crate::domain::table_texts::types::Texts;
use crate::domain::tree_join::service::tree_finding;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::{
        service::parameter_in,
        types::{ColumnIndex, LiteralText},
    },
    plans::types::Operation,
};

fn input<'f>(inputs: &[BorrowedRows<'f>], at: usize) -> Result<BorrowedRows<'f>, OperatingError> {
    inputs
        .get(at)
        .copied()
        .ok_or(OperatingError::MissingInput { position: at + 1 })
}

/// Resources are prepared once, after the scheduler's static/empty/first-row gates.
/// Regex resources are acquired separately after the checked physical/text row gates.
pub(crate) struct InvocationResources<'f> {
    pub(crate) cached_graph: Option<super::cached_graph::CachedGraph>,
    pub(super) index: Option<IndexResource<'f>>,
    pub(crate) appended: Option<Vec<Value>>,
    pub(crate) rank_layers: Option<Vec<Vec<LiteralText>>>,
    pub(crate) native_join: Option<NativeJoin<'f>>,
    pub(crate) native_storage: Option<AcquiredView>,
    pub(crate) cycles: Option<AcquiredView>,
    pub(crate) closure: Vec<ClosurePartition>,
    pub(super) closure_leases: Vec<std::sync::Arc<super::derived::ledger::ResourceLease>>,
    pub(crate) partitioned_closure: Option<super::closure_partitions::PartitionedClosure<'f>>,
    pub(crate) ordered: Option<AcquiredView>,
    pub(crate) dfs: Option<AcquiredView>,
    pub(crate) axis: Option<super::axis_resources::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
}
impl<'f> InvocationResources<'f> {
    /// Streaming aggregation has already acquired and validated the contributing population.
    pub(crate) fn with_groups(groups: Vec<Group>) -> Self {
        Self {
            cached_graph: None,
            index: None,
            appended: None,
            rank_layers: None,
            native_join: None,
            native_storage: None,
            cycles: None,
            closure: Vec::new(),
            closure_leases: Vec::new(),
            partitioned_closure: None,
            ordered: None,
            dfs: None,
            axis: None,
            groups: Some(groups),
        }
    }
    pub(crate) fn prepare(
        contract: &OperatorContract,
        inputs: &[BorrowedRows<'f>],
        context: &mut OperatorContext<'_>,
        mode: EvaluationMode,
        originals: &[crate::domain::table_model::types::Rows<'f>],
        admission: &super::workspace::InvocationWorkspace,
    ) -> Result<Self, OperatingError> {
        let mut resources = Self {
            cached_graph: None,
            index: None,
            appended: None,
            rank_layers: None,
            native_join: None,
            native_storage: None,
            cycles: None,
            closure: Vec::new(),
            closure_leases: Vec::new(),
            partitioned_closure: None,
            ordered: None,
            dfs: None,
            axis: None,
            groups: None,
        };
        let program = match mode {
            EvaluationMode::Streaming => &contract.resources,
            EvaluationMode::Collected => &contract.collected_resources,
        };
        let admitted = admission.inputs(inputs);
        // Empty contributing domain gates retain their ordinary typed resource
        // shortcut; no traversal request or memo is made for them.
        let empty_domain=contract.collected_empty.iter().any(|gate|gate.input>0 && gate.representation==crate::domain::operator::contract::types::EmptyRepresentation::Computed &&
          originals.get(gate.input).is_some_and(|rows|matches!(rows,crate::domain::table_model::types::Rows::Computed(table) if table.count.0==0)));
        if !empty_domain {
            resources.cached_graph = admission
                .owner()
                .derived
                .prepare_graph(contract, admission, mode, program, originals)?;
            if resources.cached_graph.is_some() {
                return Ok(resources);
            }
        }
        match program {
            ResourceProgram::None => {}
            ResourceProgram::Appended(values) => {
                let mut values_out = Vec::with_capacity(values.len());
                appended_values(values, context, &mut values_out)?;
                resources.appended = Some(values_out);
            }
            ResourceProgram::Rank(sets) => {
                let ParameterBinding::TextSets(layers) =
                    parameter_in(context.bindings, *sets).ok_or(OperatingError::Unbound)?
                else {
                    return Err(OperatingError::WrongBinding);
                };
                resources.rank_layers = Some(layers.clone());
            }
            ResourceProgram::Index {
                input: place,
                columns,
            } => {
                let source = input(inputs, *place)?;
                let skip_empty = matches!(columns, IndexColumns::ColumnsUnlessEmpty(_));
                resources.index = Some(IndexResource::acquired(
                    contract,
                    mode,
                    originals.get(*place),
                    &admitted,
                    context,
                    skip_empty,
                )?);
            }
            ResourceProgram::Join(JoinResources {
                left_on,
                right_on,
                has_filter,
                view,
                reach,
                ..
            }) => {
                let right = input(inputs, 1)?;
                if let Some(reach) = reach {
                    let mut cursor = partitioned(contract, mode, right, &admitted, context)?;
                    while cursor.advance()? {
                        let (part, lease) = cursor.take_current()?;
                        resources.closure.push(part);
                        resources.closure_leases.push(lease);
                    }
                } else if let BorrowedRows::Tree(tree) = right
                    && tree_finding(&tree, left_on, right_on, view).is_some()
                {
                    let use_ = prepared(
                        contract,
                        mode,
                        crate::domain::derived_views::types::DerivedKind::NativeLookup,
                    )?;
                    let storage = admitted.acquire_rows(use_, context)?;
                    let acquired = storage;
                    let crate::domain::derived_views::types::DerivedView::NativeLookup(finding) =
                        acquired.view()?
                    else {
                        return Err(OperatingError::WrongOperation);
                    };
                    resources.native_join = Some(NativeJoin { tree, finding });
                    resources.native_storage = Some(acquired);
                } else {
                    if *has_filter || !view.is_empty() {
                        return Err(OperatingError::WrongOperation);
                    }
                    resources.index = Some(IndexResource::acquired(
                        contract,
                        mode,
                        originals.get(1),
                        &admitted,
                        context,
                        false,
                    )?);
                }
            }
            ResourceProgram::Count { keys, distinct } => {
                resources.groups = Some(groups(input(inputs, 0)?, keys, *distinct, None, context)?);
            }
            ResourceProgram::Digest { keys, over } => {
                resources.groups =
                    Some(groups(input(inputs, 0)?, keys, None, Some(*over), context)?);
            }
            ResourceProgram::Closure { .. } => {
                resources.partitioned_closure = Some(partitioned(
                    contract,
                    mode,
                    input(inputs, 0)?,
                    &admitted,
                    context,
                )?);
            }
            ResourceProgram::Cycles { .. } => {
                let use_ = prepared(
                    contract,
                    mode,
                    crate::domain::derived_views::types::DerivedKind::CycleAdjacency,
                )?;
                resources.cycles = Some(admitted.acquire_rows(use_, context)?);
            }
            ResourceProgram::Ordered { .. } => {
                use crate::domain::derived_views::types::{DerivedInputs, DerivedKind};
                let use_ = prepared(contract, mode, DerivedKind::OrderedKeys)?;
                let texts = Texts {
                    symbols: context.symbols.as_ref(),
                    source: context.source,
                    paths: context.paths,
                    arenas: context.arenas,
                };
                resources.ordered = Some(admitted.acquire_derived(
                    use_,
                    super::derived::owner::ViewContext::default(),
                    super::workspace::ViewInputs::Text(&texts),
                )?);
            }
            ResourceProgram::Dfs(_) => {
                use crate::domain::derived_views::types::{DerivedInputs, DerivedKind};
                let use_ = prepared(contract, mode, DerivedKind::OrderedDfs)?;
                resources.dfs = Some(admitted.acquire_derived(
                    use_,
                    super::derived::owner::ViewContext::default(),
                    super::workspace::ViewInputs::Dfs {
                        symbols: &mut context.symbols,
                        source: context.source,
                        paths: context.paths,
                        arenas: context.arenas,
                    },
                )?);
            }
            ResourceProgram::Axis(program) => {
                use super::axis_resources::{AxisMode, classify};
                use crate::domain::derived_views::{
                    service::slot,
                    types::{DerivedInputs, DerivedKind, MembershipSpace},
                };
                let edges = input(inputs, 1)?;
                let domain = input(inputs, 2)?;
                let mode_axis = classify(program, edges, domain)?;
                let slots = match mode {
                    EvaluationMode::Streaming => &contract.derived.streaming,
                    EvaluationMode::Collected => &contract.derived.collected,
                };
                let equality = if matches!(mode_axis, AxisMode::Indexed { .. }) {
                    Some(
                        admitted.acquire_rows(
                            slots[slot(DerivedKind::Equality)]
                                .as_ref()
                                .ok_or(OperatingError::NotRun)?,
                            context,
                        )?,
                    )
                } else {
                    None
                };
                let membership = match mode_axis {
                    AxisMode::Empty | AxisMode::Native { among: Some(_), .. } => None,
                    AxisMode::Native { tree, among: None } => Some(
                        admitted.acquire_derived(
                            slots[slot(DerivedKind::Membership)]
                                .as_ref()
                                .ok_or(OperatingError::NotRun)?,
                            super::derived::owner::ViewContext {
                                membership_order: Some(1),
                                ..Default::default()
                            },
                            super::workspace::ViewInputs::MembershipNative,
                        )?,
                    ),
                    AxisMode::Indexed { .. } => Some(
                        admitted.acquire_derived(
                            slots[slot(DerivedKind::Membership)]
                                .as_ref()
                                .ok_or(OperatingError::NotRun)?,
                            super::derived::owner::ViewContext::default(),
                            super::workspace::ViewInputs::MembershipValues,
                        )?,
                    ),
                };
                resources.axis = Some(prepare_axis(program, edges, domain, equality, membership)?);
            }
        }
        Ok(resources)
    }
}

fn prepared(
    contract: &OperatorContract,
    mode: EvaluationMode,
    kind: crate::domain::derived_views::types::DerivedKind,
) -> Result<&crate::domain::derived_views::types::PreparedViewUse, OperatingError> {
    let slots = match mode {
        EvaluationMode::Streaming => &contract.derived.streaming,
        EvaluationMode::Collected => &contract.derived.collected,
    };
    slots[crate::domain::derived_views::service::slot(kind)]
        .as_ref()
        .map(|group| group.primary.as_ref())
        .ok_or(OperatingError::NotRun)
}

fn partitioned<'f>(
    contract: &OperatorContract,
    mode: EvaluationMode,
    input: BorrowedRows<'f>,
    admitted: &super::workspace::InputAdmissions<'_, 'f>,
    context: &mut OperatorContext<'_>,
) -> Result<super::closure_partitions::PartitionedClosure<'f>, OperatingError> {
    use crate::domain::derived_views::types::DerivedKind;
    let order = prepared(contract, mode, DerivedKind::PartitionOrder)?;
    let closure = prepared(contract, mode, DerivedKind::ClosureAdjacency)?;
    let storage = admitted.acquire_rows(order, context)?;
    super::closure_partitions::PartitionedClosure::from_prepared(
        input,
        (order.plan).clone(),
        (closure.plan).clone(),
        storage,
        admitted.owner_handle(),
    )
}
