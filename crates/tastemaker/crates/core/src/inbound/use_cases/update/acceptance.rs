//! The update scenarios run the production use case and its disk, package, manifest and matching adapters.

use crate::Behavior;
use cucumber::{given, then, when};
use std::sync::Arc;
use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::packages::types::{DefinitionName, VersionText};
use tastemaker_core::domain::writing::types::Writing;
use tastemaker_core::inbound::use_cases::check::fixtures::{
    PACKAGE, RULE, SPEC, TEMPLATE, WORKSPACE,
};
use tastemaker_core::inbound::use_cases::update::interface::UpdateUseCase as _;
use tastemaker_core::inbound::use_cases::update::types::{Entries, UpdateRequest, VersionChoice};
use tastemaker_core::inbound::use_cases::update::use_case::UpdateUseCaseImpl;
use tastemaker_core::outbound::manifest::interface::ManifestPort as _;
use tastemaker_core::outbound::manifest::types::DefinitionText;
use tastemaker_core::outbound::{manifest, matching, packages, repository};

/// A lock that pins `marker` and `part` at their first versions.
const LOCKED: &str = "version = 1\ndependencies = [\"p\"]\n\n[packages.p]\npath = \"p\"\n\n[packages.p.rules]\nmarker = \"0.1.0\"\n\n[packages.p.specs]\npart = \"0.1.0\"\n";

/// Sets what the `update` scenario hands the use case.
#[given(expr = "the update input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Lays out the workspace, its package and its lock as the scenario's input says.
fn setup(root: &std::path::Path, input: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(root.join(".git"))?;
    let put = |name: &str, text: &str| -> std::io::Result<()> {
        let path = root.join(name);
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| std::io::Error::other("fixture has no parent"))?,
        )?;
        std::fs::write(path, text)
    };
    for (name, text) in [
        ("tastemaker.toml", WORKSPACE),
        ("p/package.toml", PACKAGE),
        ("p/rules/marker/0.1.0/rule.toml", RULE),
        ("p/rules/marker/0.2.0/rule.toml", RULE),
        ("p/specs/part/0.1.0/spec.toml", SPEC),
        (
            "p/specs/part/0.1.0/templates/errors/template.toml",
            TEMPLATE,
        ),
        ("src/a/errors.rs", "pub enum Error {}\n"),
    ] {
        put(name, text)?;
    }
    let transitive = input == "transitive";
    if transitive {
        put(
            "p/package.toml",
            "name='p'\ndependencies=[{package='q',path='../q'}]\n",
        )?;
        put("q/package.toml", "name='q'\n")?;
        put("q/rules/new/0.1.0/rule.toml", RULE)?;
        put("p/rules/local/0.1.0/rule.toml", RULE)?;
        put("p/rules/old/0.1.0/rule.toml", RULE)?;
        put(
            "p/specs/part/0.1.0/spec.toml",
            &SPEC.replacen(
                "\n[[scenarios]]",
                "\nrules=[{rule='p/old@0.1.0',include=['**']}]\n\n[[scenarios]]",
                1,
            ),
        )?;
        put(
            "p/specs/part/0.2.0/spec.toml",
            &SPEC.replacen(
                "\n[[scenarios]]",
                "\nrules=[{rule='q/new@0.1.0',include=['**']}]\n\n[[scenarios]]",
                1,
            ),
        )?;
        put(
            "p/specs/part/0.2.0/templates/errors/template.toml",
            TEMPLATE,
        )?;
        put(
            "tastemaker.toml",
            &WORKSPACE.replace(
                "name = \"w\",",
                "name = \"w\", members=[\"src/*\"], rules-exclude=[\"src/excluded/**\"],",
            ),
        )?;
        put(
            "src/member/tastemaker.toml",
            "rules=[{rule='p/local',include=['**']}]\n",
        )?;
        for name in ["outside/tastemaker.toml", "src/excluded/tastemaker.toml"] {
            put(name, "rules=[{rule='absent/missing',include=['**']}]\n")?;
        }
    }
    if input == "missing-transitive" {
        put(
            "p/specs/part/0.1.0/spec.toml",
            &SPEC.replacen(
                "\n[[scenarios]]",
                "\nrules=[{rule='absent/missing@0.1.0',include=['**']}]\n\n[[scenarios]]",
                1,
            ),
        )?;
    }
    // The lock pins `marker` and `part` at their first versions, which an update moves.
    put("tastemaker.lock", LOCKED)?;
    if input == "lock-write" {
        // The lock reads, but it and the directory it is written in refuse the write: a lock that does not read is refused before
        // anything is written, so only a refused write is a partial update.
        for path in [root.join("tastemaker.lock"), root.to_path_buf()] {
            let mut permissions = std::fs::metadata(&path)?.permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    Ok(())
}

/// Runs `update` on the scenario's input and records its answer.
#[when(expr = "update runs")]
fn runs(world: &mut Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    setup(root, &world.input)?;
    let transitive = world.input == "transitive";
    let before = std::fs::read(root.join("tastemaker.toml"))?;
    let locked = std::fs::read(root.join("tastemaker.lock")).ok();
    let update = UpdateUseCaseImpl::new(
        Arc::new(repository::factory::disk(
            Vec::new(),
            tastemaker_core::domain::files::types::DirectoryName(".tastemaker_cache".into()),
        )),
        Arc::new(manifest::factory::toml()),
        Arc::new(packages::factory::path()),
        Arc::new(matching::factory::globset()),
    );
    let write = !matches!(world.input.as_str(), "every" | "absent" | "dry-run");
    let entry = if world.input == "every" {
        Entries::Every
    } else {
        Entries::Named(DefinitionName(
            if transitive {
                "part"
            } else if world.input == "absent" {
                "absent"
            } else {
                "marker"
            }
            .into(),
        ))
    };
    let target = if world.input == "missing-target" {
        VersionChoice::Version(VersionText("99.0.0".into()))
    } else {
        VersionChoice::Latest
    };
    let result = update.update(UpdateRequest {
        start: MachinePath(root.display().to_string().into()),
        entries: entry,
        target,
        writing: if write {
            Writing::Write
        } else {
            Writing::DryRun
        },
    });
    if world.input == "lock-write" {
        // Writable again, so the directory can be removed.
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "a test directory made read-only for the row is made writable again so it can be removed"
        )]
        for path in [root.to_path_buf(), root.join("tastemaker.lock")] {
            let mut permissions = std::fs::metadata(&path)?.permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    // The lock pins each entry's version, so `update` never writes the tastemaker.toml.
    assert_eq!(
        before,
        std::fs::read(root.join("tastemaker.toml"))?,
        "update never rewrites the workspace file"
    );
    if !write || result.is_err() {
        assert_eq!(
            locked,
            std::fs::read(root.join("tastemaker.lock")).ok(),
            "a dry run or a refused update leaves the lock as it was"
        );
    }
    world.answer = match result {
        Ok(report) if write => locked_as_moved(root, transitive, &report)?,
        Ok(_) if world.input == "dry-run" => "lock unchanged".into(),
        Ok(report) => report
            .moves
            .iter()
            .map(|moved| format!("{} {}>{}", moved.entry.0, moved.from.0, moved.to.0))
            .collect::<Vec<_>>()
            .join(","),
        Err(tastemaker_core::inbound::use_cases::update::errors::UpdateError::Resolution {
            ..
        }) => "unresolved transitive asset refused before writes".into(),
        Err(tastemaker_core::inbound::use_cases::update::errors::UpdateError::Relock {
            ..
        }) => "lock write failed; check --fix repairs".into(),
        Err(error) => error.to_string(),
    };
    Ok(())
}

/// What the lock `update` wrote says: the releases it pins after the move, held to what the scenario moved.
fn locked_as_moved(
    root: &std::path::Path,
    transitive: bool,
    report: &tastemaker_core::inbound::use_cases::update::types::UpdateReport,
) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(root.join("tastemaker.lock"))?;
    let lock = manifest::factory::toml().lock(&DefinitionText(text.into()))?;
    let releases = lock
        .packages
        .iter()
        .flat_map(|package| {
            package.releases.iter().map(|release| {
                (
                    package.name.0.to_string(),
                    release.name.0.to_string(),
                    release.version.0.to_string(),
                )
            })
        })
        .collect::<std::collections::BTreeSet<_>>();
    if transitive {
        assert!(
            releases.contains(&("p".into(), "part".into(), "0.2.0".into())),
            "the lock pins the moved spec"
        );
        assert!(
            releases.contains(&("p".into(), "local".into(), "0.1.0".into())),
            "the lock pins the selected local's release"
        );
        assert!(
            releases.contains(&("q".into(), "new".into(), "0.1.0".into())),
            "the lock pins the transitive release"
        );
        assert!(
            !releases.iter().any(|(_, name, _)| name == "old"),
            "the lock drops the release no entry names"
        );
        Ok("transitive releases and selected locals locked".into())
    } else {
        assert!(
            releases.contains(&("p".into(), "marker".into(), "0.2.0".into())),
            "the lock pins the moved rule"
        );
        assert_eq!(report.moves.len(), 1, "one entry moves");
        Ok("lock uses 0.2.0".into())
    }
}

/// Compares the answer of `update` with the one the scenario expects.
#[then(expr = "update answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    let expected: Box<str> = expected.into();
    assert_eq!(
        &*world.answer, &*expected,
        "update answers what the scenario expects"
    );
}
