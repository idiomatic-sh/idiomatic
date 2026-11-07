//! Invocation resources are acquired once, at the contract's demand boundary.
use super::groups::groups;
use super::index::{EmptyInput, IndexResource};
use super::prepared_use::service::prepared;
use super::{InvocationInputs, InvocationResources, input, partitioned};
use crate::application::file_dag::axis_resources::prepare_axis;
use crate::application::file_dag::resources::appended_values;
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::{
    join::NativeJoin,
    resources::{IndexColumns, JoinResources, ResourceProgram},
    types::OperatorContract,
    
};
use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_texts::types::Texts;
use ryo_values::domain::plan_parts::{service::parameter_in, types::InputNumber};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// A compiler-proven direct row continuation has no acquisition program.
/// This creates borrowed semantic capability state, not an invocation setup.
pub(in crate::application::file_dag) fn continuation<'f>(
    contract: &OperatorContract,
) -> Result<InvocationResources<'f>, OperatingError> {
    if !matches!(contract.resources, ResourceProgram::None)
        || contract.derived.slots.iter().any(Option::is_some)
    {
        return Err(OperatingError::WrongOperation);
    }
    Ok(InvocationResources {
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


pub(in crate::application::file_dag) fn prepare<'f>(
    contract: &OperatorContract,
    inputs: &[RepresentationInput<'f>],
    context: &mut OperatorContext<'_>,
    originals: &[crate::domain::table_model::types::Rows<'f>],
    admission: &crate::application::file_dag::workspace::InvocationWorkspace,
    semantics: &crate::application::file_dag::execution::PrimitiveSemanticsBridge<'_>,
) -> Result<InvocationResources<'f>, OperatingError> {
    InvocationResources::validate_row_metadata(contract, inputs)?;
    let mut resources = InvocationResources {
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
            gate.input.0 > 0
                && gate.representation
                    == crate::domain::operator::contract::types::EmptyRepresentation::Computed
                && originals.get(gate.input.0).is_some_and(|rows| {
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
            .derived()
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
                originals.get(place.0),
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
            let right = input(inputs, InputNumber(1))?;
            if let Some(reach) = reach {
                let mut cursor = partitioned(contract, right, &admitted, context)?;
                while cursor.advance()? == crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced {
                    resources.inputs.closure.push(cursor.take_current()?);
                }
            } else {
                let native = if let RepresentationInput::Tree(tree) = right {
                    let use_ = prepared(
                        contract,
                        ryo_derived::domain::request::types::DerivedKind::NativeLookup,
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
                    if *has_filter == crate::domain::operator::contract::resources::Filtering::Filtered || !view.is_empty() {
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
                input(inputs, InputNumber(0))?,
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
                input(inputs, InputNumber(0))?,
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
                input(inputs, InputNumber(0))?,
                &admitted,
                context,
            )?);
        }
        ResourceProgram::Cycles { .. } => {
            let use_ = prepared(
                contract,
                ryo_derived::domain::request::types::DerivedKind::CycleAdjacency,
            )?;
            resources.inputs.cycles = Some(admitted.acquire_rows(use_, context)?);
        }
        ResourceProgram::Ordered { .. } => {
            use crate::domain::derived_views::types::DerivedInputs;
            use ryo_derived::domain::request::types::DerivedKind;
            let use_ = prepared(contract, DerivedKind::OrderedKeys)?;
            let texts = Texts {
                symbols: context.symbols.as_ref(),
                source: context.source,
                paths: context.paths,
                arenas: context.arenas,
            };
            resources.inputs.ordered = Some(admitted.acquire_derived(
                use_,
                crate::application::file_dag::derived::owner::ViewContext::default(),
                crate::application::file_dag::workspace::ViewInputs::Text(&texts),
            )?);
        }
        ResourceProgram::Dfs(_) => {
            use crate::domain::derived_views::types::DerivedInputs;
            use ryo_derived::domain::request::types::DerivedKind;
            let use_ = prepared(contract, DerivedKind::OrderedDfs)?;
            resources.inputs.dfs = Some(admitted.acquire_derived(
                use_,
                crate::application::file_dag::derived::owner::ViewContext::default(),
                crate::application::file_dag::workspace::ViewInputs::Dfs {
                    symbols: &mut context.symbols,
                    source: context.source,
                    paths: context.paths,
                    arenas: context.arenas,
                },
            )?);
        }
        ResourceProgram::Axis(program) => {
            use crate::application::file_dag::axis_resources::{AxisMode, classify};
            use crate::domain::derived_views::{
                service::slot,
                types::{DerivedInputs, MembershipSpace},
            };
            use ryo_derived::domain::request::types::DerivedKind;
            let edges = input(inputs, InputNumber(1))?;
            let domain = input(inputs, InputNumber(2))?;
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
                        crate::application::file_dag::derived::owner::ViewContext {
                            membership_order: Some(
                                crate::application::file_dag::derived::owner::types::MembershipOrder(1),
                            ),
                            ..Default::default()
                        },
                        crate::application::file_dag::workspace::ViewInputs::MembershipNative,
                    )?,
                ),
                AxisMode::Indexed { .. } => Some(
                    admitted.acquire_derived(
                        slots[slot(DerivedKind::Membership)]
                            .as_ref()
                            .ok_or(OperatingError::NotRun)?
                            .primary
                            .as_ref(),
                        crate::application::file_dag::derived::owner::ViewContext::default(),
                        crate::application::file_dag::workspace::ViewInputs::MembershipValues,
                    )?,
                ),
            };
            resources.inputs.axis =
                Some(prepare_axis(program, edges, domain, equality, membership)?);
        }
    }
    Ok(resources)
}
