//! The crate's test support: the rows every `*_test.rs` states its cases in, and the one `run` that checks them.
#![doc(hidden)]

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

/// What a refusal selector answers about an error: whether it is the variant the row names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    /// The error is the variant the row names.
    Matches,
    /// The error is some other variant.
    Differs,
}

/// Whether an error is the one a row picks, as a selector answers it: `selects!(error, pattern)` is `Matches` when `error` matches
/// the pattern the row names and `Differs` when it does not.
#[macro_export]
macro_rules! selects {
    ($error:expr, $($pattern:tt)+) => {
        if matches!($error, $($pattern)+) {
            $crate::cases::Selection::Matches
        } else {
            $crate::cases::Selection::Differs
        }
    };
}
/// What a row expects: a value, an error variant, or an error picked by a selector.
#[derive(Debug)]
pub enum Expect<V, E> {
    /// The value the subject answers, compared by equality.
    Value(V),
    /// The error the subject fails with, compared in full.
    Error(E),
    /// The error the subject fails with, picked by a selector that names its variant, for an error
    /// that cannot be written in a constant (it holds an allocation); a non-capturing closure
    /// coerces to the `fn` pointer in a `const`.
    Refusal(fn(&E) -> Selection),
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
    /// The expected value or error variant.
    pub expect: Expect<V, E>,
}

/// Runs every row of `cases` whose word is `word` through `subject` and asserts its expectation.
///
/// A `Succeeds` row expects a `Value` and every other word expects an `Error` or a `Refusal`; a row that names
/// the other kind is a failure of the table, reported with the row's name. An `Error` is compared by value, in full. A word no row has
/// is a failure too: a runner with nothing to run proves nothing.
///
/// Every row runs: a row that fails (or whose subject panics) does not hide the rows after it, and the one panic names every failed row.
///
/// # Panics
///
/// When a row's expectation is not met, or its kind does not match its word.
pub fn run<I, V, E>(cases: &[Case<I, V, E>], word: Word, subject: impl Fn(I) -> Result<V, E>)
where
    V: PartialEq + Debug,
    E: PartialEq + Debug,
{
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
                    Expect::Error(_) | Expect::Refusal(_)
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
            (Expect::Refusal(selects), Err(actual)) => selects(actual) == Selection::Matches,
            (Expect::Error(expected), Err(actual)) => expected == actual,
            _ => false,
        };
        // The expectation is shown by its value, never by the selector it carries: a function pointer prints only
        // its address, so a refusal is shown by its row's case word.
        let expected: &dyn Debug = match &case.expect {
            Expect::Value(value) => value,
            Expect::Error(error) => error,
            Expect::Refusal(_) => &case.word,
        };
        if !met {
            failed.push(format!(
                "{}: expected {expected:?}, answered {answer:?}\n",
                case.name.0
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
