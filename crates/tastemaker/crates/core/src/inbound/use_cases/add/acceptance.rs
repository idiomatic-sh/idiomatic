//! The add scenarios use the production adapters against a real temporary workspace.

use crate::Behavior;
use cucumber::{given, then, when};
use std::collections::BTreeMap;
use std::sync::Arc;
use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::domain::writing::types::Writing;
use tastemaker_core::inbound::use_cases::add::interface::AddUseCase as _;
use tastemaker_core::inbound::use_cases::add::types::{AddRequest, DefinitionAsked};
use tastemaker_core::inbound::use_cases::add::use_case::AddUseCaseImpl;
use tastemaker_core::inbound::use_cases::check::fixtures::{
    PACKAGE, RULE, SPEC, TEMPLATE, WORKSPACE,
};
use tastemaker_core::outbound::manifest::interface::ManifestPort as _;
use tastemaker_core::outbound::manifest::types::DefinitionText;
use tastemaker_core::outbound::{manifest, matching, packages, repository};

/// Sets the definition the add asks for, or `write` to run it for real.
#[given(expr = "the add input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `add` over a real temporary workspace with the production adapters.
#[when(expr = "add runs")]
fn runs(world: &mut Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir_all(root.join(".git"))?;
    for (name, text) in [
        ("tastemaker.toml", WORKSPACE),
        ("p/package.toml", PACKAGE),
        ("p/rules/marker/0.1.0/rule.toml", RULE),
        ("p/specs/part/0.1.0/spec.toml", SPEC),
        (
            "p/specs/part/0.1.0/templates/errors/template.toml",
            TEMPLATE,
        ),
    ] {
        let path = root.join(name);
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| std::io::Error::other("fixture has no parent"))?,
        )?;
        std::fs::write(path, text)?;
    }
    let writing = if world.input == "write" {
        Writing::Write
    } else {
        Writing::DryRun
    };
    let before = std::fs::read(root.join("tastemaker.toml"))?;
    let add = AddUseCaseImpl::new(
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        )),
        Arc::new(manifest::factory::toml()),
        Arc::new(packages::factory::path()),
        Arc::new(matching::factory::globset()),
    );
    let result = add.add(AddRequest {
        start: MachinePath(root.display().to_string().into()),
        definition: DefinitionAsked(if world.input == "write" {
            "part".into()
        } else {
            world.input.clone().into()
        }),
        at: RepoPath(".config/owner's space".into()),
        params: BTreeMap::new(),
        writing,
    });
    if writing == Writing::DryRun || result.is_err() {
        assert_eq!(
            before,
            std::fs::read(root.join("tastemaker.toml"))?,
            "a dry run or a refused add leaves the workspace file as it was"
        );
        assert!(
            !root.join("tastemaker.lock").exists(),
            "a dry run or a refused add writes no lock"
        );
    }
    world.answer = match result {
        Ok(_) if writing == Writing::Write => {
            let written = std::fs::read_to_string(root.join("tastemaker.toml"))?;
            assert!(
                written.contains(".config/owner's space"),
                "the written workspace file holds the added entry's path"
            );
            let text = std::fs::read_to_string(root.join("tastemaker.lock"))?;
            let lock = manifest::factory::toml().lock(&DefinitionText(text.into()))?;
            assert!(
                lock.packages.iter().any(|package| &*package.name.0 == "p"
                    && package.releases.iter().any(
                        |release| &*release.name.0 == "part" && &*release.version.0 == "0.1.0"
                    )),
                "the lock pins the added release"
            );
            "entry and lock written".into()
        }
        Ok(report) => report.entry.0.to_string(),
        Err(error) => error.to_string(),
    };
    Ok(())
}

/// Checks what `add` answered.
#[then(expr = "add answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "add answers what the scenario expects"
    );
}
