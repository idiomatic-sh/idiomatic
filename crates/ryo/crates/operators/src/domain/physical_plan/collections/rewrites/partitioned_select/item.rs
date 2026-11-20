//! Spike (ryo-fmx3): the `partitioned_select` item of the `rewrites` collection: sibling filters over one source whose first
//! comparison tests the same column for equality with a different constant read one partition of that source, made once per
//! file on the first sibling's demand; each sibling then reads only its bucket. The lowerings are unchanged: the plan answers
//! the same rows, and a run with the partition switched off reads every row as before. It reads the plan after every rewrite
//! (`plan_with`), as `guards` does, so it sees the filters `keyed_select` left.

use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_language::domain::symbols::service::hash_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Operand, PlanComparison, QueryLiteral, Test,
};

/// What a partition's column is compared as: a name's text (keyed by its hash) or a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PartitionKind {
    /// A symbol column against a text: the key is the text's hash, as the filter's specialised comparison reads it.
    Text,
    /// A number-like column against a number: the key is the number.
    Number,
}

/// One partition: the source, the column and how it is keyed, and the slot of each constant its siblings ask for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartitionGroup {
    /// The rows partitioned.
    pub(crate) source: Option<RowSource>,
    /// The column.
    pub(crate) column: ColumnIndex,
    /// How the column is keyed.
    pub(crate) kind: Option<PartitionKind>,
    /// Each asked constant's key, by slot (siblings asking for one constant share it).
    pub(crate) keys: Vec<u64>,
    /// The slot of each asked key.
    pub(crate) slots: hashbrown::HashMap<u64, u32>,
    /// By member: its first comparison alone, which decides the rows no key was read from.
    pub(crate) firsts: Vec<crate::domain::select::types::SelectParts>,
}

/// The plan's partitions and, by node, the partition and slot a sibling reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SelectPartitions {
    /// The partitions, by place.
    pub(crate) groups: Vec<PartitionGroup>,
    /// By node: the partition it reads, its slot in it and its place among the partition's members.
    pub(crate) of_node: Vec<Option<(u32, u32, u32)>>,
}

impl SelectPartitions {
    /// The partition, slot and member place `node` reads, if it is a partitioned sibling.
    pub(crate) fn of(&self, node: PlanNodeId) -> Option<(u32, u32, u32)> {
        self.of_node
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .copied()
            .flatten()
    }
}

/// The key a filter's first comparison asks for: a column equal to a written text or number. Only the first comparison, so
/// a row outside the bucket fails it before any later comparison is tried and raises no fault.
fn first_key(comparisons: &[PlanComparison]) -> Option<(ColumnIndex, PartitionKind, u64)> {
    let first = comparisons.first()?;
    if first.test != Test::Equal {
        return None;
    }
    let Operand::Column(column) = first.left else {
        return None;
    };
    match &first.right {
        Operand::Literal(QueryLiteral::Text(text)) => Some((
            column,
            PartitionKind::Text,
            hash_of(ByteSlice(text.0.as_bytes())).0,
        )),
        Operand::Literal(QueryLiteral::Number(number)) => {
            Some((column, PartitionKind::Number, number.0.cast_unsigned()))
        }
        _ => None,
    }
}

/// The partitions of `nodes`: the filters over one relation, the tree or a held node whose first comparison keys the same
/// column the same way, when two or more of them do.
pub(crate) fn partitions(nodes: &[PhysicalNode]) -> SelectPartitions {
    type Member = (
        PlanNodeId,
        u64,
        Option<PlanComparison>,
        Vec<ryo_values::domain::query_bindings::types::ParameterBinding>,
    );
    let mut found: hashbrown::HashMap<(RowSource, ColumnIndex, PartitionKind), Vec<Member>> =
        hashbrown::HashMap::new();
    let mut order: Vec<(RowSource, ColumnIndex, PartitionKind)> = Vec::new();
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        let Lowering::Filter { source, select } = &node.lowering else {
            continue;
        };
        if !matches!(source, RowSource::Facts(_) | RowSource::Tree(_) | RowSource::Node(_)) {
            continue;
        }
        let Some((column, kind, key)) = first_key(&select.comparisons) else {
            continue;
        };
        let group = (*source, column, kind);
        found
            .entry(group)
            .or_insert_with(|| {
                order.push(group);
                Vec::new()
            })
            .push((id, key, select.comparisons.first().cloned(), select.bindings.clone()));
    }
    let mut partitions = SelectPartitions {
        groups: Vec::new(),
        of_node: vec![None; nodes.len()],
    };
    for group in order {
        let Some(members) = found.get(&group) else {
            continue;
        };
        if members.len() < 2 {
            continue;
        }
        let (source, column, kind) = group;
        let place = u32::try_from(partitions.groups.len()).unwrap_or(u32::MAX);
        let mut made = PartitionGroup {
            source: Some(source),
            column,
            kind: Some(kind),
            keys: Vec::new(),
            slots: hashbrown::HashMap::new(),
            firsts: Vec::new(),
        };
        for (node, key, first, bindings) in members {
            let member = u32::try_from(made.firsts.len()).unwrap_or(u32::MAX);
            made.firsts.push(crate::domain::select::service::lower(
                first.as_slice(),
                &[],
                bindings,
            ));
            let next = u32::try_from(made.keys.len()).unwrap_or(u32::MAX);
            let slot = *made.slots.entry(*key).or_insert(next);
            if slot == next {
                made.keys.push(*key);
            }
            if let Some(entry) = partitions
                .of_node
                .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                *entry = Some((place, slot, member));
            }
        }
        partitions.groups.push(made);
    }
    partitions
}
