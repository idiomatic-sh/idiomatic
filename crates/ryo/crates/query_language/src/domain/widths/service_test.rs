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
use std::borrow::Cow;

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

/// What a row asks: the width of one scenario's step, or of the last step of a ladder at two depths.
#[derive(Clone, Copy)]
enum Ask {
    /// The width of the scenario's step.
    Width(Scenario),
    /// A ladder of so many levels and of twice as many, each level a union of the step before with itself and a split of that.
    Ladder(usize),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The width found.
    Width(Option<InputWidth>),
    /// The widths of a ladder at two depths: a split is counted once each, so each counts the splits on its chain.
    Counted(Cow<'static, str>),
}

type Value = Answer;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Ask;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_count_answers_its_keys_and_the_count"),
        word: Word::Succeeds,
        input: || Ask::Width(Scenario::Counted),
        expect: Expect::Value(Answer::Width(Some(InputWidth(2)))),
    },
    Case {
        name: CaseName("a_digest_answers_its_keys_the_digest_and_the_count"),
        word: Word::Succeeds,
        input: || Ask::Width(Scenario::Digested),
        expect: Expect::Value(Answer::Width(Some(InputWidth(3)))),
    },
    Case {
        name: CaseName("an_extension_answers_the_columns_it_reads_and_one_for_each_value"),
        word: Word::Succeeds,
        input: || Ask::Width(Scenario::Extended),
        expect: Expect::Value(Answer::Width(Some(InputWidth(3)))),
    },
    Case {
        name: CaseName("a_difference_answers_the_columns_of_its_left_input"),
        word: Word::Succeeds,
        input: || Ask::Width(Scenario::LeftOfADifference),
        expect: Expect::Value(Answer::Width(Some(InputWidth(2)))),
    },
    Case {
        name: CaseName("a_split_answers_the_columns_it_reads_and_the_word"),
        word: Word::Succeeds,
        input: || Ask::Width(Scenario::Split),
        expect: Expect::Value(Answer::Width(Some(InputWidth(3)))),
    },
    Case {
        name: CaseName("the_width_under_a_ladder_counts_each_step_on_its_chain_once"),
        word: Word::Succeeds,
        input: || Ask::Ladder(16),
        expect: Expect::Value(Answer::Counted(Cow::Borrowed(
            "16 levels, 33 steps: Some(18) columns; 32 levels, 65 steps: Some(34) columns",
        ))),
    },
];

/// The two columns of functions kept, then `levels` levels of a union of the step before with itself and a split of that union.
fn ladder(levels: usize) -> Vec<PlanStep> {
    let step = |operation| PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::PerFile,
    };
    let mut steps = vec![step(Operation::Select {
        from: Reads::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: vec![ColumnIndex(0), ColumnIndex(1)],
    })];
    for _ in 0..levels {
        let before = Reads::Step(StepId(
            u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
        ));
        steps.push(step(Operation::Union {
            left: before,
            right: before,
        }));
        let union = Reads::Step(StepId(
            u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
        ));
        steps.push(step(Operation::Words {
            from: union,
            text: ColumnIndex(1),
        }));
    }
    steps
}

/// The width of the last step of a ladder of `levels` levels, beside how many steps the ladder has.
fn last_width(levels: usize) -> String {
    let steps = ladder(levels);
    let last = Reads::Step(StepId(
        u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
    ));
    let width = width_of(&steps, last).map(|width| width.0);
    format!("{levels} levels, {} steps: {width:?} columns", steps.len())
}

fn subject(ask: Input) -> Result<Value, Failure> {
    let scenario = match ask {
        Ask::Width(scenario) => scenario,
        Ask::Ladder(levels) => {
            return Ok(Answer::Counted(Cow::Owned(format!(
                "{}; {}",
                last_width(levels),
                last_width(levels.saturating_mul(2))
            ))));
        }
    };
    let kept = PlanStep {
        written: Default::default(),
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
            written: Default::default(),
            operation,
            scope: StepScope::PerFile,
        },
    ];
    Ok(Answer::Width(width_of(&steps, Reads::Step(StepId(1)))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
