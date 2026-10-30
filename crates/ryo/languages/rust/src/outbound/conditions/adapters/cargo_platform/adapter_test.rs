//! The `cargo-platform` adapter of the conditions port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::ConditionsCargoPlatformAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::targets::types::Platform;
use crate::outbound::conditions::fixtures::contract::check;
use crate::outbound::conditions::interface::ConditionsPort as _;

/// The platform as a manifest writes it.
type Input = &'static str;
/// The platform as Cargo shows it.
type Value = &'static str;
type Failure = ();

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_triple_with_white_space_around_it_is_trimmed"),
        word: Word::Succeeds,
        input: || "  x86_64-unknown-linux-gnu ",
        expect: Expect::Value("x86_64-unknown-linux-gnu"),
    },
    Case {
        name: CaseName("a_negation_is_shown_as_cargo_shows_it"),
        word: Word::Succeeds,
        input: || "cfg(not( windows ))",
        expect: Expect::Value("cfg(not(windows))"),
    },
];

fn subject(platform: Input) -> Value {
    // The answer is leaked: a row compares it with text that lives for the whole run.
    Box::leak(
        ConditionsCargoPlatformAdapter
            .show(&Platform(platform.into()))
            .0,
    )
}

#[test]
fn succeeds() {
    check(&ConditionsCargoPlatformAdapter);
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
