//! `tm init`, over each sample: the project it gives validates and checks clean, whatever its language.

use crate::languages::{initialized, over_samples, ran, reported};

#[test]
fn init_gives_a_project_that_validates_and_checks_clean() {
    over_samples("init", |sample| {
        let scratch = initialized(sample)?;
        ran(&["validate"], scratch.path())?;
        ran(&["check"], scratch.path())?;
        let found = reported(scratch.path())?;
        if found.is_empty() {
            Ok(())
        } else {
            Err(format!("a fresh project reports {found:?}"))
        }
    });
}

/// A project whose top-level names are `created`, written with the pattern grammar's own characters: `tm init` writes each as
/// the name itself, so `tm check` places exactly those paths, and each of `siblings`, which the unescaped name would match as
/// a pattern, is `#closed`.
fn names_are_the_names_themselves(
    created: &[(&str, &str)],
    siblings: &[&str],
) -> Result<(), String> {
    let scratch = tempfile::tempdir().map_err(|error| format!("the scene directory: {error}"))?;
    let root = scratch.path();
    let write = |path: &str, text: &str| -> Result<(), String> {
        let file = root.join(path);
        let parent = file
            .parent()
            .ok_or_else(|| format!("{path} has no parent"))?;
        std::fs::create_dir_all(parent).map_err(|error| format!("{path}'s directory: {error}"))?;
        std::fs::write(&file, text).map_err(|error| format!("{path}: {error}"))
    };
    for (path, text) in created {
        write(path, text)?;
    }
    let git = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(root)
        .status()
        .map_err(|error| format!("git does not start: {error}"))?;
    if !git.success() {
        return Err("git init fails".to_owned());
    }
    ran(&["init"], root)?;
    ran(&["check"], root)?;
    let fresh = reported(root)?;
    if !fresh.is_empty() {
        return Err(format!("a fresh project reports {fresh:?}"));
    }
    for path in siblings {
        write(path, "a new file\n")?;
    }
    let closed = reported(root)?
        .into_iter()
        .filter(|(rule, _, _)| rule.ends_with("#closed"))
        .map(|(_, path, _)| path)
        .collect::<std::collections::BTreeSet<_>>();
    let expected: std::collections::BTreeSet<String> =
        siblings.iter().map(|path| (*path).to_owned()).collect();
    if closed == expected {
        Ok(())
    } else {
        Err(format!(
            "the new paths the names would match as patterns are closed as {closed:?}, not {expected:?}"
        ))
    }
}

/// Brackets and braces, which every platform allows in a name.
#[test]
fn init_writes_names_with_brackets_and_braces_as_the_names_themselves() {
    names_are_the_names_themselves(
        &[
            ("data[1]/rows.txt", "a directory named with brackets\n"),
            ("{curly}/notes.txt", "a directory named with braces\n"),
        ],
        &["data1/rows.txt", "other/notes.txt"],
    )
    .expect("each name is the name itself");
}

/// A star and a question mark, which Windows refuses in a file name: Unix only.
#[cfg(unix)]
#[test]
fn init_writes_names_with_a_star_and_a_question_mark_as_the_names_themselves() {
    names_are_the_names_themselves(
        &[
            ("star*.txt", "a file named with a star\n"),
            ("what?.txt", "a file named with a question mark\n"),
        ],
        &["starX.txt", "whatX.txt"],
    )
    .expect("each name is the name itself");
}

/// `-n` says its plan on stderr, as every `--help` promises: stdout holds nothing, and nothing is written.
#[test]
fn a_dry_run_says_its_plan_on_stderr_and_writes_nothing() {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let root = scratch.path();
    let git = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(root)
        .status()
        .expect("git starts");
    assert!(git.success(), "git init");
    let planned = std::process::Command::new(env!("CARGO_BIN_EXE_tm"))
        .args(["init", "-n"])
        .current_dir(root)
        .output()
        .expect("tm starts");
    assert!(planned.status.success(), "the dry run ends 0");
    assert!(planned.stdout.is_empty(), "a plan is not said on stdout");
    assert!(
        String::from_utf8_lossy(&planned.stderr).contains("would write"),
        "the plan is said on stderr"
    );
    assert!(
        !root.join("tastemaker.toml").exists(),
        "a dry run writes nothing"
    );
}
