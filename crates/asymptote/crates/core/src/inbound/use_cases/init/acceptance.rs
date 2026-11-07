//! The steps of the `init` feature: each a cucumber expression over the world, asserting the use case's own results,
//! over real git repositories made in temporary directories.

use cucumber::{given, then, when};

use crate::Behavior;

use asymptote_core::domain::manifest::types::ProductRoot;
use asymptote_core::domain::ratchet::types::Writing;
use asymptote_core::inbound::use_cases::init::interface::InitUseCase as _;
use asymptote_core::inbound::use_cases::init::types::{InitPorts, InitRequest};
use asymptote_core::inbound::use_cases::init::use_case::InitUseCaseImpl;

/// Names the scene the next `init` step runs.
#[given(expr = "the init input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// `init` over the real adapters, its lock at the top level of the repository `start` is in, as the edge binds it.
fn real_init(start: &ProductRoot) -> InitUseCaseImpl {
    use asymptote_core::outbound::hooks::interface::HooksPort as _;
    use asymptote_core::outbound::{hooks, lock, manifest};
    use std::sync::Arc;
    let hooks = hooks::factory::git();
    let top = hooks.top(start).unwrap_or_else(|_| start.clone());
    InitUseCaseImpl::new(InitPorts {
        manifest: Arc::new(manifest::factory::toml()),
        lock: Arc::new(lock::factory::toml(top)),
        hooks: Arc::new(hooks),
    })
}

/// Runs git with `arguments` in `directory`, outside any repository the test itself runs in.
fn git(directory: &std::path::Path, arguments: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("git {arguments:?}: {output:?}").into())
    }
}

/// The hook scripts the repository holds, relative to `root`, comma separated, or `-`.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a scene's hooks that cannot be listed are a broken step: it fails, never reads as no scripts"
)]
fn scripts(root: &std::path::Path) -> String {
    let mut found = Vec::new();
    for directory in [".git/hooks", "hooks-here"] {
        // A scene whose repository has no such directory holds no script there; any other failure fails the step.
        let entries = match std::fs::read_dir(root.join(directory)) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("the scripts in {directory} cannot be listed: {error}"),
        };
        for entry in entries {
            let entry = entry.expect("a script's entry can be read");
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".sample") {
                found.push(format!("{directory}/{name}"));
            }
        }
    }
    found.sort();
    if found.is_empty() {
        "-".to_owned()
    } else {
        found.join(",")
    }
}

/// How many operations the lock of the product at `root` holds.
fn operations(root: &std::path::Path) -> usize {
    std::fs::read_dir(root.join(".asymptote/operations")).map_or(0, Iterator::count)
}

/// Prepares a repository as the input asks and runs `init` over the production adapters, keeping what it
/// configured, installed and wrote.
#[when(expr = "init runs")]
fn runs(world: &mut Behavior) -> Result<(), Box<dyn std::error::Error>> {
    let input: Box<str> = world.input.clone().into();
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    if &*input != "not-a-repo" {
        git(root, &["init", "-q"])?;
    }
    let start = ProductRoot(root.display().to_string().into());
    let ask = |writing| InitRequest {
        start: start.clone(),
        writing,
    };
    match &*input {
        // The first `init` only leaves the configuration and hooks the asked one runs over: its answer is not judged.
        "rerun" => {
            if real_init(&start).init(ask(Writing::Write))?.writing != Writing::Write {
                return Err("the first init wrote nothing".into());
            }
        }
        "removed-hook" => {
            if real_init(&start).init(ask(Writing::Write))?.writing != Writing::Write {
                return Err("the first init wrote nothing".into());
            }
            std::fs::write(
                root.join("asymptote.toml"),
                "[hooks]\npre-push = [\"lint\"]\n",
            )?;
        }
        "existing" => std::fs::write(
            root.join("asymptote.toml"),
            "[hooks]\npre-commit = [\"lint\"]\n",
        )?,
        "hooks-path" => git(root, &["config", "core.hooksPath", "hooks-here"])?,
        "foreign" => {
            std::fs::create_dir_all(root.join(".git/hooks"))?;
            std::fs::write(
                root.join(".git/hooks/pre-commit"),
                "#!/bin/sh\necho theirs\n",
            )?;
        }
        "foreign-unlisted" => {
            std::fs::create_dir_all(root.join(".git/hooks"))?;
            std::fs::write(
                root.join(".git/hooks/commit-msg"),
                "#!/bin/sh\necho theirs\n",
            )?;
        }
        _ => {}
    }
    let writing = if &*input == "dry-run" {
        Writing::DryRun
    } else {
        Writing::Write
    };
    let names = |names: &[asymptote_core::domain::manifest::types::HookName]| {
        if names.is_empty() {
            "-".to_owned()
        } else {
            names
                .iter()
                .map(|name| name.0.to_string())
                .collect::<Vec<_>>()
                .join(",")
        }
    };
    let manifest = || {
        if root.join("asymptote.toml").is_file() {
            "yes"
        } else {
            "no"
        }
    };
    world.answer = match real_init(&start).init(ask(writing)) {
        Ok(report) => format!(
            "{:?}; installed {}; removed {}; scripts {}; manifest {}; operations {}",
            report.configuration,
            names(&report.installed),
            names(&report.removed),
            scripts(root),
            manifest(),
            operations(root)
        ),
        Err(error) => format!(
            "{}; scripts {}; manifest {}; operations {}",
            error
                .to_string()
                .replace(&*root.display().to_string(), "$ROOT"),
            scripts(root),
            manifest(),
            operations(root)
        ),
    };
    Ok(())
}

/// Asserts the last `init` step's answer.
#[then(expr = "init answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    // The step's text, kept as the answer's type: what a cucumber argument arrives as is an owned `String`.
    let expected: Box<str> = expected.into();
    assert_eq!(&*world.answer, &*expected, "init answered otherwise");
}
