// ignore: idiomatic.arch/file-length - in the fused rewrite's replaced set (rewrite-replaced.toml): one invocation's preparation and the resources it holds, which the rewrite deletes whole; the part's module file kept 171 of these lines until the part-namespace placement moved them here
//! Invocation resources are acquired once, at the contract's demand boundary.
use super::groups::service::groups;
use super::index::{service::IndexResource, types::EmptyInput};
use super::prepared_use::service::prepared;
use crate::application::file_dag::axis_resources::service::prepare_axis;
use crate::application::file_dag::resources::service::appended_values;
use ryo_execution_domain::domain::operator::contract::{
    resources::{types::IndexColumns, types::JoinResources, types::ResourceProgram},
    types::OperatorContract,
    views::types::NativeJoin,
};
use ryo_execution_domain::domain::table_groups::types::Group;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_texts::types::Texts;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError, operator::types::OperatorContext,
};
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::{service::parameter_in, types::InputNumber};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// A compiler-proven direct row continuation has no acquisition program.
/// This creates borrowed semantic capability state, not an invocation setup.
pub(crate) fn continuation<'f>(
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
            closure: AcquiredPartitions::default(),
            partitioned_closure: None,
            ordered: None,
            dfs: None,
            axis: None,
            groups: None,
            // A continuation lends no index or axis, so nothing reports progress through it.
            #[cfg(feature = "diagnostic-row-progress")]
            progress: Default::default(),
        },
    })
}

pub(crate) fn prepare<'f>(
    contract: &OperatorContract,
    inputs: &[RepresentationInput<'f>],
    context: &mut OperatorContext<'_>,
    originals: &[ryo_execution_domain::domain::table_model::types::Rows<'f>],
    admission: &ryo_execution_store::application::file_dag::workspace::service::InvocationWorkspace,
    semantics: &crate::application::file_dag::execution::types::PrimitiveSemanticsBridge<'_>,
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
            closure: AcquiredPartitions::default(),
            partitioned_closure: None,
            ordered: None,
            dfs: None,
            axis: None,
            groups: None,
            #[cfg(feature = "diagnostic-row-progress")]
            progress: admission.progress_origin().profile().clone(),
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
                    == ryo_execution_domain::domain::operator::contract::types::EmptyRepresentation::Computed
                && originals.get(gate.input.0).is_some_and(|rows| {
                    matches!(
                        rows,
                        ryo_execution_domain::domain::table_model::types::Rows::Computed(_)
                            | ryo_execution_domain::domain::table_model::types::Rows::Compact(_)
                    ) && ryo_execution_domain::domain::table_model::service::count_of(rows).0 == 0
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
                contract, &admitted, context, empty,
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
                while cursor.advance()? == ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced {
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
                    let ryo_derived::domain::contract::types::DerivedView::NativeLookup(finding) =
                        acquired.view()?
                    else {
                        return Err(OperatingError::WrongOperation);
                    };
                    resources.inputs.native_join = Some(NativeJoin {
                        tree,
                        finding: *finding,
                    });
                    resources.inputs.native_storage = Some(acquired);
                } else {
                    if *has_filter == ryo_execution_domain::domain::operator::contract::resources::types::Filtering::Filtered || !view.is_empty() {
                        return Err(OperatingError::WrongOperation);
                    }
                    resources.inputs.index = Some(IndexResource::acquired(
                        contract,
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
                ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
                ryo_execution_store::application::file_dag::workspace::types::ViewInputs::Text(&texts),
            )?);
        }
        ResourceProgram::Dfs(_) => {
            use ryo_derived::domain::request::types::DerivedKind;
            let use_ = prepared(contract, DerivedKind::OrderedDfs)?;
            resources.inputs.dfs = Some(admitted.acquire_derived(
                use_,
                ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
                ryo_execution_store::application::file_dag::workspace::types::ViewInputs::Dfs {
                    symbols: &mut context.symbols,
                    source: context.source,
                    paths: context.paths,
                    arenas: context.arenas,
                },
            )?);
        }
        ResourceProgram::Axis(program) => {
            use crate::application::file_dag::axis_resources::{
                service::classify, types::AxisMode,
            };

            use ryo_derived::domain::request::types::DerivedKind;
            let edges = input(inputs, InputNumber(1))?;
            let domain = input(inputs, InputNumber(2))?;
            let mode_axis = classify(program, edges, domain)?;
            let slots = &contract.derived.slots;
            let equality = if matches!(mode_axis, AxisMode::Indexed { .. }) {
                Some(
                    admitted.acquire_rows(
                        slots[DerivedKind::Equality.ordinal()]
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
                        slots[DerivedKind::Membership.ordinal()]
                            .as_ref()
                            .ok_or(OperatingError::NotRun)?
                            .primary
                            .as_ref(),
                        ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::ordered(
                            ryo_execution_store::application::file_dag::derived::owner::types::MembershipOrder(1),
                        ),
                        ryo_execution_store::application::file_dag::workspace::types::ViewInputs::MembershipNative,
                    )?,
                ),
                AxisMode::Indexed { .. } => Some(
                    admitted.acquire_derived(
                        slots[DerivedKind::Membership.ordinal()]
                            .as_ref()
                            .ok_or(OperatingError::NotRun)?
                            .primary
                            .as_ref(),
                        ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
                        ryo_execution_store::application::file_dag::workspace::types::ViewInputs::MembershipValues,
                    )?,
                ),
            };
            resources.inputs.axis =
                Some(prepare_axis(program, edges, domain, equality, membership)?);
        }
    }
    Ok(resources)
}

pub(super) fn input<'f>(
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
                == ryo_execution_domain::domain::operator::contract::resources::types::Filtering::Filtered
                || !join.view.is_empty()
            {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        } else {
            // The ordinary values driver has no native filter/view protocol.
            if join.has_filter
                == ryo_execution_domain::domain::operator::contract::resources::types::Filtering::Filtered
                || !join.view.is_empty()
            {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        }
    }
}

pub(super) fn partitioned<'f>(
    contract: &OperatorContract,
    input: RepresentationInput<'f>,
    admitted: &ryo_execution_store::application::file_dag::workspace::service::InputAdmissions<
        '_,
        'f,
    >,
    context: &mut OperatorContext<'_>,
) -> Result<
    ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure<'f>,
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
pub(crate) struct AcquiredPartitions(pub(super) Vec<AcquiredView>);

impl AcquiredPartitions {
    /// Appends the next partition's view.
    pub(crate) fn push(&mut self, view: AcquiredView) {
        self.0.push(view);
    }
}

impl ryo_execution_domain::domain::operator::contract::views::types::ClosureParts
    for AcquiredPartitions
{
    fn len(
        &self,
    ) -> ryo_execution_domain::domain::operator::contract::views::types::PartitionCount {
        ryo_execution_domain::domain::operator::contract::views::types::PartitionCount(self.0.len())
    }
    fn get(
        &self,
        at: ryo_execution_domain::domain::operator::contract::views::types::PartitionIndex,
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

/// The invocation binds these acquired handles once. Primitives see only the
/// immutable reader implemented for this value, never the owner or scheduler.
pub(crate) struct InvocationInputs<'f> {
    pub(crate) index: Option<IndexResource<'f>>,
    pub(crate) appended: Option<Vec<Value>>,
    pub(crate) rank_layers: Option<Vec<Vec<LiteralText>>>,
    pub(crate) native_join: Option<NativeJoin<'f>>,
    pub(crate) native_storage: Option<AcquiredView>,
    pub(crate) cycles: Option<AcquiredView>,
    pub(crate) closure: AcquiredPartitions,
    pub(crate) partitioned_closure: Option<
        ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure<
            'f,
        >,
    >,
    pub(crate) ordered: Option<AcquiredView>,
    pub(crate) dfs: Option<AcquiredView>,
    pub(crate) axis: Option<super::super::axis_resources::view::service::AxisResource<'f>>,
    pub(crate) groups: Option<Vec<Group>>,
    /// The file's profile, the observer the lent indexes report their probes to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) progress:
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
}
