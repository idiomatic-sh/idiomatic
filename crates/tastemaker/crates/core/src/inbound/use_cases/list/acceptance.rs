//! The steps of the `list` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::inbound::use_cases::check::fixtures::{package, repository};
use tastemaker_core::inbound::use_cases::list::interface::ListUseCase as _;
use tastemaker_core::inbound::use_cases::list::types::{Kinds, ListRequest, SearchTerm};
use tastemaker_core::inbound::use_cases::list::use_case::ListUseCaseImpl;

/// Sets what the `list` scenario hands the use case.
#[given(expr = "the list input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `list` on the scenario's input and records its answer.
#[when(expr = "list runs")]
fn runs(world: &mut Behavior) {
    let mut files = repository();
    if world.input == "nowhere" {
        files.workspace =
            tastemaker_core::outbound::repository::fixtures::fake::FakeWorkspace::Missing;
    }
    let list = ListUseCaseImpl::new(
        Arc::new(files),
        Arc::new(tastemaker_core::outbound::manifest::factory::toml()),
        Arc::new(package()),
        Arc::new(tastemaker_core::outbound::analysis::fixtures::fake::AnalysisFake::default()),
    );
    world.answer = match list.list(ListRequest {
        start: MachinePath("/fake".into()),
        kinds: Kinds::All,
        term: SearchTerm::default(),
    }) {
        Ok(report) => report
            .listed
            .iter()
            .map(|listed| listed.name.0.to_string())
            .collect::<Vec<_>>()
            .join(" "),
        Err(error) => error.to_string(),
    };
}

/// Compares the answer of `list` with the one the scenario expects.
#[then(expr = "list answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "list answers what the scenario expects"
    );
}
