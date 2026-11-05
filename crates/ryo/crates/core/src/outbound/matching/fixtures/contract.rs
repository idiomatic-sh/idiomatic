//! What every `matching` adapter must pass.

use ryo_execution::inbound::use_cases::execution::types::matching::MatchingError;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::{Hits, MatchCandidate};

/// What a row of the contract is given: the globs, and the path matched against them.
type Input = (&'static [&'static str], &'static str);
/// Which of the globs matched, as a number written in binary: the first glob is the lowest bit.
type Value = usize;
/// What a row of the contract expects when it is refused.
type Failure = MatchingError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_star_crosses_no_slash_and_two_stars_cross_any"),
        word: Word::Succeeds,
        input: || (&["*.rs", "**/*.rs", "src/*"], "src/lib.rs"),
        expect: Expect::Value(0b110),
    },
    Case {
        name: CaseName("a_path_that_matches_nothing_has_no_hits"),
        word: Word::Succeeds,
        input: || (&["**/*.rs"], "notes.txt"),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_glob_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || (&["["], "a.rs"),
        expect: Expect::Refusal(|error| selects(matches!(error, MatchingError::BadGlob { .. }))),
    },
];

/// Matches one input against one implementation: each glob read, the globs made ready, and the hits of the path.
///
/// # Errors
///
/// `BadGlob` when the implementation refuses a glob.
pub fn answer(
    adapter: &impl MatchingPort,
    globs: &[&str],
    path: &str,
) -> Result<usize, MatchingError> {
    let globs: Vec<Glob> = globs.iter().map(|glob| Glob(Box::from(*glob))).collect();
    // A glob the matching reads is one it makes ready, and one it refuses to read it refuses to make ready.
    for glob in &globs {
        adapter.parse(glob)?;
    }
    let matcher = adapter.compile(&globs)?;
    let mut hits = Hits::default();
    matcher.matches.find(MatchCandidate(path), &mut hits);
    Ok(hits.0.iter().map(|place| 1_usize << place).sum())
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl MatchingPort) {
    run(CASES, Word::Succeeds, |(globs, path): Input| {
        answer(adapter, globs, path)
    });
    run(CASES, Word::Refuses, |(globs, path): Input| {
        answer(adapter, globs, path)
    });
}
