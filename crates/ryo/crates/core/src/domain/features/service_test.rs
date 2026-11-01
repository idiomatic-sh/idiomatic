//! The `features` service's cases, on values alone.

use ryo_language::domain::relations::types::Relation;

use super::service::{catalog, fed_over, kind_of, tally};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operator::types::RowsFed;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::{
    Emit, Feature, Operation, OperationKind, Plan, PlanStep, StepScope,
};

/// What a row of the cases asks.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many features the catalog lists.
    Catalog,
    /// How many rows a select with an equality over `function` was counted as feeding the feature asked for.
    SelectFed(Feature),
    /// The kind of the operation of the one step of the plan.
    KindOfTheStep,
    /// How many rows a feature was fed over runs of plans that each have one select, fed these rows.
    FedOver(Feature, &'static [u64]),
    /// How many features are told of when no plan ran.
    FedOverNothing,
}

/// What a row of the cases asks.
type Input = Scenario;
/// A count.
type Value = u64;
type Failure = ();

/// A plan of one step: a select over `function` that compares for equality.
fn select_over_function() -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::Function),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                    origin: None,
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

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::KindOfTheStep => {
            u64::from(kind_of(&select_over_function().steps[0].operation) == OperationKind::Select)
        }
        Scenario::FedOver(wanted, rows) => {
            let plans: Vec<Plan> = rows.iter().map(|_| select_over_function()).collect();
            let fed: Vec<Vec<RowsFed>> = rows
                .iter()
                .map(|each| vec![RowsFed([*each, 0, 0])])
                .collect();
            fed_over(&plans, &fed)
                .into_iter()
                .filter(|each| each.feature == wanted)
                .map(|each| each.rows.0)
                .sum()
        }
        Scenario::FedOverNothing => {
            let told = fed_over(&[], &[]);
            u64::from(told.len() == catalog().len() && told.iter().all(|each| each.rows.0 == 0))
        }
        Scenario::Catalog => u64::try_from(catalog().len()).unwrap_or(u64::MAX),
        Scenario::SelectFed(wanted) => tally(&select_over_function(), &[RowsFed([1500, 0, 0])])
            .into_iter()
            .filter(|each| each.feature == wanted)
            .map(|each| each.rows.0)
            .sum(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_catalog_lists_every_operation_way_of_comparing_direction_and_relation"),
        word: Word::Succeeds,
        input: || Scenario::Catalog,
        expect: Expect::Value(22 + 14 + 8 + 59),
    },
    Case {
        name: CaseName("a_select_is_fed_the_rows_of_its_input"),
        word: Word::Succeeds,
        input: || Scenario::SelectFed(Feature::Operation(OperationKind::Select)),
        expect: Expect::Value(1500),
    },
    Case {
        name: CaseName("the_way_a_select_compares_is_fed_the_same_rows"),
        word: Word::Succeeds,
        input: || Scenario::SelectFed(Feature::Family(Test::Equal)),
        expect: Expect::Value(1500),
    },
    Case {
        name: CaseName("the_relation_a_select_reads_is_fed_the_rows_it_read"),
        word: Word::Succeeds,
        input: || Scenario::SelectFed(Feature::Relation(Relation::Function)),
        expect: Expect::Value(1500),
    },
    Case {
        name: CaseName("the_rows_a_feature_was_fed_by_every_plan_are_summed"),
        word: Word::Succeeds,
        input: || Scenario::FedOver(Feature::Operation(OperationKind::Select), &[1500, 500]),
        expect: Expect::Value(2000),
    },
    Case {
        name: CaseName("the_rows_of_a_relation_are_summed_over_the_plans_that_read_it"),
        word: Word::Succeeds,
        input: || Scenario::FedOver(Feature::Relation(Relation::Function), &[1500, 500, 1]),
        expect: Expect::Value(2001),
    },
    Case {
        name: CaseName("a_feature_no_plan_has_is_fed_nothing_over_the_run"),
        word: Word::Succeeds,
        input: || Scenario::FedOver(Feature::Operation(OperationKind::Join), &[1500, 500]),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("with_no_plan_every_feature_of_the_catalog_is_told_with_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::FedOverNothing,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_kind_of_an_operation_is_named_by_its_variant"),
        word: Word::Succeeds,
        input: || Scenario::KindOfTheStep,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_feature_the_plan_does_not_use_is_fed_nothing"),
        word: Word::Succeeds,
        input: || Scenario::SelectFed(Feature::Operation(OperationKind::Join)),
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
