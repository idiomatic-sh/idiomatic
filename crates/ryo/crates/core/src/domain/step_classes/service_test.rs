//! The `step_classes` service's cases: the merge makes one class of the steps that select the same elements.

use super::service::merge;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Comparison, Function, Input as PlanInput, LiteralText, Operand,
    ParameterId, QueryLiteral, StepId, Test, WholeNumber,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};

/// The kinds the one step of each rule selects, an empty one for the elements with no text.
type Input = &'static [&'static str];
/// How many classes the merge made, and the class of each rule's step.
type Value = &'static str;
type Failure = ();

fn text_is(text: &str) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: None,
    }
}

fn kind_is(name: &str) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
        origin: None,
    }
}

/// A plan whose one step selects the elements of the kind `name` from the tree, or, when `name` is empty, those with no text.
fn plan(name: &str) -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons: vec![if name.is_empty() {
                    text_is("")
                } else {
                    kind_is(name)
                }],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn subject(kinds: Input) -> Value {
    let plans = kinds.iter().map(|name| plan(name)).collect::<Vec<_>>();
    let bindings = vec![Vec::new(); plans.len()];
    let merged = merge(&plans, &bindings);
    Box::leak(
        format!(
            "{} classes, rules {:?}",
            merged.classes.len(),
            merged
                .class_of
                .iter()
                .map(|each| each[0].map(|place| place.0))
                .collect::<Vec<_>>()
        )
        .into_boxed_str(),
    )
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rules_that_select_the_same_elements_are_one_class"),
        word: Word::Succeeds,
        input: || &["word", "other", "word", ""],
        expect: Expect::Value("3 classes, rules [Some(0), Some(1), Some(0), Some(2)]"),
    },
    Case {
        name: CaseName("rules_that_select_different_elements_are_classes_apart"),
        word: Word::Succeeds,
        input: || &["word", "other"],
        expect: Expect::Value("2 classes, rules [Some(0), Some(1)]"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}

#[test]
fn comparison_origin_does_not_split_shared_step_classes() {
    use ryo_values::domain::plan_parts::types::ComparisonOriginId;
    use ryo_values::domain::program::types::{ByteOffset, QueryOrigin, QuerySourceId, QuerySpan};

    let mut first = plan("word");
    let mut second = plan("word");
    for (plan, source) in [(&mut first, "first"), (&mut second, "second")] {
        plan.comparison_origins.push(QueryOrigin {
            source_id: QuerySourceId(source.into()),
            span: QuerySpan {
                start: ByteOffset(0),
                end: ByteOffset(4),
            },
            trace: Vec::new(),
        });
        let Operation::Select { comparisons, .. } = &mut plan.steps[0].operation else {
            panic!("the fixture has one select");
        };
        comparisons[0].origin = Some(ComparisonOriginId(0));
    }

    let merged = merge(&[first, second], &[Vec::new(), Vec::new()]);
    assert_eq!(merged.classes.len(), 1);
    assert_eq!(merged.classes[0].members.len(), 2);
}

/// Keep the original predicate order and source origins while exercising parameter references.
fn with_operands(left: Operand, right: Operand) -> Plan {
    let mut selected = plan("word");
    let Operation::Select { comparisons, .. } = &mut selected.steps[0].operation else {
        panic!("fixture is a selection");
    };
    comparisons.push(Comparison {
        test: Test::Equal,
        left,
        right,
        origin: None,
    });
    selected
}

fn text(value: &str) -> ParameterBinding {
    ParameterBinding::Text(LiteralText(value.into()))
}

#[test]
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

#[test]
fn only_referenced_slots_must_match_including_left_operands_and_duplicate_reads() {
    let selected = with_operands(
        Operand::Parameter(ParameterId(1)),
        Operand::Parameter(ParameterId(1)),
    );
    let equal = merge(
        &[selected.clone(), selected.clone()],
        &[
            vec![text("unused-a"), text("word")],
            vec![
                ParameterBinding::Number(WholeNumber(999)),
                text("word"),
                text("unused-b"),
            ],
        ],
    );
    assert_eq!(equal.classes.len(), 1);
    let changed = merge(
        &[selected.clone(), selected.clone()],
        &[
            vec![text("same-unused"), text("word")],
            vec![text("same-unused"), text("other")],
        ],
    );
    assert_eq!(changed.classes.len(), 2);
    let wrong_type = merge(
        &[selected.clone(), selected],
        &[
            vec![text("same-unused"), text("1")],
            vec![
                text("same-unused"),
                ParameterBinding::Number(WholeNumber(1)),
            ],
        ],
    );
    assert_eq!(wrong_type.classes.len(), 2);
}

#[test]
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
        let merged = merge(
            &[selected.clone(), selected.clone()],
            &[
                vec![text("unused-a"), text("word")],
                vec![text("unused-b"), second],
            ],
        );
        assert_eq!(merged.classes.len(), expected);
    }
    assert_eq!(
        merge(
            &[selected.clone(), selected],
            &[vec![text("unused-a")], vec![text("unused-b")]]
        )
        .classes
        .len(),
        2
    );
}

#[test]
fn missing_vectors_or_referenced_slots_never_become_equal_values() {
    let empty = plan("word");
    assert_eq!(
        merge(&[empty.clone(), empty.clone()], &[vec![]])
            .classes
            .len(),
        2
    );
    assert_eq!(merge(&[empty.clone(), empty], &[]).classes.len(), 2);
    for id in [0, 1, u32::MAX] {
        let selected = with_operands(
            Operand::Parameter(ParameterId(id)),
            Operand::Column(ColumnIndex(2)),
        );
        assert_eq!(
            merge(&[selected.clone(), selected.clone()], &[vec![], vec![]])
                .classes
                .len(),
            2
        );
        assert_eq!(
            merge(&[selected.clone(), selected], &[vec![], vec![text("word")]])
                .classes
                .len(),
            2
        );
    }
}
