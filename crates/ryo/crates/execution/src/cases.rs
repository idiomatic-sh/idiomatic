//! The crate's test support: the rows every `*_test.rs` states its cases in, and the one `run` that checks them.

use std::fmt::Debug;

/// The case word of a row: the four a part's cases are named with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    /// The operation completes and answers the expected value.
    Succeeds,
    /// The input is refused with the expected error variant.
    Refuses,
    /// A dependency fails and its failure becomes the expected error variant.
    Fails,
    /// A weaker implementation is supplied and the expected error variant rejects it.
    Rejects,
}

/// What a row expects: a value or a full error.
#[derive(Debug)]
pub enum Expect<V, E> {
    /// The value the subject answers, compared by equality.
    Value(V),
    /// The error the subject fails with, compared in full by equality.
    Error(E),
}

/// What a case shows, in the product's nouns.
#[derive(Clone, Copy, Debug)]
pub struct CaseName(pub &'static str);

/// One row of a part's cases.
#[derive(Debug)]
pub struct Case<I, V, E> {
    /// What the case shows.
    pub name: CaseName,
    /// The row's one case word.
    pub word: Word,
    /// Builds the row's input from the fixtures or a constructor.
    pub input: fn() -> I,
    /// The expected value or error.
    pub expect: Expect<V, E>,
}

/// Runs every row of `cases` whose word is `word` through `subject` and asserts its expectation.
///
/// A `Succeeds` row expects a `Value` and every other word expects an `Error`; a row that names
/// the other kind is a failure of the table, reported with the row's name. An `Error` is compared by value, in full. A word no row has
/// is a failure too: a runner with nothing to run proves nothing.
///
/// Every row runs: a row that fails (or whose subject panics) does not hide the rows after it, and the one panic names every failed row.
///
/// # Panics
///
/// When a row's expectation is not met, or its kind does not match its word.
pub fn run<I, V: PartialEq + Debug, E: PartialEq + Debug>(
    cases: &[Case<I, V, E>],
    word: Word,
    subject: impl Fn(I) -> Result<V, E>,
) {
    assert!(
        cases.iter().any(|case| case.word == word),
        "no row has the case word {word:?}: a runner with nothing to run proves nothing"
    );
    let mut failed: Vec<String> = Vec::new();
    for case in cases.iter().filter(|case| case.word == word) {
        let kind_matches = matches!(
            (case.word, &case.expect),
            (Word::Succeeds, Expect::Value(_))
                | (
                    Word::Refuses | Word::Fails | Word::Rejects,
                    Expect::Error(_)
                )
        );
        if !kind_matches {
            failed.push(format!(
                "{}: a {:?} row expects the wrong kind\n",
                case.name.0, case.word
            ));
            continue;
        }
        let answer = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            subject((case.input)())
        })) {
            Ok(answer) => answer,
            Err(_) => {
                failed.push(format!("{}: the subject panicked\n", case.name.0));
                continue;
            }
        };
        let met = match (&case.expect, &answer) {
            (Expect::Value(expected), Ok(actual)) => expected == actual,
            (Expect::Error(expected), Err(actual)) => expected == actual,
            _ => false,
        };
        if !met {
            failed.push(format!(
                "{}: expected {:?}, answered {:?}\n",
                case.name.0, case.expect, answer
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "{} row(s) of {word:?} failed:\n{}",
        failed.len(),
        failed.concat()
    );
}
