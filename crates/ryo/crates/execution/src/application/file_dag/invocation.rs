//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
use super::{
    axis_resources::prepare_axis, derived::owner::acquisition::admission::service::AcquiredView,
    resources::appended_values,
};
mod binding;
mod constants;
pub(in crate::application::file_dag) mod native;
use native::service::NativeJoinLease;
mod groups;
pub(in crate::application::file_dag) mod index;
mod prepared_use;
mod views;
use groups::groups;
use index::{EmptyInput, IndexResource};
use prepared_use::service::prepared;
#[cfg(test)]
mod observation;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::{
    join::NativeJoin,
    resources::{IndexColumns, JoinResources, ResourceProgram},
    types::OperatorContract,
    views::{BorrowedRows, IndexedRows},
};
use crate::domain::table_groups::types::Group;
use crate::domain::table_texts::types::Texts;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_derived::domain::contract::types::{
    ClosurePartition, CycleGraph, OrderedMatching, PreparedDfs,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::query_bindings::types::ParameterBinding;
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
    pub(crate) axis: Option<super::axis_resources::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
}
impl<'f> InvocationResources<'f> {
    /// A compiler-proven direct row continuation has no acquisition program.
    /// This creates borrowed semantic capability state, not an invocation setup.
    pub(in crate::application::file_dag) fn continuation(
        contract: &OperatorContract,
    ) -> Result<Self, OperatingError> {
        if !matches!(contract.resources, ResourceProgram::None)
            || contract.derived.slots.iter().any(Option::is_some)
        {
            return Err(OperatingError::WrongOperation);
        }
        Ok(Self {
            cached_graph: None,
            inputs: InvocationInputs {
                index: None,
                appended: None,
                rank_layers: None,
                native_join: None,
                native_storage: None,
                cycles: None,
                closure: Vec::new(),
                partitioned_closure: None,
                ordered: None,
                dfs: None,
                axis: None,
                groups: None,
            },
        })
    }

    /// Validate required Join resource metadata without constructing a payload.
    /// Availability, geometry and static output checks remain at their existing
    /// owner boundaries; an optional unsupported native lookup still declines.
    pub(in crate::application::file_dag) fn validate_row_metadata(
        contract: &OperatorContract,
        inputs: &[BorrowedRows<'_>],
    ) -> Result<(), OperatingError> {
        use ryo_derived::domain::contract::types::DerivedKind;
        let ResourceProgram::Join(join) = &contract.resources else {
            return Ok(());
        };
        let right = input(inputs, 1)?;
        let validate = |kind| {
            let request = prepared(contract, kind)?;
            crate::domain::derived_views::service::validate(kind, &request.plan)
        };
        if join.reach.is_some() {
            validate(DerivedKind::PartitionOrder)?;
            validate(DerivedKind::ClosureAdjacency)
        } else if let BorrowedRows::Tree(tree) = right {
            validate(DerivedKind::NativeLookup)?;
            let native = prepared(contract, DerivedKind::NativeLookup)?;
            let form = match tree.form {
                crate::domain::table_cells::types::TreeForm::Elements => {
                    ryo_derived::domain::contract::types::NativeForm::Elements
                }
                crate::domain::table_cells::types::TreeForm::Edges => {
                    ryo_derived::domain::contract::types::NativeForm::Edges
                }
            };
            if native
                .plan
                .semantic
                .native_finding(form)
                .map_err(crate::domain::operator::contract::derived::portable_fault)?
                .is_some()
            {
                return Ok(());
            }
            if join.has_filter || !join.view.is_empty() {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        } else {
            // The ordinary values driver has no native filter/view protocol.
            if join.has_filter || !join.view.is_empty() {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        }
    }

    pub(in crate::application::file_dag) fn prepare(
        contract: &OperatorContract,
        inputs: &[BorrowedRows<'f>],
        context: &mut OperatorContext<'_>,
        originals: &[crate::domain::table_model::types::Rows<'f>],
        admission: &super::workspace::InvocationWorkspace,
        semantics: &super::execution::PrimitiveSemanticsBridge<'_>,
    ) -> Result<Self, OperatingError> {
        Self::validate_row_metadata(contract, inputs)?;
        let mut resources = Self {
            cached_graph: None,
            inputs: InvocationInputs {
                index: None,
                appended: None,
                rank_layers: None,
                native_join: None,
                native_storage: None,
                cycles: None,
                closure: Vec::new(),
                partitioned_closure: None,
                ordered: None,
                dfs: None,
                axis: None,
                groups: None,
            },
        };
        let program = &contract.resources;
        let admitted = admission.inputs(inputs);
        // Empty contributing domain gates retain their ordinary typed resource
        // shortcut; no traversal request or memo is made for them.
        let empty_domain = contract
            .empty_results
            .iter()
            .find(|gate| {
                gate.input > 0
                    && gate.representation
                        == crate::domain::operator::contract::types::EmptyRepresentation::Computed
                    && originals.get(gate.input).is_some_and(|rows| {
                        matches!(
                            rows,
                            crate::domain::table_model::types::Rows::Computed(_)
                                | crate::domain::table_model::types::Rows::Compact(_)
                        ) && crate::domain::table_model::service::count_of(rows).0 == 0
                    })
            })
            .map(|gate| gate.kinds.as_ref().map(|_| true).map_err(Clone::clone))
            .transpose()?
            .unwrap_or(false);
        if !empty_domain {
            resources.cached_graph = admission
                .owner()
                .derived
                .prepare_graph(contract, admission, program, originals)?;
            if resources.cached_graph.is_some() {
                return Ok(resources);
            }
        }
        match program {
            ResourceProgram::None => {}
            ResourceProgram::Appended(values) => {
                let mut values_out = Vec::with_capacity(values.len());
                appended_values(values, context, &mut values_out)?;
                resources.inputs.appended = Some(values_out);
            }
            ResourceProgram::Rank(sets) => {
                let ParameterBinding::TextSets(layers) =
                    parameter_in(context.bindings, *sets).ok_or(OperatingError::Unbound)?
                else {
                    return Err(OperatingError::WrongBinding);
                };
                resources.inputs.rank_layers = Some(layers.clone());
            }
            ResourceProgram::Index {
                input: place,
                columns,
            } => {
                let source = input(inputs, *place)?;
                let empty = if matches!(columns, IndexColumns::ColumnsUnlessEmpty(_)) {
                    EmptyInput::Skip
                } else {
                    EmptyInput::Keep
                };
                resources.inputs.index = Some(IndexResource::acquired(
                    contract,
                    originals.get(*place),
                    &admitted,
                    context,
                    empty,
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
                    let mut cursor = partitioned(contract, right, &admitted, context)?;
                    while cursor.advance()? {
                        resources.inputs.closure.push(cursor.take_current()?);
                    }
                } else {
                    let native = if let BorrowedRows::Tree(tree) = right {
                        let use_ = prepared(
                            contract,
                            ryo_derived::domain::contract::types::DerivedKind::NativeLookup,
                        )?;
                        admitted
                            .try_native_lookup(use_)?
                            .map(|acquired| (tree, acquired))
                    } else {
                        None
                    };
                    if let Some((tree, acquired)) = native {
                        let ryo_derived::domain::contract::types::DerivedView::NativeLookup(
                            finding,
                        ) = acquired.view()?
                        else {
                            return Err(OperatingError::WrongOperation);
                        };
                        resources.inputs.native_join = Some(NativeJoin {
                            tree,
                            finding: *finding,
                        });
                        resources.inputs.native_storage = Some(acquired);
                    } else {
                        if *has_filter || !view.is_empty() {
                            return Err(OperatingError::WrongOperation);
                        }
                        resources.inputs.index = Some(IndexResource::acquired(
                            contract,
                            originals.get(1),
                            &admitted,
                            context,
                            EmptyInput::Keep,
                        )?);
                    }
                }
            }

            ResourceProgram::Count { keys, distinct } => {
                resources.inputs.groups = Some(groups(
                    input(inputs, 0)?,
                    keys,
                    *distinct,
                    None,
                    context,
                    contract,
                    &admitted,
                    semantics,
                )?);
            }
            ResourceProgram::Digest { keys, over } => {
                resources.inputs.groups = Some(groups(
                    input(inputs, 0)?,
                    keys,
                    None,
                    Some(*over),
                    context,
                    contract,
                    &admitted,
                    semantics,
                )?);
            }
            ResourceProgram::Closure { .. } => {
                resources.inputs.partitioned_closure = Some(partitioned(
                    contract,
                    input(inputs, 0)?,
                    &admitted,
                    context,
                )?);
            }
            ResourceProgram::Cycles { .. } => {
                let use_ = prepared(
                    contract,
                    ryo_derived::domain::contract::types::DerivedKind::CycleAdjacency,
                )?;
                resources.inputs.cycles = Some(admitted.acquire_rows(use_, context)?);
            }
            ResourceProgram::Ordered { .. } => {
                use crate::domain::derived_views::types::DerivedInputs;
                use ryo_derived::domain::contract::types::DerivedKind;
                let use_ = prepared(contract, DerivedKind::OrderedKeys)?;
                let texts = Texts {
                    symbols: context.symbols.as_ref(),
                    source: context.source,
                    paths: context.paths,
                    arenas: context.arenas,
                };
                resources.inputs.ordered = Some(admitted.acquire_derived(
                    use_,
                    super::derived::owner::ViewContext::default(),
                    super::workspace::ViewInputs::Text(&texts),
                )?);
            }
            ResourceProgram::Dfs(_) => {
                use crate::domain::derived_views::types::DerivedInputs;
                use ryo_derived::domain::contract::types::DerivedKind;
                let use_ = prepared(contract, DerivedKind::OrderedDfs)?;
                resources.inputs.dfs = Some(admitted.acquire_derived(
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
                    types::{DerivedInputs, MembershipSpace},
                };
                use ryo_derived::domain::contract::types::DerivedKind;
                let edges = input(inputs, 1)?;
                let domain = input(inputs, 2)?;
                let mode_axis = classify(program, edges, domain)?;
                let slots = &contract.derived.slots;
                let equality = if matches!(mode_axis, AxisMode::Indexed { .. }) {
                    Some(
                        admitted.acquire_rows(
                            slots[slot(DerivedKind::Equality)]
                                .as_ref()
                                .ok_or(OperatingError::NotRun)?
                                .primary
                                .as_ref(),
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
                                .ok_or(OperatingError::NotRun)?
                                .primary
                                .as_ref(),
                            super::derived::owner::ViewContext {
                                membership_order: Some(
                                    crate::application::file_dag::derived::owner::types::MembershipOrder(1),
                                ),
                                ..Default::default()
                            },
                            super::workspace::ViewInputs::MembershipNative,
                        )?,
                    ),
                    AxisMode::Indexed { .. } => Some(
                        admitted.acquire_derived(
                            slots[slot(DerivedKind::Membership)]
                                .as_ref()
                                .ok_or(OperatingError::NotRun)?
                                .primary
                                .as_ref(),
                            super::derived::owner::ViewContext::default(),
                            super::workspace::ViewInputs::MembershipValues,
                        )?,
                    ),
                };
                resources.inputs.axis =
                    Some(prepare_axis(program, edges, domain, equality, membership)?);
            }
        }
        Ok(resources)
    }
}

fn partitioned<'f>(
    contract: &OperatorContract,
    input: BorrowedRows<'f>,
    admitted: &super::workspace::InputAdmissions<'_, 'f>,
    context: &mut OperatorContext<'_>,
) -> Result<super::closure_partitions::PartitionedClosure<'f>, OperatingError> {
    use ryo_derived::domain::contract::types::DerivedKind;
    let order = prepared(contract, DerivedKind::PartitionOrder)?;
    let closure = prepared(contract, DerivedKind::ClosureAdjacency)?;
    admitted.partitioned(order, closure, input, context)
}

use ryo_values::domain::rows::types::Table;

impl crate::domain::operator::contract::views::ClosureParts
    for Vec<super::derived::owner::acquisition::admission::service::AcquiredView>
{
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn get(
        &self,
        at: usize,
    ) -> Result<&ryo_derived::domain::contract::types::ClosurePartition, OperatingError> {
        let view = self
            .as_slice()
            .get(at)
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
