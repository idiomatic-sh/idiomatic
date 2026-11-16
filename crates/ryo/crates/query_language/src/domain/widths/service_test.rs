//! A grouping answers its keys and what it appends, an extension the rows it reads and a column for each value, a split the rows
//! it reads and the word, and a combination of two inputs the columns of its left input.

use super::service::width_of;
use super::types::InputWidth;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, Operand, QueryLiteral, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service: the width of a step over the two columns of functions kept.
#[derive(Clone, Copy)]
enum Scenario {
    /// A count of them by their first column.
    Counted,
    /// A digest of them by their first column.
    Digested,
    /// The two columns with one value appended.
    Extended,
    /// Those functions without the functions the right input holds.
    LeftOfADifference,
    /// The two columns, once for each word of the second.
    Split,
}

/// The width found.
type Value = Option<InputWidth>;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_count_answers_its_keys_and_the_count"),
        word: Word::Succeeds,
        input: || Scenario::Counted,
        expect: Expect::Value(Some(InputWidth(2))),
    },
    Case {
        name: CaseName("a_digest_answers_its_keys_the_digest_and_the_count"),
        word: Word::Succeeds,
        input: || Scenario::Digested,
        expect: Expect::Value(Some(InputWidth(3))),
    },
    Case {
        name: CaseName("an_extension_answers_the_columns_it_reads_and_one_for_each_value"),
        word: Word::Succeeds,
        input: || Scenario::Extended,
        expect: Expect::Value(Some(InputWidth(3))),
    },
    Case {
        name: CaseName("a_difference_answers_the_columns_of_its_left_input"),
        word: Word::Succeeds,
        input: || Scenario::LeftOfADifference,
        expect: Expect::Value(Some(InputWidth(2))),
    },
    Case {
        name: CaseName("a_split_answers_the_columns_it_reads_and_the_word"),
        word: Word::Succeeds,
        input: || Scenario::Split,
        expect: Expect::Value(Some(InputWidth(3))),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kept = PlanStep {
        operation: Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: vec![ColumnIndex(0), ColumnIndex(1)],
        },
        scope: StepScope::PerFile,
    };
    let operation = match scenario {
        Scenario::Counted => Operation::GroupCount {
            from: Reads::Step(StepId(0)),
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
        Scenario::Digested => Operation::GroupDigest {
            from: Reads::Step(StepId(0)),
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(1),
        },
        Scenario::Extended => Operation::Extend {
            from: Reads::Step(StepId(0)),
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(1),
            ))],
        },
        Scenario::Split => Operation::Words {
            from: Reads::Step(StepId(0)),
            text: ColumnIndex(1),
        },
        Scenario::LeftOfADifference => Operation::Difference {
            left: Reads::Step(StepId(0)),
            right: Reads::Step(StepId(0)),
        },
    };
    let steps = vec![
        kept,
        PlanStep {
            operation,
            scope: StepScope::PerFile,
        },
    ];
    Ok(width_of(&steps, Reads::Step(StepId(1))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
