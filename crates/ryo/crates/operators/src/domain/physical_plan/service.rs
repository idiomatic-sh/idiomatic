//! The planner's one dispatch over the operation kinds, and the `keyed_select` rewrite over what it lowered.
pub(crate) mod filter;
pub(crate) mod index;
pub(crate) mod run;

use crate::domain::comparing::service::fixed_prefix;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    IndexPlace, KeyedIndex, Lowering, NodeReaders, PhysicalNode, PlanInput, PlanNodeId, Retention,
    RowSource, SelectKey, SelectParts, ServedAt, Unserved,
};
use ryo_language::domain::symbols::service::hash_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The physical plan of `program`: each node lowered once, the selects keyed on one relation's column by two or more of them
/// rewritten to lookups of one shared index of it, and every node of a kind of a later stage recorded as unserved.
///
/// # Errors
///
/// `WrongOperation` when a node reads a later node.
pub(crate) fn plan(program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault> {
    program.check_shape()?;
    let logical = program.nodes();
    let mut nodes: Vec<PhysicalNode> = logical
        .iter()
        .map(|node| PhysicalNode {
            lowering: lowered(node.operation, node.inputs.0.first(), node.bindings.0),
            readers: NodeReaders(0),
            retained: Retention::Released,
        })
        .collect();
    let mut unserved = Vec::new();
    for (node, id) in logical.iter().zip((0..).map(PlanNodeId)) {
        for input in node.inputs.0 {
            if let PlanInput::Node(read) = input
                && let Some(reader) = place(&mut nodes, *read)
            {
                reader.readers = NodeReaders(reader.readers.0.saturating_add(1));
            }
        }
        if let Some(until) = stage_of(node.operation, node.inputs.0.first()) {
            unserved.push(Unserved { node: id, until });
        }
    }
    for kept in program.emitted().chain(program.held().iter().copied()) {
        if let Some(node) = place(&mut nodes, kept) {
            node.retained = Retention::Retained;
        }
    }
    let indexes = keyed_selects(&mut nodes);
    Ok(PhysicalPlan {
        nodes,
        indexes,
        unserved,
    })
}

/// The node at `id`, to change.
fn place(nodes: &mut [PhysicalNode], id: PlanNodeId) -> Option<&mut PhysicalNode> {
    nodes.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// What a node reads its rows from, when this stage's kernels read it.
fn source_of(input: Option<&PlanInput>) -> Option<RowSource> {
    match input? {
        PlanInput::Node(node) => Some(RowSource::Node(*node)),
        PlanInput::Relation(relation) => Some(RowSource::Facts(*relation)),
        PlanInput::Bound(_) | PlanInput::Observation(_) => None,
    }
}

/// The kernels of one operation: the planner's one dispatch over the kinds.
fn lowered(
    operation: &Operation,
    input: Option<&PlanInput>,
    bindings: &[ParameterBinding],
) -> Lowering {
    match (operation, source_of(input)) {
        (
            Operation::Select {
                comparisons, kept, ..
            },
            Some(source),
        ) => Lowering::Filter {
            source,
            select: SelectParts {
                comparisons: comparisons.clone(),
                kept: kept.clone(),
                bindings: bindings.to_vec(),
            },
        },
        (
            Operation::Project {
                columns, distinct, ..
            },
            Some(source),
        ) => Lowering::Project {
            source,
            columns: columns.clone(),
            distinct: *distinct,
        },
        _ => Lowering::Unlowered,
    }
}

/// The stage that lowers a node this stage does not, or nothing when it is lowered.
fn stage_of(operation: &Operation, input: Option<&PlanInput>) -> Option<ServedAt> {
    let stage = match operation {
        Operation::Select { .. } | Operation::Project { .. } => {
            return source_of(input).map_or(Some(ServedAt(4)), |_| None);
        }
        Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. } => 1,
        Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => 2,
        Operation::GroupDigest { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. } => 3,
        Operation::Rows { .. } | Operation::Hosted { .. } => 4,
    };
    Some(ServedAt(stage))
}

/// The key a filter of a relation fixes in its infallible prefix: a name column equal to a written, non-empty text. A key
/// after the prefix is no key: a comparison before it may fail, and the lookup would skip the rows it fails on.
fn key_of(source: RowSource, comparisons: &[PlanComparison]) -> Option<SelectKey> {
    let RowSource::Facts(relation) = source else {
        return None;
    };
    let columns = kinds(relation);
    fixed_prefix(comparisons, columns)
        .0
        .iter()
        .find_map(|comparison| {
            let ((Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
            | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column))) =
                (&comparison.left, &comparison.right)
            else {
                return None;
            };
            let named = columns
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .is_some_and(|kind| *kind == CellKind::Symbol);
            (comparison.test == Test::Equal && named && !text.0.is_empty()).then(|| SelectKey {
                relation,
                column: *column,
                key: hash_of(ByteSlice(text.0.as_bytes())),
            })
        })
}

/// The `keyed_select` rewrite: the filters of one relation keyed on one of its columns become lookups of one shared index of
/// it when two or more of them key on it; a single one keeps its scan, which costs less than an index.
fn keyed_selects(nodes: &mut [PhysicalNode]) -> Vec<KeyedIndex> {
    let mut keyed: Vec<KeyedIndex> = Vec::new();
    let mut keys: Vec<Vec<SelectKey>> = Vec::new();
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        let Lowering::Filter { source, select } = &node.lowering else {
            continue;
        };
        let Some(key) = key_of(*source, &select.comparisons) else {
            continue;
        };
        match keyed
            .iter()
            .position(|index| index.relation == key.relation && index.column == key.column)
        {
            Some(at) => {
                if let (Some(index), Some(held)) = (keyed.get_mut(at), keys.get_mut(at)) {
                    index.lookups.push(id);
                    held.push(key);
                }
            }
            None => {
                keyed.push(KeyedIndex {
                    relation: key.relation,
                    column: key.column,
                    lookups: vec![id],
                });
                keys.push(vec![key]);
            }
        }
    }
    let mut indexes = Vec::new();
    for (index, held) in keyed.into_iter().zip(keys) {
        if index.lookups.len() < 2 {
            continue;
        }
        let shared = IndexPlace(indexes.len());
        for (lookup, key) in index.lookups.iter().zip(&held) {
            if let Some(node) = place(nodes, *lookup)
                && let Lowering::Filter { select, .. } =
                    std::mem::replace(&mut node.lowering, Lowering::Unlowered)
            {
                node.lowering = Lowering::Lookup {
                    index: shared,
                    key: key.key,
                    select,
                };
            }
        }
        indexes.push(index);
    }
    indexes
}
