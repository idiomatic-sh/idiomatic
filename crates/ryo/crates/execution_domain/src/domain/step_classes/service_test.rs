//! The `step_classes` service's cases: the merge makes one class of the steps that select the same elements.

use super::service::merge;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Function, Input as PlanInput, LiteralText, Operand, ParameterId,
    PlanComparison, QueryLiteral, StepId, Test, WholeNumber,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The kinds the one step of each rule selects, an empty one for the elements with no text.
type Input = Scenario;
/// How many classes the merge made, and the class of each rule's step.
type Value = &'static str;
type Failure = ();

fn text_is(text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

fn kind_is(name: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

/// A plan whose one step selects the elements of the kind `name` from the tree, or, when `name` is empty, those with no text.
fn plan(name: &str) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
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

#[derive(Clone, Copy)]
enum Scenario {
    Kinds(&'static [&'static str]),
    Check(fn()),
}
fn subject(scenario: Input) -> Value {
    let kinds = match scenario {
        Scenario::Kinds(kinds) => kinds,
        Scenario::Check(check) => {
            check();
            return "held";
        }
    };
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

fn comparison_origin_does_not_split_shared_step_classes() {
    use ryo_values::domain::query_tokens::types::{
        ByteOffset, QueryOrigin, QuerySourceId, QuerySpan,
    };

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
        comparisons[0].origin = ryo_values::domain::plan_parts::types::ComparisonOrigin::Recorded(
            ryo_values::domain::plan_parts::types::ComparisonOriginId(0),
        );
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
        vec![ParameterBinding::Number(WholeNumber(999)), text("word"), text("unused-b")],
    ];
    assert_eq!(classes(&pair, &equal), 1);
    let changed = [
        vec![text("same-unused"), text("word")],
        vec![text("same-unused"), text("other")],
    ];
    assert_eq!(classes(&pair, &changed), 2);
    let wrong_type = [
        vec![text("same-unused"), text("1")],
        vec![text("same-unused"), ParameterBinding::Number(WholeNumber(1))],
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
        assert_eq!(classes(&[selected.clone(), selected.clone()], &bindings), expected);
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

fn relation_identity_and_wide_source_projections_are_preserved() {
    let mut wide = plan("word");
    let Operation::Select {
        from,
        comparisons,
        kept,
    } = &mut wide.steps[0].operation
    else {
        panic!("the fixture builds a Select");
    };
    *from = PlanInput::Relation(Relation::SourceLocation);
    comparisons[0] = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(8)),
        right: Operand::Literal(QueryLiteral::Number(
            ryo_relations::domain::relations::types::Integer(7),
        )),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    *kept = vec![ColumnIndex(8), ColumnIndex(0), ColumnIndex(8)];
    wide.steps.push(PlanStep {
        operation: Operation::Select {
            from: PlanInput::Step(StepId(0)),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(2)),
                right: Operand::Column(ColumnIndex(0)),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            kept: vec![ColumnIndex(1)],
        },
        scope: StepScope::PerFile,
    });
    let mut semantic = plan("word");
    let Operation::Select { from, .. } = &mut semantic.steps[0].operation else {
        panic!("the fixture builds a Select");
    };
    *from = PlanInput::Relation(Relation::SemanticSyntax);
    let merged = merge(&[plan("word"), semantic, wide], &vec![vec![]; 3]);
    assert_eq!(merged.classes.len(), 4);
    assert_eq!(merged.classes[0].source, Relation::SyntaxTree);
    assert_eq!(merged.classes[1].source, Relation::SemanticSyntax);
    assert_eq!(merged.classes[3].source, Relation::SourceLocation);
    assert_eq!(
        merged.classes[3].comparisons[1].left,
        Operand::Column(ColumnIndex(8))
    );
    assert_eq!(
        merged.classes[3].comparisons[1].right,
        Operand::Column(ColumnIndex(8))
    );
}

/// A project step and a file step over one relation select the rows of different partitions: two
/// classes, and a project step that reads a file step's frontier joins no class of its own.
fn a_class_never_spans_two_partitions() {
    let mut project = plan("word");
    project.steps[0].scope = StepScope::CrossFile;
    let merged = merge(&[plan("word"), project.clone(), project], &vec![vec![]; 3]);
    assert_eq!(merged.classes.len(), 2);
    assert_eq!(merged.classes[0].scope, StepScope::PerFile);
    assert_eq!(merged.classes[1].scope, StepScope::CrossFile);
    assert_eq!(merged.classes[1].members.len(), 2);
    let mut frontier = plan("word");
    frontier.steps.push(PlanStep {
        operation: Operation::Select {
            from: PlanInput::Step(StepId(0)),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::CrossFile,
    });
    let merged = merge(&[frontier], &[Vec::new()]);
    assert_eq!(
        merged.class_of[0],
        vec![Some(crate::domain::step_classes::types::ClassPlace(0)), None]
    );
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rules_that_select_the_same_elements_are_one_class"),
        word: Word::Succeeds,
        input: || Scenario::Kinds(&["word", "other", "word", ""]),
        expect: Expect::Value("3 classes, rules [Some(0), Some(1), Some(0), Some(2)]"),
    },
    Case {
        name: CaseName("rules_that_select_different_elements_are_classes_apart"),
        word: Word::Succeeds,
        input: || Scenario::Kinds(&["word", "other"]),
        expect: Expect::Value("2 classes, rules [Some(0), Some(1)]"),
    },
    Case {
        name: CaseName("comparison_origin_does_not_split_shared_step_classes"),
        word: Word::Succeeds,
        input: || Scenario::Check(comparison_origin_does_not_split_shared_step_classes),
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("parameter_free_predicates_share_across_unrelated_typed_binding_vectors"),
        word: Word::Succeeds,
        input: || {
            Scenario::Check(parameter_free_predicates_share_across_unrelated_typed_binding_vectors)
        },
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName(
            "only_referenced_slots_must_match_including_left_operands_and_duplicate_reads",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::Check(
                only_referenced_slots_must_match_including_left_operands_and_duplicate_reads,
            )
        },
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("nested_function_parameters_are_referenced_but_unread_slots_are_not"),
        word: Word::Succeeds,
        input: || {
            Scenario::Check(nested_function_parameters_are_referenced_but_unread_slots_are_not)
        },
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("missing_vectors_or_referenced_slots_never_become_equal_values"),
        word: Word::Succeeds,
        input: || Scenario::Check(missing_vectors_or_referenced_slots_never_become_equal_values),
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("relation_identity_and_wide_source_projections_are_preserved"),
        word: Word::Succeeds,
        input: || Scenario::Check(relation_identity_and_wide_source_projections_are_preserved),
        expect: Expect::Value("held"),
    },
    Case {
        name: CaseName("a_class_never_spans_two_partitions"),
        word: Word::Succeeds,
        input: || Scenario::Check(a_class_never_spans_two_partitions),
        expect: Expect::Value("held"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
