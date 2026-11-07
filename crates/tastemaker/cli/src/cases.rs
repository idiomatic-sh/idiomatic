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

/// What a row expects: a value, an error variant, or an error picked by a selector.
#[derive(Debug)]
pub enum Expect<V, E> {
    /// The value the subject answers, compared by equality.
    Value(V),
    /// The error the subject fails with, compared including its context.
    Error {
        /// The error the row's subject must fail with.
        expected: E,
        /// Whether an answered error matches `expected`: equality, or a looser test that ignores context the row
        /// cannot know in advance.
        compare: fn(&E, &E) -> bool,
    },
    /// The error the subject fails with, picked by a selector that names its variant, for an error
    /// that cannot be written in a constant (it holds an allocation); a non-capturing closure
    /// coerces to the `fn` pointer in a `const`.
    Refusal(fn(&E) -> bool),
}

impl<V, E: PartialEq> Expect<V, E> {
    /// Build an exact error expectation without constraining contextual selectors.
    pub const fn error(expected: E) -> Self {
        Self::Error {
            expected,
            compare: |wanted, actual| wanted == actual,
        }
    }
}

/// What a case shows, in the product's nouns.
#[derive(Clone, Copy, Debug)]
pub struct CaseName(pub &'static str);

/// One row of a part's cases.
#[derive(Debug)]
pub struct Case<I, V, E> {
    /// What the case shows, in the product's nouns.
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
/// the other kind is a failure of the table, reported with the row's name.
///
/// # Panics
///
/// When a row's expectation is not met, or its kind does not match its word.
pub fn run<I, V, E>(cases: &[Case<I, V, E>], word: Word, subject: impl Fn(I) -> Result<V, E>)
where
    V: PartialEq + Debug,
    E: Debug,
{
    assert!(
        cases.iter().any(|case| case.word == word),
        "no row selected for {word:?}"
    );
    for case in cases.iter().filter(|case| case.word == word) {
        let kind_matches = matches!(
            (case.word, &case.expect),
            (Word::Succeeds, Expect::Value(_))
                | (
                    Word::Refuses | Word::Fails | Word::Rejects,
                    Expect::Error { .. } | Expect::Refusal(_)
                )
        );
        assert!(
            kind_matches,
            "{}: a {:?} row expects the wrong kind",
            case.name.0, case.word
        );
        let answer = subject((case.input)());
        let met = match (&case.expect, &answer) {
            (Expect::Value(expected), Ok(actual)) => expected == actual,
            (Expect::Refusal(selects), Err(actual)) => selects(actual),
            (Expect::Error { expected, compare }, Err(actual)) => compare(expected, actual),
            _ => false,
        };
        assert!(
            met,
            "{}: expected {}, answered {answer:?}",
            case.name.0,
            expectation(&case.expect)
        );
    }
}

/// Names what a row expects for a failure message: the value or error it holds, never the selector's address.
fn expectation<V, E>(expect: &Expect<V, E>) -> String
where
    V: Debug,
    E: Debug,
{
    match expect {
        Expect::Value(value) => format!("{value:?}"),
        Expect::Error { expected, .. } => format!("{expected:?}"),
        Expect::Refusal(_) => "the error its selector picks".to_owned(),
    }
}
