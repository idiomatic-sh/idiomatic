//! The steps of the `project` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::fixtures::contract::Collect;
use ryo_language::inbound::use_cases::project::service::project;
use ryo_language::inbound::use_cases::project::types::{ProjectRequest, RootPath};
use ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl;

use crate::Behavior;

/// Names the scene the next `project` step runs.
#[given(expr = "the project input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `project` over the scene the input names, keeping what it answered.
#[when(expr = "project runs")]
#[expect(
    clippy::expect_used,
    reason = "a cucumber step fails by panicking: the runner reports the panic as the failed step, with this message"
)]
fn runs(world: &mut Behavior) {
    let mut out = Collect::default();
    let resolved = project(
        &ProjectUseCaseImpl,
        &ProjectRequest {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Vec::new(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(Box::default()),
            manifests: Vec::new(),
            files: Vec::new(),
            notes: Vec::new(),
            wanted: Vec::new(),
        },
        &mut Symbols::default(),
        &mut out,
    )
    .expect("nothing to refuse");
    world.answer = if resolved.refused.is_empty() {
        format!("{} rows", out.relations.len())
    } else {
        format!("{} manifests refused", resolved.refused.len())
    };
}

/// Asserts the last `project` step's answer.
#[then(expr = "project answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber gives a captured string by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "project answered otherwise");
}
