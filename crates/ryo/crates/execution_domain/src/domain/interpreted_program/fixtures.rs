//! The drafted regions the merge cases compare: an axis region per rule and node, its contract, and the linear scan the
//! merge replaced.
use super::{
    fusion::types::RegionSharing,
    merge::service::merge_equivalent,
    types::{
        DraftIndex, DraftRegion, LogicalInstruction, RegionMerge,
        demand_program::value::DemandProgram, instruction::value::Instruction,
    },
};
use crate::domain::file_program::types::{NodeInput, ProgramNodeId};
use crate::domain::operator::types::ReaderMultiplicity;
use crate::domain::source_partitions::types::PartitionRequirement;
use crate::domain::{
    operation_schema::types::InputShape, operator::contract::service::prepare_for,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Direction, Input as PlanInput, StepScope},
    plans::types::Operation,
};
use std::borrow::Cow;
use std::sync::Arc;

pub(super) fn axis(id: u32) -> Operation {
    Operation::Axis {
        from: PlanInput::Relation(Relation::Function),
        edges: PlanInput::Relation(Relation::SyntaxChild),
        domain: PlanInput::Relation(Relation::SyntaxTree),
        direction: Direction::Child,
        id: ColumnIndex(id),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    }
}
pub(super) fn contract() -> Arc<crate::domain::operator::contract::types::OperatorContract> {
    let inputs = [
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Node, CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
    ];
    Arc::new(
        prepare_for(
            OperationKind::Axis,
            &axis(0),
            &inputs,
            &[],
            StepScope::PerFile,
        )
        .expect("the axis contract prepares"),
    )
}
pub(super) struct Draft {
    pub(super) id: u32,
    pub(super) partition: PartitionRequirement,
    pub(super) phase: StepScope,
    pub(super) sharing: RegionSharing,
    pub(super) input: NodeInput,
}
pub(super) fn region(
    contract: &Arc<crate::domain::operator::contract::types::OperatorContract>,
    node: usize,
    spec: &Draft,
) -> DraftRegion<Instruction> {
    DraftRegion {
        phase: spec.phase,
        nodes: vec![LogicalInstruction(node)],
        instructions: vec![Instruction::new(
            LogicalInstruction(node),
            crate::domain::coverage::types::RuleIndex(node),
            axis(spec.id),
            Arc::clone(contract),
            DemandProgram::compiled(
                contract.semantics,
                ryo_values::domain::plan_parts::types::InputCount(1),
            ),
            spec.partition,
        )],
        inputs: vec![spec.input],
        needs: Ok(Vec::new()),
        sharing: spec.sharing,
    }
}
/// One leaf region and one region reading it per rule; `variant` picks what the rule's regions
/// repeat (base), nearly repeat (one field differs) or never repeat.
pub(super) fn spec(rule: usize, variant: usize, leaf: bool) -> Draft {
    let node = rule * 2;
    let base = Draft {
        id: 0,
        partition: PartitionRequirement::InputLineage,
        phase: StepScope::PerFile,
        sharing: RegionSharing::Shareable,
        input: if leaf {
            NodeInput::Relation(Relation::Function)
        } else {
            NodeInput::Node(ProgramNodeId(node))
        },
    };
    match variant {
        0 | 1 => base,
        2 => Draft {
            id: u32::try_from(rule + 1).expect("small"),
            ..base
        },
        3 => Draft {
            partition: PartitionRequirement::CurrentFile,
            ..base
        },
        4 => Draft {
            phase: StepScope::CrossFile,
            ..base
        },
        5 if leaf => Draft {
            input: NodeInput::Relation(Relation::SyntaxChild),
            ..base
        },
        _ if leaf => Draft {
            sharing: RegionSharing::RuleScoped,
            ..base
        },
        _ => base,
    }
}
pub(super) struct State {
    pub(super) drafts: Vec<Option<DraftRegion<Instruction>>>,
    pub(super) mapping: Vec<DraftIndex>,
    pub(super) representative: Vec<LogicalInstruction>,
    pub(super) readers: Vec<ReaderMultiplicity>,
}
pub(super) fn state(rules: usize, variant: impl Fn(usize) -> usize) -> State {
    let contract = contract();
    let mut drafts = Vec::new();
    for rule in 0..rules {
        let variant = variant(rule);
        drafts.push(Some(region(
            &contract,
            rule * 2,
            &spec(rule, variant, true),
        )));
        drafts.push(Some(region(
            &contract,
            rule * 2 + 1,
            &spec(rule, variant, false),
        )));
    }
    let count = drafts.len();
    State {
        drafts,
        mapping: (0..count).map(DraftIndex).collect(),
        representative: (0..count).map(LogicalInstruction).collect(),
        readers: vec![ReaderMultiplicity(1); count],
    }
}
/// The linear scan the merge replaced: every earlier kept region compared in full.
pub(super) fn reference(state: &mut State) -> Vec<DraftIndex> {
    let canonical = |draft: &DraftRegion<Instruction>, mapping: &[DraftIndex]| -> Vec<NodeInput> {
        draft
            .inputs
            .iter()
            .map(|input| match input {
                NodeInput::Node(child) => NodeInput::Node(ProgramNodeId(mapping[child.0].0)),
                other => *other,
            })
            .collect()
    };
    let equivalent = |left: &DraftRegion<Instruction>,
                      right: &DraftRegion<Instruction>,
                      mapping: &[DraftIndex]| {
        left.phase == right.phase
            && left.instructions.len() == right.instructions.len()
            && left
                .instructions
                .iter()
                .zip(&right.instructions)
                .all(|(a, b)| a.operation() == b.operation() && a.partition() == b.partition())
            && canonical(left, mapping) == canonical(right, mapping)
    };
    let mut shared: Vec<DraftIndex> = Vec::new();
    for at in 0..state.drafts.len() {
        let Some(draft) = state.drafts[at].as_ref() else {
            continue;
        };
        if draft.sharing == RegionSharing::RuleScoped {
            continue;
        }
        let twin = shared.iter().copied().find(|DraftIndex(earlier)| {
            state.drafts[*earlier]
                .as_ref()
                .is_some_and(|earlier| equivalent(earlier, draft, &state.mapping))
        });
        let Some(DraftIndex(twin)) = twin else {
            shared.push(DraftIndex(at));
            continue;
        };
        let Some(merged) = state.drafts[at].take() else {
            continue;
        };
        if let Some(kept) = state.drafts[twin].as_ref() {
            for (node, served) in merged.nodes.iter().zip(&kept.nodes) {
                state.representative[node.0] = *served;
            }
        }
        for node in &merged.nodes {
            state.mapping[node.0] = DraftIndex(twin);
        }
        for input in &merged.inputs {
            if let NodeInput::Node(child) = input {
                let DraftIndex(producer) = state.mapping[child.0];
                state.readers[producer].0 = state.readers[producer].0.saturating_sub(1);
            }
        }
        state.readers[twin].0 += state.readers[at].0;
    }
    shared
}
pub(super) fn merged(state: &mut State) -> RegionMerge {
    merge_equivalent(
        &mut state.drafts,
        &mut state.mapping,
        &mut state.representative,
        &mut state.readers,
    )
}

/// The region merge beside the linear scan: the kept regions of each, the drafts, the drafts left,
/// and whether its mapping, representatives and readers equal the scan's.
#[derive(Debug, PartialEq)]
pub(super) struct Merge {
    pub(super) kept: usize,
    pub(super) kept_as_the_scan: bool,
    pub(super) drafts: usize,
    pub(super) drafts_left: usize,
    pub(super) drafts_left_as_the_scan: bool,
    pub(super) mapping_as_the_scan: bool,
    pub(super) representatives_as_the_scan: bool,
    pub(super) readers_as_the_scan: bool,
}

/// The comparisons at each rule count, and whether each is under three times the one before plus 16.
#[derive(Debug, PartialEq)]
pub(super) struct Cost {
    pub(super) comparisons: Cow<'static, [usize]>,
    pub(super) linear: bool,
}
pub(super) fn merge_beside_the_linear_scan() -> Merge {
    let mixed = |rule: usize| rule % 7;
    let mut fast = state(40, mixed);
    let mut slow = state(40, mixed);
    let shared = merged(&mut fast).shared;
    let live = |state: &State| state.drafts.iter().map(Option::is_some).collect::<Vec<_>>();
    Merge {
        kept: shared.len(),
        kept_as_the_scan: shared == reference(&mut slow),
        drafts: fast.drafts.len(),
        drafts_left: fast.drafts.iter().flatten().count(),
        drafts_left_as_the_scan: live(&fast) == live(&slow),
        mapping_as_the_scan: fast.mapping == slow.mapping,
        representatives_as_the_scan: fast.representative == slow.representative,
        readers_as_the_scan: fast.readers == slow.readers,
    }
}
fn comparisons(rules: usize) -> usize {
    // Even rules repeat rule 0; odd rules are distinct from every other rule.
    let mut state = state(rules, |rule| if rule % 2 == 0 { 0 } else { 2 });
    merged(&mut state).comparisons.0
}
pub(super) fn merge_cost() -> Cost {
    let counts: Vec<usize> = [50, 100, 200, 400].into_iter().map(comparisons).collect();
    Cost {
        linear: counts.windows(2).all(|pair| pair[1] < 3 * pair[0] + 16),
        comparisons: Cow::Owned(counts),
    }
}
