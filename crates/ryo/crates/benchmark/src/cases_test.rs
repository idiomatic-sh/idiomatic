//! The runner runs every row of a word: a red row does not hide the rows after it, and the one failure names each red row.
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What the inner table's subject is handed: the row it answers for.
#[derive(Clone, Copy)]
enum Row {
    First,
    Second,
    Third,
}

/// The inner table: its first and third rows are red, its second is green.
const TABLE: &[Case<Row, u8, u8>] = &[
    Case {
        name: CaseName("the_first_row_is_red"),
        word: Word::Succeeds,
        input: || Row::First,
        expect: Expect::Value(9),
    },
    Case {
        name: CaseName("the_second_row_is_green"),
        word: Word::Succeeds,
        input: || Row::Second,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("the_third_row_is_red"),
        word: Word::Succeeds,
        input: || Row::Third,
        expect: Expect::Value(9),
    },
];

fn answer(row: Row) -> Result<u8, u8> {
    match row {
        Row::First => Ok(1),
        Row::Second => Ok(2),
        Row::Third => Ok(3),
    }
}

/// Which rows the runner's one failure names, as the outer table expects them.
#[derive(Debug, PartialEq, Eq)]
struct Reported {
    first: bool,
    second: bool,
    third: bool,
}

#[derive(Clone, Copy)]
enum Scenario {
    RedFirstAndThirdRows,
    NoRowOfTheWord,
}

/// Why the inner run did not report: it passed, or its failure carried no text.
#[derive(Debug, PartialEq, Eq)]
enum Unreported {
    Passed,
    Silent,
}

type Input = Scenario;
type Value = Reported;
type Failure = Unreported;

/// The outer table: what the runner reports of the inner one.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_table_with_a_red_first_and_a_red_third_row_reports_both"),
        word: Word::Succeeds,
        input: || Scenario::RedFirstAndThirdRows,
        expect: Expect::Value(Reported {
            first: true,
            second: false,
            third: true,
        }),
    },
    Case {
        name: CaseName("a_word_no_row_has_is_a_failure_too"),
        word: Word::Succeeds,
        input: || Scenario::NoRowOfTheWord,
        expect: Expect::Value(Reported {
            first: false,
            second: false,
            third: false,
        }),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let word = match input {
        Scenario::RedFirstAndThirdRows => Word::Succeeds,
        Scenario::NoRowOfTheWord => Word::Refuses,
    };
    let failure = std::panic::catch_unwind(|| run(TABLE, word, answer))
        .err()
        .ok_or(Unreported::Passed)?;
    let text = failure.downcast_ref::<String>().ok_or(Unreported::Silent)?;
    Ok(Reported {
        first: text.contains("the_first_row_is_red"),
        second: text.contains("the_second_row_is_green"),
        third: text.contains("the_third_row_is_red"),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
