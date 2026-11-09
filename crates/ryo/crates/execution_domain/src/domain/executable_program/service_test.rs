//! A region the recogniser did not resolve keeps its registered operators, equivalent drafted regions merge as the linear scan
//! would in linear cost, and demand compilation preserves declared order.
use super::fixtures::{axis, merged, reference, state};
use super::{
    service::ordered_existence,
    types::{DemandAction, Route, demand_program::value::DemandProgram},
};
use crate::domain::{
    operation_schema::types::InputShape, operator::contract::service::prepare_for,
    operator::contract::types::DemandOrder,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, StepScope},
        plans::types::Operation,
    },
};
/// What a case checks: the demand program of an order, or the region merge.
#[derive(Clone, Copy, Debug)]
enum Check {
    Demand(DemandOrder, usize, &'static [usize]),
    MergeMatchesLinearScan,
    MergeCostIsLinear,
    UnresolvedRegionKeepsItsOperators,
    NameWithoutTextFailsTheReplaceGate,
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
        name: CaseName("a_name_without_text_is_refused_by_the_replace_text_gate"),
        word: Word::Succeeds,
        input: || Check::NameWithoutTextFailsTheReplaceGate,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unresolved_region_keeps_its_registered_operators"),
        word: Word::Succeeds,
        input: || Check::UnresolvedRegionKeepsItsOperators,
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
        Check::NameWithoutTextFailsTheReplaceGate => {
            a_name_without_text_is_refused_by_the_replace_text_gate()
        }
        Check::UnresolvedRegionKeepsItsOperators => {
            let route = Route::default();
            assert!(matches!(route, Route::Generic));
            assert!(ordered_existence(&route).is_none());
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
    let mut contract = prepare_for(
        OperationKind::Axis,
        &operation,
        &inputs,
        &[],
        StepScope::PerFile,
    )?;
    assert_eq!(contract.semantics.demand, DemandOrder::LeftThenRight);
    contract.semantics.demand = order;
    let program = DemandProgram::compiled(
        contract.semantics,
        ryo_values::domain::plan_parts::types::InputCount(count),
    );
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

fn merge_matches_the_linear_scan() {
    let mixed = |rule: usize| rule % 7;
    let mut fast = state(40, mixed);
    let mut slow = state(40, mixed);
    let shared = merged(&mut fast).shared;
    assert_eq!(shared, reference(&mut slow));
    assert!(shared.len() < fast.drafts.len(), "duplicates merged");
    assert!(
        fast.drafts.iter().any(Option::is_none),
        "some regions merged away"
    );
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
        assert!(
            pair[1] < 3 * pair[0] + 16,
            "comparisons grew faster than linearly: {counts:?}"
        );
    }
    assert!(
        counts[3] < 3 * counts[2] + 16,
        "comparisons grew faster than linearly: {counts:?}"
    );
}
/// The boundary the Replace declaration rests on: a name that left its file and has no text fails the text gate, so only
/// a path where no such name can arrive may declare Replace over a Symbol column infallible.
fn a_name_without_text_is_refused_by_the_replace_text_gate() -> Result<Value, Failure> {
    let kinds = [CellKind::Symbol];
    let contract = prepare_for(
        OperationKind::Replace,
        &Operation::Replace {
            from: PlanInput::Relation(Relation::Comment),
            text: ColumnIndex(0),
            pattern: ryo_values::domain::plan_parts::types::LiteralText("a".into()),
            replacement: ryo_values::domain::plan_parts::types::LiteralText("b".into()),
        },
        &[InputShape {
            kinds: &kinds,
            tree: None,
        }],
        &[],
        StepScope::PerFile,
    )
    .expect("the replace contract prepares");
    let symbols = ryo_language::domain::symbols::types::Symbols::default();
    let texts = crate::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let row = [ryo_relations::domain::relations::types::Value::Name(
        ryo_relations::domain::relations::types::Name {
            hash: ryo_relations::domain::relations::types::TextHash(7),
            place: ryo_relations::domain::relations::types::PlaceNumber(0),
        },
    )];
    assert_eq!(
        crate::domain::executable_program::kernel::row_admissible(&contract, &row, &texts),
        Err(crate::domain::operating::errors::OperatingError::ShapeMismatch)
    );
    Ok(())
}
