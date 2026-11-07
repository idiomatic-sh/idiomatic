//! The steps of the `list_relations` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_core::inbound::use_cases::list_relations::interface::ListRelationsUseCase as _;
use ryo_core::inbound::use_cases::list_relations::use_case::ListRelationsUseCaseImpl;
use ryo_core::outbound::language::fixtures::fake::{FakeCatalog, LanguageFake};

/// Names the scene the next `list_relations` step runs.
#[given(expr = "the list_relations input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `list_relations` over the scene the input names, keeping what it answered.
#[when(expr = "list_relations runs")]
fn runs(world: &mut Behavior) {
    let language = LanguageFake {
        catalog: if world.input == "fails" {
            FakeCatalog::Fails
        } else {
            FakeCatalog::Lists
        },
        ..LanguageFake::default()
    };
    world.answer = match ListRelationsUseCaseImpl::new(Arc::new(language)).list_relations() {
        Ok(report) => format!("{} relations", report.relations.len()),
        Err(error) => error.to_string(),
    };
}

/// Asserts what the last `list_relations` step answered.
#[then(expr = "list_relations answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "list_relations answered otherwise");
}
