//! The steps of the `scan` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_engine::inbound::use_cases::scan::fixtures::{Scanned, scanned};

use crate::Behavior;

/// Names the scene the next `scan` step runs.
#[given(expr = "the scan input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Loads the scene's rule set and scans a file of `text` with it through the fixture language.
#[when(expr = "scan runs over {string}")]
fn runs(world: &mut Behavior, text: String) {
    (world.answer, world.filled) =
        match crate::load::lowered(&world.input).map(|programs| scanned(&programs, &text)) {
            Ok(Ok(Some(Scanned {
                answer: Ok(counters),
                fills,
            }))) => (
                format!(
                    "{} candidates, {} steps",
                    counters.candidates.0, counters.steps.0
                ),
                fills.0.to_string(),
            ),
            Ok(Ok(Some(Scanned {
                answer: Err(error),
                fills,
            }))) => (error.to_string(), fills.0.to_string()),
            Ok(walked) => (format!("{walked:?}"), String::new()),
            Err(error) => (error.to_string(), error.to_string()),
        };
}

/// Asserts what the scan counted.
#[then(expr = "scan counted {string} candidates and {string} steps")]
fn counted(world: &mut Behavior, candidates: String, steps: String) {
    assert_eq!(
        world.answer,
        format!("{candidates} candidates, {steps} steps")
    );
}

/// Asserts how many relations the language was asked to fill.
#[then(expr = "the language filled {string} relations")]
fn filled(world: &mut Behavior, relations: String) {
    assert_eq!(world.filled, relations);
}

/// Asserts what `scan` answered.
#[then(expr = "scan answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
