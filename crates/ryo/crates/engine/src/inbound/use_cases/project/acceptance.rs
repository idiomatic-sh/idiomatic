//! The steps of the `project` feature: each a cucumber expression over the world, one to six lines, asserting the part's own
//! results.

use cucumber::{given, then, when};
use ryo_engine::inbound::use_cases::project::fixtures::projected;

use crate::Behavior;

/// Names the scene the next `project` step runs.
#[given(expr = "the project input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Loads the scene's rule set and runs its project pass over one file's frontier rows.
#[when(expr = "project runs")]
fn runs(world: &mut Behavior) {
    world.answer =
        match crate::load::lowered(&world.input).map(|programs| projected(&programs, false)) {
            Ok(Ok((_, found))) => found.0.to_string(),
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
}

/// Asserts how many rows the pass handed over.
#[then(expr = "project found {string} rows")]
fn found(world: &mut Behavior, rows: String) {
    assert_eq!(world.answer, rows);
}
