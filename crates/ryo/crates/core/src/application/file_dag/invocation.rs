//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
use super::{axis_resources::prepare_axis, graph_resources, resources::appended_values};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operator::contract::{
    axis::PreparedAxis,
    graph::{ClosurePartition, CycleGraph, OrderedMatching, PreparedDfs},
    join::NativeJoin,
    resources::{IndexColumns, JoinResources, ResourceProgram},
    types::{EvaluationMode, OperatorContract},
    views::{BorrowedRows, IndexedRows},
};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_groups::{
    service::{count_groups, digest_groups},
    types::{CountRoom, Group, Hashes},
};
use crate::domain::table_model::{
    service::{index, index_stored, push_row},
    types::{KeyIndex, On, ParameterBinding, Table},
};
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

/// Ownership stays at the invocation edge; borrowed views contain no retained handles.
pub(super) enum IndexedInput<'f> {
    Borrowed(BorrowedRows<'f>),
    Owned(Table),
}
pub(super) enum IndexStorage {
    Local(KeyIndex),
    Shared(std::sync::Arc<KeyIndex>),
}
impl IndexStorage {
    fn borrow(&self) -> &KeyIndex {
        match self {
            Self::Local(index) => index,
            Self::Shared(index) => index.as_ref(),
        }
    }
}
pub(super) enum IndexResource<'f> {
    Ordinary {
        rows: IndexedInput<'f>,
        columns: Vec<ColumnIndex>,
        index: IndexStorage,
    },
    Selected(super::selected::SelectedInput<'f>),
}
impl<'f> IndexResource<'f> {
    fn prepare(
        input: BorrowedRows<'f>,
        columns: &[ColumnIndex],
        logical: Option<ryo_values::domain::plan_parts::types::Input>,
        context: &mut OperatorContext<'_>,
        skip_empty: bool,
        original: Option<&crate::domain::table_model::types::Rows<'f>>,
        proven: bool,
    ) -> Result<Self, OperatingError> {
        let input = crate::domain::table_model::collections::representations::service::registered(
            crate::domain::table_model::collections::representations::service::kind(input),
        )
        .bulk(input)?;
        if let Some(crate::domain::table_model::types::Rows::Selected(selected)) = original {
            return Ok(Self::Selected(
                super::selected::SelectedInput::prepare_admitted(selected, columns, proven)?,
            ));
        }
        if skip_empty && input.count() == 0 {
            return Ok(Self::Ordinary {
                rows: IndexedInput::Borrowed(input),
                columns: columns.to_vec(),
                index: IndexStorage::Local(KeyIndex::default()),
            });
        }
        crate::domain::table_model::service::check_columns(input.kinds(), columns)?;
        if !proven {
            input.validate()?;
        }
        let mut order = KeyIndex::default();
        let mut shared = None;
        let rows = if let BorrowedRows::Stored(view) = input {
            if let (
                Some(ryo_values::domain::plan_parts::types::Input::Relation(relation)),
                Some(cache),
            ) = (logical, context.stored_indexes.as_deref_mut())
            {
                let acquired = crate::domain::stored_indexes::service::acquire(
                    cache, relation, view, columns, &mut order,
                )?;
                tracing::trace!(target:"ryo::diagnostic::resource_acquire",event="index_acquire",built=acquired.built,eligible=acquired.eligible,shared=acquired.shared.is_some());
                shared = acquired.shared;
            } else {
                index_stored(view, columns, &mut order)?;
            }
            IndexedInput::Borrowed(input)
        } else if let BorrowedRows::Computed(table) = input {
            index(table, On::Columns(columns), &mut order)?;
            IndexedInput::Borrowed(input)
        } else {
            let table = materialized(input);
            index(&table, On::Columns(columns), &mut order)?;
            IndexedInput::Owned(table)
        };
        Ok(Self::Ordinary {
            rows,
            columns: columns.to_vec(),
            index: shared.map_or(IndexStorage::Local(order), IndexStorage::Shared),
        })
    }

    pub(super) fn view(&self) -> IndexedRows<'_> {
        match self {
            Self::Selected(selected) => selected.view(),
            Self::Ordinary {
                rows,
                columns,
                index,
            } => IndexedRows {
                rows: match rows {
                    IndexedInput::Borrowed(rows) => *rows,
                    IndexedInput::Owned(table) => BorrowedRows::Computed(table),
                },
                columns,
                index: index.borrow(),
            },
        }
    }
    pub(super) fn selected(&self) -> Option<&super::selected::SelectedInput<'_>> {
        match self {
            Self::Selected(selected) => Some(selected),
            Self::Ordinary { .. } => None,
        }
    }
}
fn materialized(input: BorrowedRows<'_>) -> Table {
    let mut table = Table {
        kinds: input.kinds().to_vec(),
        ..Table::default()
    };
    let mut row = Vec::new();
    for at in 0..input.count() {
        if let Some(row) = input.read(RowIndex(at), &mut row) {
            push_row(&mut table, row);
        }
    }
    table
}
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
    pub(crate) cycles: Option<CycleGraph>,
    pub(crate) closure: Vec<ClosurePartition>,
    pub(crate) partitioned_closure: Option<super::closure_partitions::PartitionedClosure<'f>>,
    pub(crate) ordered: Option<OrderedMatching>,
    pub(crate) dfs: Option<PreparedDfs>,
    pub(crate) axis: Option<PreparedAxis<'f>>,
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
            cycles: None,
            closure: Vec::new(),
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
            cycles: None,
            closure: Vec::new(),
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
        // Empty contributing domain gates retain their ordinary typed resource
        // shortcut; no traversal request or memo is made for them.
        let empty_domain=contract.collected_empty.iter().any(|gate|gate.input>0 && gate.representation==crate::domain::operator::contract::types::EmptyRepresentation::Computed &&
          originals.get(gate.input).is_some_and(|rows|matches!(rows,crate::domain::table_model::types::Rows::Computed(table) if table.count.0==0)));
        if !empty_domain {
            resources.cached_graph = super::cached_graph::CachedGraph::prepare(
                program,
                ryo_values::domain::plans::service::named(&contract.inputs),
                originals,
                context.stored_indexes.as_deref_mut(),
            )?;
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
                let whole;
                let columns = match columns {
                    IndexColumns::Columns(columns) | IndexColumns::ColumnsUnlessEmpty(columns) => {
                        columns.as_slice()
                    }
                    IndexColumns::Whole => {
                        whole = (0..contract
                            .input_kinds
                            .get(*place)
                            .map_or(source.kinds().len(), Vec::len))
                            .map(|at| ColumnIndex(at as u32))
                            .collect::<Vec<_>>();
                        &whole
                    }
                };
                resources.index = Some(IndexResource::prepare(
                    source,
                    columns,
                    contract.inputs.list.get(*place).copied(),
                    context,
                    skip_empty,
                    originals.get(*place),
                    admission.proven(*place, source)?,
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
                    resources.closure = graph_resources::prepare_closure(
                        right,
                        &reach.keys,
                        reach.source,
                        reach.destination,
                    )?;
                } else if let BorrowedRows::Tree(tree) = right
                    && let Some(finding) = tree_finding(&tree, left_on, right_on, view)
                {
                    resources.native_join = Some(NativeJoin { tree, finding });
                } else {
                    if *has_filter || !view.is_empty() {
                        return Err(OperatingError::WrongOperation);
                    }
                    resources.index = Some(IndexResource::prepare(
                        right,
                        right_on,
                        contract.inputs.list.get(1).copied(),
                        context,
                        false,
                        originals.get(1),
                        admission.proven(1, right)?,
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
            ResourceProgram::Closure {
                keys,
                source,
                destination,
            } => {
                if keys.is_empty() {
                    resources.closure = graph_resources::prepare_closure(
                        input(inputs, 0)?,
                        keys,
                        *source,
                        *destination,
                    )?;
                } else {
                    resources.partitioned_closure =
                        Some(super::closure_partitions::PartitionedClosure::prepare(
                            input(inputs, 0)?,
                            keys,
                            *source,
                            *destination,
                        )?);
                }
            }
            ResourceProgram::Cycles {
                source,
                destination,
            } => {
                resources.cycles = Some(graph_resources::prepare_cycles(
                    input(inputs, 0)?,
                    *source,
                    *destination,
                )?);
            }
            ResourceProgram::Ordered {
                left_on,
                right_on,
                left_order,
                right_order,
            } => {
                let texts = Texts {
                    symbols: context.symbols.as_ref(),
                    source: context.source,
                    paths: context.paths,
                    arenas: context.arenas,
                };
                resources.ordered = Some(graph_resources::prepare_ordered(
                    input(inputs, 0)?,
                    input(inputs, 1)?,
                    left_on,
                    right_on,
                    *left_order,
                    *right_order,
                    &texts,
                )?);
            }
            ResourceProgram::Dfs(program) => {
                resources.dfs = Some(graph_resources::prepare_dfs(
                    program,
                    input(inputs, 0)?,
                    input(inputs, 1)?,
                    &mut context.symbols,
                    context.source,
                    context.paths,
                    context.arenas,
                )?);
            }
            ResourceProgram::Axis(program) => {
                resources.axis = Some(prepare_axis(program, input(inputs, 1)?, input(inputs, 2)?)?);
            }
        }
        Ok(resources)
    }
    #[cfg(test)]
    pub(crate) fn observe(&self, workspace: &mut super::workspace::InvocationWorkspace) {
        let view = self.index.as_ref().map(IndexResource::view);
        let observed = &mut workspace.observation;
        observed.owners_initialized = usize::from(self.index.is_some())
            + usize::from(self.groups.is_some())
            + usize::from(!self.closure.is_empty() || self.partitioned_closure.is_some())
            + usize::from(self.cycles.is_some())
            + usize::from(self.dfs.is_some())
            + usize::from(self.axis.is_some())
            + usize::from(self.cached_graph.is_some())
            + usize::from(workspace.distinct.cells.capacity() > 0);
        if let Some(view) = view {
            observed.index_rows = view.rows.count();
        }
        let mut rows = self.closure.iter().map(|part| part.table.count.0).sum();
        let mut capacity = self
            .closure
            .iter()
            .map(|part| part.table.cells.capacity() + part.key.capacity() + part.sources.capacity())
            .sum();
        if let Some(parts) = &self.partitioned_closure {
            rows = parts.current().table.count.0;
            capacity = parts.working_capacity();
        }
        if !self.closure.is_empty() || self.partitioned_closure.is_some() {
            capacity += workspace.row.capacity()
                + workspace.other.capacity()
                + workspace.closure.key.capacity()
                + workspace.closure.frontier.capacity()
                + workspace.closure.next.capacity()
                + workspace.closure.reached.capacity()
                + workspace.closure.visited.capacity();
            observed.closure_partition_rows = observed.closure_partition_rows.max(rows);
            observed.closure_partition_capacity = observed.closure_partition_capacity.max(capacity);
        }
        if let Some(dfs) = &self.dfs {
            observed.dfs_adjacency_capacity = dfs.layout.adjacency.capacity();
            observed.dfs_adjacency_len = dfs.layout.adjacency.len();
        }
    }
}

fn groups(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    distinct: Option<ColumnIndex>,
    digest: Option<ColumnIndex>,
    context: &OperatorContext<'_>,
) -> Result<Vec<Group>, OperatingError> {
    let table = materialized(input);
    let mut groups = Vec::new();
    let mut order = KeyIndex::default();
    let mut values = Vec::new();
    let mut room = CountRoom {
        order: &mut order,
        groups: &mut groups,
        values: &mut values,
    };
    if let Some(over) = digest {
        let texts = Texts {
            symbols: context.symbols.as_ref(),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
        };
        digest_groups(
            &table,
            keys,
            over,
            &texts,
            &mut room,
            &mut Hashes::default(),
        )?;
    } else {
        count_groups(&table, On::Columns(keys), distinct, &mut room)?;
    }
    Ok(groups)
}
