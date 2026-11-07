//! The steps of the `create` feature: each a cucumber expression over the world, asserting the use case's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::packages::service::kind_of;
use tastemaker_core::domain::packages::types::{DefinitionKind, DefinitionName};
use tastemaker_core::domain::writing::types::Writing;
use tastemaker_core::inbound::use_cases::check::fixtures::{package, repository};
use tastemaker_core::inbound::use_cases::create::interface::CreateUseCase as _;
use tastemaker_core::inbound::use_cases::create::types::{
    CreateRequest, Destination, Extension, ScenarioExtension,
};
use tastemaker_core::inbound::use_cases::create::use_case::CreateUseCaseImpl;
use tastemaker_core::outbound::repository::fixtures::fake::RepositoryFake;

/// Sets what the `create` scenario hands the use case.
#[given(expr = "the create input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `create` on the scenario's input and records its answer.
#[when(expr = "create runs")]
fn runs(world: &mut Behavior) {
    let (definition, extension) = world
        .input
        .split_once(" --set extension=")
        .map_or((&*world.input, None), |(definition, extension)| {
            (definition, Some(extension))
        });
    let (kind, name) = definition.split_once('/').unwrap_or_default();
    let files = if kind == "spec" {
        RepositoryFake::of(&[("tastemaker.toml", "")])
    } else {
        repository()
    };
    let create = CreateUseCaseImpl::new(
        Arc::new(files),
        Arc::new(tastemaker_core::outbound::manifest::factory::toml()),
        Arc::new(package()),
    );
    world.answer = match create.create(CreateRequest {
        start: MachinePath("/fake".into()),
        kind: kind_of(tastemaker_core::domain::packages::types::KindWord(kind))
            .unwrap_or(DefinitionKind::Rule),
        name: DefinitionName(name.into()),
        into: Destination::Workspace,
        extension: extension.map_or(Extension::Unnamed, |extension| {
            Extension::Named(ScenarioExtension(extension.into()))
        }),
        writing: Writing::DryRun,
    }) {
        Ok(report) => report.file.0.to_string(),
        Err(error) => error.to_string(),
    };
}

/// Compares the answer of `create` with the one the scenario expects.
#[then(expr = "create answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "create answers what the scenario expects"
    );
}
