//! What every `matching` adapter must pass.

// Error locations are contractual; diagnostic wording belongs to each implementation.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::selection::types::{PathMatch, PathPattern};
use crate::domain::violations::types::RepoPath;
use crate::outbound::matching::errors::MatchingError;
use crate::outbound::matching::interface::MatchingPort;

/// The patterns and the path.
type Input = (&'static [&'static str], &'static str);
/// Whether it matched.
type Value = PathMatch;
/// The refusal of a pattern that is no glob, which no contract row expects.
type Failure = MatchingError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_double_star_crosses_directories"),
        word: Word::Succeeds,
        input: || (&["crates/core/**"], "crates/core/src/lib.rs"),
        expect: Expect::Value(PathMatch::Matched),
    },
    Case {
        name: CaseName("a_single_star_does_not"),
        word: Word::Succeeds,
        input: || (&["crates/*.rs"], "crates/core/lib.rs"),
        expect: Expect::Value(PathMatch::Unmatched),
    },
    Case {
        name: CaseName("no_pattern_matches_every_path"),
        word: Word::Succeeds,
        input: || (&[], "a.rs"),
        expect: Expect::Value(PathMatch::Matched),
    },
    Case {
        name: CaseName("a_pattern_that_is_no_glob_is_refused"),
        word: Word::Refuses,
        input: || (&["a[.rs"], "a.rs"),
        expect: Expect::Refusal(|error| {
            if matches!(error, MatchingError::Pattern {pattern,reason} if pattern == "a[.rs" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl MatchingPort) {
    let subject = |(patterns, path): Input| -> Result<Value, Failure> {
        let patterns: Vec<PathPattern> = patterns
            .iter()
            .map(|pattern| PathPattern((*pattern).into()))
            .collect();
        adapter.matches(&patterns, &RepoPath(path.into()))
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
