//! The service keeps the generic route when no family offers one, equivalent drafted regions merge as the linear scan
//! would in linear cost, and demand compilation preserves declared order.
use super::{
    fusion::types::RegionSharing,
    merge::merge_equivalent,
    service::{choose, column, native_positions, ordered_existence, selection},
    types::{
        DemandAction, DraftIndex, DraftRegion, LogicalInstruction, OperandSource,
        PartitionRequirement, ReaderMultiplicity, RegionMerge, Route, RouteOffers, demand_program::value::DemandProgram,
        instruction::value::Instruction,
    },
};
use crate::domain::file_program::types::{ProgramNodeId, NodeInput};
use std::sync::Arc;
use crate::domain::{
    operation_schema::types::InputShape, operator::contract::service::prepare_for,
    operator::contract::types::DemandOrder,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Direction, Input as PlanInput, StepScope},
        plans::types::{Operation},
    },
};
use ryo_values::domain::query_operators::types::OperationKind;
/// What a case checks: the demand program of an order, or the region merge.
#[derive(Clone, Copy, Debug)]
enum Check {
    Demand(DemandOrder, usize, &'static [usize]),
    MergeMatchesLinearScan,
    MergeCostIsLinear,
    NoOfferKeepsTheGenericRoute,
}
type Input = Check;
type Value = ();
type Failure = crate::domain::operating::errors::OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_inputs"),
        word: Word::Succeeds,
        input: || Check::Demand(DemandOrder::None, 0, &[]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unary"),
        word: Word::Succeeds,
        input: || Check::Demand(DemandOrder::Unary, 1, &[0]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_ordered"),
        word: Word::Succeeds,
        input: || Check::Demand(DemandOrder::LeftThenRight, 0, &[]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one_ordered"),
        word: Word::Succeeds,
        input: || Check::Demand(DemandOrder::LeftThenRight, 1, &[0]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("two_ordered"),
        word: Word::Succeeds,
        input: || Check::Demand(DemandOrder::LeftThenRight, 2, &[0, 1]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("three_ordered"),
        word: Word::Succeeds,
        input: || Check::Demand(DemandOrder::LeftThenRight, 3, &[0, 1, 2]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("merge_matches_the_linear_scan"),
        word: Word::Succeeds,
        input: || Check::MergeMatchesLinearScan,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("regions_merge_cost_is_linear_in_rule_count"),
        word: Word::Succeeds,
        input: || Check::MergeCostIsLinear,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_region_no_family_offers_a_route_keeps_the_generic_route"),
        word: Word::Succeeds,
        input: || Check::NoOfferKeepsTheGenericRoute,
        expect: Expect::Value(()),
    },
];
fn subject(check: Input) -> Result<Value, Failure> {
    match check {
        Check::Demand(order, count, expected) => demand_program(order, count, expected),
        Check::MergeMatchesLinearScan => {
            merge_matches_the_linear_scan();
            Ok(())
        }
        Check::MergeCostIsLinear => {
            regions_merge_cost_is_linear_in_rule_count();
            Ok(())
        }
        Check::NoOfferKeepsTheGenericRoute => {
            let route = choose(RouteOffers::default());
            assert!(matches!(route, Route::Generic));
            assert!(ordered_existence(&route).is_none());
            assert!(column(&route).is_none());
            assert!(selection(&route).is_none());
            assert!(native_positions(&route).is_none());
            Ok(())
        }
    }
}
fn demand_program(order: DemandOrder, count: usize, expected: &[usize]) -> Result<Value, Failure> {
    let operation = axis(0);
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
    let mut contract = prepare_for(OperationKind::Axis, &operation, &inputs, &[])?;
    assert_eq!(contract.semantics.demand, DemandOrder::LeftThenRight);
    contract.semantics.demand = order;
    let program = DemandProgram::compiled(contract.semantics, ryo_values::domain::plan_parts::types::InputCount(count));
    let actual = program
        .actions()
        .iter()
        .map(|action| match action {
            DemandAction::Admit(input) => input.0,
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn axis(id: u32) -> Operation {
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
fn contract() -> Arc<crate::domain::operator::contract::types::OperatorContract> {
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
        prepare_for(OperationKind::Axis, &axis(0), &inputs, &[])
            .expect("the axis contract prepares"),
    )
}
struct Draft {
    id: u32,
    partition: PartitionRequirement,
    phase: StepScope,
    sharing: RegionSharing,
    input: NodeInput,
}
fn region(contract: &Arc<crate::domain::operator::contract::types::OperatorContract>, node: usize, spec: &Draft) -> DraftRegion<Instruction> {
    DraftRegion {
        phase: spec.phase,
        nodes: vec![LogicalInstruction(node)],
        instructions: vec![Instruction::new(
            LogicalInstruction(node),
            crate::domain::coverage::types::RuleIndex(node),
            axis(spec.id),
            Arc::clone(contract),
            Box::new([OperandSource::PreviousRow]),
            DemandProgram::compiled(contract.semantics, ryo_values::domain::plan_parts::types::InputCount(1)),
            spec.partition,
        )],
        inputs: vec![spec.input],
        needs: Ok(Vec::new()),
        sharing: spec.sharing,
    }
}
/// One leaf region and one region reading it per rule; `variant` picks what the rule's regions
/// repeat (base), nearly repeat (one field differs) or never repeat.
fn spec(rule: usize, variant: usize, leaf: bool) -> Draft {
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
        2 => Draft { id: u32::try_from(rule + 1).expect("small"), ..base },
        3 => Draft { partition: PartitionRequirement::CurrentFile, ..base },
        4 => Draft { phase: StepScope::CrossFile, ..base },
        5 if leaf => Draft { input: NodeInput::Relation(Relation::SyntaxChild), ..base },
        _ if leaf => Draft { sharing: RegionSharing::RuleScoped, ..base },
        _ => base,
    }
}
struct State {
    drafts: Vec<Option<DraftRegion<Instruction>>>,
    mapping: Vec<DraftIndex>,
    representative: Vec<LogicalInstruction>,
    readers: Vec<ReaderMultiplicity>,
}
fn state(rules: usize, variant: impl Fn(usize) -> usize) -> State {
    let contract = contract();
    let mut drafts = Vec::new();
    for rule in 0..rules {
        let variant = variant(rule);
        drafts.push(Some(region(&contract, rule * 2, &spec(rule, variant, true))));
        drafts.push(Some(region(&contract, rule * 2 + 1, &spec(rule, variant, false))));
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
fn reference(state: &mut State) -> Vec<DraftIndex> {
    let canonical = |draft: &DraftRegion<Instruction>, mapping: &[DraftIndex]| -> Vec<NodeInput> {
        draft
            .inputs
            .iter()
            .map(|input| match input {
                NodeInput::Node(child) => NodeInput::Node(NodeId(mapping[child.0].0)),
                other => *other,
            })
            .collect()
    };
    let equivalent = |left: &DraftRegion<Instruction>, right: &DraftRegion<Instruction>, mapping: &[DraftIndex]| {
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
fn merged(state: &mut State) -> RegionMerge {
    merge_equivalent(
        &mut state.drafts,
        &mut state.mapping,
        &mut state.representative,
        &mut state.readers,
    )
}
fn merge_matches_the_linear_scan() {
    let mixed = |rule: usize| rule % 7;
    let mut fast = state(40, mixed);
    let mut slow = state(40, mixed);
    let shared = merged(&mut fast).shared;
    assert_eq!(shared, reference(&mut slow));
    assert!(shared.len() < fast.drafts.len(), "duplicates merged");
    assert!(fast.drafts.iter().any(Option::is_none), "some regions merged away");
    assert_eq!(fast.mapping, slow.mapping);
    assert_eq!(fast.representative, slow.representative);
    assert_eq!(fast.readers, slow.readers);
    assert_eq!(
        fast.drafts.iter().map(Option::is_some).collect::<Vec<_>>(),
        slow.drafts.iter().map(Option::is_some).collect::<Vec<_>>()
    );
}
fn comparisons(rules: usize) -> usize {
    // Even rules repeat rule 0; odd rules are distinct from every other rule.
    let mut state = state(rules, |rule| if rule % 2 == 0 { 0 } else { 2 });
    merged(&mut state).comparisons.0
}
fn regions_merge_cost_is_linear_in_rule_count() {
    let counts: Vec<usize> = [50, 100, 200, 400].into_iter().map(comparisons).collect();
    for pair in counts.windows(2) {
        assert!(pair[1] < 3 * pair[0] + 16, "comparisons grew faster than linearly: {counts:?}");
    }
    assert!(counts[3] < 3 * counts[2] + 16, "comparisons grew faster than linearly: {counts:?}");
}
