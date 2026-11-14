//! The `features` service's cases, on values alone.

use ryo_relations::domain::relations::types::Relation;

use super::service::{catalog, fed_over, tally};
use crate::domain::operator::types::RowsFed;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::run_metadata::types::Feature;

/// What a row asks of the features of a run.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// How many features the catalog lists.
    Catalog,
    /// How many rows a select with an equality over `function`, fed 1500 rows, was counted as feeding the feature asked for.
    SelectFed(Feature),
    /// The kind of the operation of the one step of the plan.
    KindOfTheStep,
    /// How many rows a feature was fed over runs of plans that each have one select, fed these rows.
    FedOver(Feature, &'static [u64]),
    /// The features told of when no plan ran.
    FedOverNothing,
}

/// What the features of a run answer.
#[derive(Debug, PartialEq)]
enum Answer {
    /// How many features there are.
    Features(usize),
    /// How many rows a feature was fed.
    Rows(u64),
    /// The kind of an operation.
    Kind(OperationKind),
    /// How many features are told of, and whether each was fed no row.
    Told { features: usize, unfed: bool },
}

type Input = Ask;
type Value = Answer;
type Failure = std::convert::Infallible;

/// A plan of one step: a select over `function` that compares for equality.
fn select_over_function() -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::Function),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(2),
            key: vec![ColumnIndex(3)],
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::KindOfTheStep => Answer::Kind(kind_of(&select_over_function().steps[0].operation)),
        Ask::FedOver(wanted, rows) => {
            let plans: Vec<Plan> = rows.iter().map(|_| select_over_function()).collect();
            let fed: Vec<Vec<RowsFed>> = rows
                .iter()
                .map(|each| vec![RowsFed([*each, 0, 0])])
                .collect();
            Answer::Rows(
                fed_over(
                    &ryo_values::domain::run_metadata::service::from_plans(&plans),
                    &fed,
                )
                .into_iter()
                .filter(|each| each.feature == wanted)
                .map(|each| each.rows.0)
                .sum(),
            )
        }
        Ask::FedOverNothing => {
            let told = fed_over(&[], &[]);
            Answer::Told {
                features: told.len(),
                unfed: told.iter().all(|each| each.rows.0 == 0),
            }
        }
        Ask::Catalog => Answer::Features(catalog().len()),
        Ask::SelectFed(wanted) => Answer::Rows(
            tally(
                &ryo_values::domain::run_metadata::service::from_plans(&[select_over_function()])
                    [0],
                &[RowsFed([1500, 0, 0])],
            )
            .into_iter()
            .filter(|each| each.feature == wanted)
            .map(|each| each.rows.0)
            .sum(),
        ),
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_catalog_lists_every_operation_way_of_comparing_direction_and_relation"),
        word: Word::Succeeds,
        input: || Ask::Catalog,
        // The feature catalog includes the internal Hosted operation as well as
        // the 25 public operators. The 72 relations include ProjectNamespaceMembership.
        expect: Expect::Value(Answer::Features(26 + 17 + 8 + 72)),
    },
    Case {
        name: CaseName("a_select_is_fed_the_rows_of_its_input"),
        word: Word::Succeeds,
        input: || Ask::SelectFed(Feature::Operation(OperationKind::Select)),
        expect: Expect::Value(Answer::Rows(1500)),
    },
    Case {
        name: CaseName("the_way_a_select_compares_is_fed_the_same_rows"),
        word: Word::Succeeds,
        input: || Ask::SelectFed(Feature::Family(Test::Equal)),
        expect: Expect::Value(Answer::Rows(1500)),
    },
    Case {
        name: CaseName("the_relation_a_select_reads_is_fed_the_rows_it_read"),
        word: Word::Succeeds,
        input: || Ask::SelectFed(Feature::Relation(Relation::Function)),
        expect: Expect::Value(Answer::Rows(1500)),
    },
    Case {
        name: CaseName("the_rows_a_feature_was_fed_by_every_plan_are_summed"),
        word: Word::Succeeds,
        input: || Ask::FedOver(Feature::Operation(OperationKind::Select), &[1500, 500]),
        expect: Expect::Value(Answer::Rows(2000)),
    },
    Case {
        name: CaseName("the_rows_of_a_relation_are_summed_over_the_plans_that_read_it"),
        word: Word::Succeeds,
        input: || Ask::FedOver(Feature::Relation(Relation::Function), &[1500, 500, 1]),
        expect: Expect::Value(Answer::Rows(2001)),
    },
    Case {
        name: CaseName("a_feature_no_plan_has_is_fed_nothing_over_the_run"),
        word: Word::Succeeds,
        input: || Ask::FedOver(Feature::Operation(OperationKind::Join), &[1500, 500]),
        expect: Expect::Value(Answer::Rows(0)),
    },
    Case {
        name: CaseName("with_no_plan_every_feature_of_the_catalog_is_told_with_no_rows"),
        word: Word::Succeeds,
        input: || Ask::FedOverNothing,
        expect: Expect::Value(Answer::Told {
            features: 26 + 17 + 8 + 72,
            unfed: true,
        }),
    },
    Case {
        name: CaseName("the_kind_of_an_operation_is_named_by_its_variant"),
        word: Word::Succeeds,
        input: || Ask::KindOfTheStep,
        expect: Expect::Value(Answer::Kind(OperationKind::Select)),
    },
    Case {
        name: CaseName("a_feature_the_plan_does_not_use_is_fed_nothing"),
        word: Word::Succeeds,
        input: || Ask::SelectFed(Feature::Operation(OperationKind::Join)),
        expect: Expect::Value(Answer::Rows(0)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
