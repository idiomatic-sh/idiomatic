//! How step classes treat parameter bindings: parameter-free predicates share a class, only referenced slots must match, and a missing vector or slot never equals a value.
use super::super::service::merge;
use super::plan;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Function, LiteralText, Operand, ParameterId, PlanComparison,
    QueryLiteral, Test, WholeNumber,
};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// Keep the original predicate order and source origins while exercising parameter references.
fn with_operands(left: Operand, right: Operand) -> Plan {
    let mut selected = plan("word");
    let Operation::Select { comparisons, .. } = &mut selected.steps[0].operation else {
        panic!("fixture is a selection");
    };
    comparisons.push(PlanComparison {
        test: Test::Equal,
        left,
        right,
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    });
    selected
}

/// How many classes the merge makes of the plans bound to the bindings.
fn classes(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> usize {
    merge(plans, bindings).classes.len()
}

fn text(value: &str) -> ParameterBinding {
    ParameterBinding::Text(LiteralText(value.into()))
}

fn parameter_free_predicates_share_across_unrelated_typed_binding_vectors() {
    let plans = [plan("word"), plan("word"), plan("word")];
    let merged = merge(
        &plans,
        &[
            vec![text("["), ParameterBinding::Numbers(vec![WholeNumber(17)])],
            vec![],
            vec![ParameterBinding::Flag(
                ryo_values::domain::plan_parts::types::FlagState::Raised,
            )],
        ],
    );
    assert_eq!(merged.classes.len(), 1);
    assert_eq!(
        merged.classes[0]
            .members
            .iter()
            .map(|member| member.rule.0)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(merged.class_of[0], merged.class_of[1]);
    assert_eq!(merged.class_of[1], merged.class_of[2]);
}

fn only_referenced_slots_must_match_including_left_operands_and_duplicate_reads() {
    let selected = with_operands(
        Operand::Parameter(ParameterId(1)),
        Operand::Parameter(ParameterId(1)),
    );
    let pair = [selected.clone(), selected];
    let equal = [
        vec![text("unused-a"), text("word")],
        vec![
            ParameterBinding::Number(WholeNumber(999)),
            text("word"),
            text("unused-b"),
        ],
    ];
    assert_eq!(classes(&pair, &equal), 1);
    let changed = [
        vec![text("same-unused"), text("word")],
        vec![text("same-unused"), text("other")],
    ];
    assert_eq!(classes(&pair, &changed), 2);
    let wrong_type = [
        vec![text("same-unused"), text("1")],
        vec![
            text("same-unused"),
            ParameterBinding::Number(WholeNumber(1)),
        ],
    ];
    assert_eq!(classes(&pair, &wrong_type), 2);
}

fn nested_function_parameters_are_referenced_but_unread_slots_are_not() {
    let nested = Operand::Function(Applied {
        function: Function::Concat,
        arguments: vec![
            Operand::Literal(QueryLiteral::Text(LiteralText("prefix".into()))),
            Operand::Function(Applied {
                function: Function::Snake,
                arguments: vec![Operand::Parameter(ParameterId(1))],
            }),
        ],
    });
    let selected = with_operands(Operand::Column(ColumnIndex(2)), nested);
    for (second, expected) in [
        (text("word"), 1),
        (text("other"), 2),
        (ParameterBinding::Numbers(vec![]), 2),
    ] {
        let bindings = [
            vec![text("unused-a"), text("word")],
            vec![text("unused-b"), second],
        ];
        assert_eq!(
            classes(&[selected.clone(), selected.clone()], &bindings),
            expected
        );
    }
    let unread = [vec![text("unused-a")], vec![text("unused-b")]];
    assert_eq!(classes(&[selected.clone(), selected], &unread), 2);
}

fn missing_vectors_or_referenced_slots_never_become_equal_values() {
    let empty = plan("word");
    assert_eq!(classes(&[empty.clone(), empty.clone()], &[vec![]]), 2);
    assert_eq!(classes(&[empty.clone(), empty], &[]), 2);
    for id in [0, 1, u32::MAX] {
        let selected = with_operands(
            Operand::Parameter(ParameterId(id)),
            Operand::Column(ColumnIndex(2)),
        );
        let pair = [selected.clone(), selected];
        assert_eq!(classes(&pair, &[vec![], vec![]]), 2);
        assert_eq!(classes(&pair, &[vec![], vec![text("word")]]), 2);
    }
}

type Input = fn();
type Value = &'static str;
type Failure = ();

fn subject(check: Input) -> Value {
    check();
    "held"
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parameter_free_predicates_share_across_unrelated_typed_binding_vectors"),
        word: Word::Succeeds,
        input: || parameter_free_predicates_share_across_unrelated_typed_binding_vectors,
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName(
            "only_referenced_slots_must_match_including_left_operands_and_duplicate_reads",
        ),
        word: Word::Succeeds,
        input: || only_referenced_slots_must_match_including_left_operands_and_duplicate_reads,
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("nested_function_parameters_are_referenced_but_unread_slots_are_not"),
        word: Word::Succeeds,
        input: || nested_function_parameters_are_referenced_but_unread_slots_are_not,
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("missing_vectors_or_referenced_slots_never_become_equal_values"),
        word: Word::Succeeds,
        input: || missing_vectors_or_referenced_slots_never_become_equal_values,
        expect: Expect::Value("held"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
