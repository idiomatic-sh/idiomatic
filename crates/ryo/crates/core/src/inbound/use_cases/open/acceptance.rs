//! The steps of the `open` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_core::inbound::use_cases::open::interface::OpenUseCase as _;
use ryo_core::inbound::use_cases::open::types::OpenRequest;
use ryo_core::inbound::use_cases::open::use_case::OpenUseCaseImpl;
use ryo_core::outbound::language::fixtures::fake::LanguageFake;
use ryo_core::outbound::memory::fixtures::fake::MemoryFake;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::runs::types::Budget;
use ryo_values::domain::runs::types::MemoryBytes;

/// Names the scene the next `open` step runs.
#[given(expr = "the open input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `open` over the scene the input names, keeping what it answered.
#[when(expr = "open runs")]
#[expect(
    clippy::expect_used,
    reason = "a cucumber step fails by panicking: the runner reports the panic as the failed step, with this message"
)]
fn runs(world: &mut Behavior) {
    let budget: u64 = world.input.parse().expect("the example gives a number");
    let use_case = OpenUseCaseImpl::new(
        Arc::new(LanguageFake::default()),
        Arc::new(MemoryFake {
            asked: Budget::Bytes(MemoryBytes(budget)),
            ..MemoryFake::default()
        }),
    );
    world.answer = match use_case.open(OpenRequest {
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: RootPath(".".into()),
        search_scope: Default::default(),
        given_inventory: Default::default(),
    }) {
        Ok(report) => format!("opened {}", report.session.root.0),
        Err(error) => error.to_string(),
    };
}

/// Asserts what the last `open` step answered.
#[then(expr = "open answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "open answered otherwise");
}
