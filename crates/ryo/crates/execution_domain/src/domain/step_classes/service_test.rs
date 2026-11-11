//! The `step_classes` service's cases: the merge makes one class of the steps that select the same elements.

use std::borrow::Cow;

use super::service::merge;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, LiteralText, Operand, PlanComparison, QueryLiteral, StepId,
    Test,
};
use ryo_values::domain::plan_parts::types::{RepeatedRows, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// The plans of the rules merged.
type Input = Vec<Plan>;

/// A class: the relation it reads, its scope, how many steps are its members, and of each of its comparisons the columns its
/// left and right read (none for an operand that is no column).
type Class = (
    Relation,
    StepScope,
    usize,
    Cow<'static, [(Option<u32>, Option<u32>)]>,
);

/// The classes the merge made, and the class of each step of each rule.
#[derive(Debug, PartialEq)]
struct Merged {
    classes: Cow<'static, [Class]>,
    rules: Cow<'static, [Cow<'static, [Option<usize>]>]>,
}

type Value = Merged;
type Failure = std::convert::Infallible;

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
pub(super) fn plan(name: &str) -> Plan {
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

/// A plan whose one step projects the tree's first column, keeping or dropping the rows that repeat.
fn projection(distinct: RepeatedRows) -> Plan {
    let mut projected = plan("word");
    projected.steps[0].operation = Operation::Project {
        from: PlanInput::Relation(Relation::SyntaxTree),
        columns: vec![ColumnIndex(0)],
        distinct,
    };
    projected
}
/// The column an operand reads, if it is a column.
fn column(operand: &Operand) -> Option<u32> {
    match operand {
        Operand::Column(index) => Some(index.0),
        _ => None,
    }
}

fn subject(plans: Input) -> Result<Value, Failure> {
    let bindings = vec![Vec::new(); plans.len()];
    let merged = merge(&plans, &bindings);
    Ok(Merged {
        classes: merged
            .classes
            .iter()
            .map(|class| {
                let compared = class
                    .comparisons
                    .iter()
                    .map(|each| (column(&each.left), column(&each.right)))
                    .collect();
                (class.source, class.scope, class.members.len(), compared)
            })
            .collect(),
        rules: merged
            .class_of
            .iter()
            .map(|steps| steps.iter().map(|each| each.map(|place| place.0)).collect())
            .collect(),
    })
}

/// The rules whose one step selects the elements of these kinds.
fn kinds(names: &[&str]) -> Input {
    names.iter().map(|name| plan(name)).collect()
}

/// Two rules selecting `word`, their comparisons recorded at two sources.
fn two_origins() -> Input {
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

    vec![first, second]
}

/// A rule selecting `word` of the tree, one of `semantic_syntax`, and one selecting `source_location` rows by column 8 keeping
/// columns 8, 0 and 8, then the rows whose third kept column equals the first.
fn wide_sources() -> Input {
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
    vec![plan("word"), semantic, wide]
}

/// A project step and a file step over one relation select the rows of different partitions: two
/// classes, and a project step that reads a file step's frontier joins no class of its own.
fn partitions() -> Input {
    let mut project = plan("word");
    project.steps[0].scope = StepScope::CrossFile;
    vec![plan("word"), project.clone(), project]
}

/// A rule selecting `word`, then a project step reading that file step's frontier.
fn frontier() -> Input {
    let mut frontier = plan("word");
    frontier.steps.push(PlanStep {
        operation: Operation::Select {
            from: PlanInput::Step(StepId(0)),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::CrossFile,
    });
    vec![frontier]
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rules_that_select_the_same_elements_are_one_class"),
        word: Word::Succeeds,
        input: || kinds(&["word", "other", "word", ""]),
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    2,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(2), None)]),
                ),
            ]),
            rules: Cow::Borrowed(&[
                Cow::Borrowed(&[Some(0)]),
                Cow::Borrowed(&[Some(1)]),
                Cow::Borrowed(&[Some(0)]),
                Cow::Borrowed(&[Some(2)]),
            ]),
        }),
    },
    Case {
        name: CaseName("rules_that_select_different_elements_are_classes_apart"),
        word: Word::Succeeds,
        input: || kinds(&["word", "other"]),
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
            ]),
            rules: Cow::Borrowed(&[Cow::Borrowed(&[Some(0)]), Cow::Borrowed(&[Some(1)])]),
        }),
    },
    Case {
        name: CaseName("a_row_dropping_projection_is_not_a_pure_projection"),
        word: Word::Succeeds,
        input: || {
            vec![
                projection(RepeatedRows::Kept),
                projection(RepeatedRows::Dropped),
            ]
        },
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[(
                Relation::SyntaxTree,
                StepScope::PerFile,
                1,
                Cow::Borrowed(&[]),
            )]),
            rules: Cow::Borrowed(&[Cow::Borrowed(&[Some(0)]), Cow::Borrowed(&[None])]),
        }),
    },
    Case {
        name: CaseName("comparison_origin_does_not_split_shared_step_classes"),
        word: Word::Succeeds,
        input: two_origins,
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[(
                Relation::SyntaxTree,
                StepScope::PerFile,
                2,
                Cow::Borrowed(&[(Some(1), None)]),
            )]),
            rules: Cow::Borrowed(&[Cow::Borrowed(&[Some(0)]), Cow::Borrowed(&[Some(0)])]),
        }),
    },
    Case {
        name: CaseName("relation_identity_and_wide_source_projections_are_preserved"),
        word: Word::Succeeds,
        input: wide_sources,
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
                (
                    Relation::SemanticSyntax,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
                (
                    Relation::SourceLocation,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(8), None)]),
                ),
                (
                    Relation::SourceLocation,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(8), None), (Some(8), Some(8))]),
                ),
            ]),
            rules: Cow::Borrowed(&[
                Cow::Borrowed(&[Some(0)]),
                Cow::Borrowed(&[Some(1)]),
                Cow::Borrowed(&[Some(2), Some(3)]),
            ]),
        }),
    },
    Case {
        name: CaseName("a_class_never_spans_two_partitions"),
        word: Word::Succeeds,
        input: partitions,
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[
                (
                    Relation::SyntaxTree,
                    StepScope::PerFile,
                    1,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
                (
                    Relation::SyntaxTree,
                    StepScope::CrossFile,
                    2,
                    Cow::Borrowed(&[(Some(1), None)]),
                ),
            ]),
            rules: Cow::Borrowed(&[
                Cow::Borrowed(&[Some(0)]),
                Cow::Borrowed(&[Some(1)]),
                Cow::Borrowed(&[Some(1)]),
            ]),
        }),
    },
    Case {
        name: CaseName("a_project_step_reading_a_file_steps_frontier_joins_no_class"),
        word: Word::Succeeds,
        input: frontier,
        expect: Expect::Value(Merged {
            classes: Cow::Borrowed(&[(
                Relation::SyntaxTree,
                StepScope::PerFile,
                1,
                Cow::Borrowed(&[(Some(1), None)]),
            )]),
            rules: Cow::Borrowed(&[Cow::Borrowed(&[Some(0), None])]),
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod bindings_test;
