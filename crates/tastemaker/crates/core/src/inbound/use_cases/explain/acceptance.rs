//! The steps of the `explain` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::inbound::use_cases::check::fixtures::{package, repository};
use tastemaker_core::inbound::use_cases::explain::interface::ExplainUseCase as _;
use tastemaker_core::inbound::use_cases::explain::types::{Asked, ExplainRequest, NameAsked};
use tastemaker_core::inbound::use_cases::explain::use_case::ExplainUseCaseImpl;

/// Sets what the `explain` scenario hands the use case.
#[given(expr = "the explain input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `explain` on the scenario's input and records its answer.
#[when(expr = "explain runs")]
fn runs(world: &mut Behavior) {
    let explain = ExplainUseCaseImpl::new(
        Arc::new(repository()),
        Arc::new(tastemaker_core::outbound::manifest::factory::toml()),
        Arc::new(package()),
        Arc::new(tastemaker_core::outbound::matching::factory::globset()),
        Arc::new(tastemaker_core::outbound::analysis::fixtures::fake::AnalysisFake::default()),
    );
    world.answer = match explain.explain(ExplainRequest {
        start: MachinePath("/fake".into()),
        asked: Asked::Named(NameAsked(world.input.clone().into())),
    }) {
        Ok(report) => report
            .sections
            .first()
            .map(|section| section.title.0.to_string())
            .unwrap_or_default(),
        Err(error) => error.to_string(),
    };
}

/// Compares the answer of `explain` with the one the scenario expects.
#[then(expr = "explain answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "explain answers what the scenario expects"
    );
}
