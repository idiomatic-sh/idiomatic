//! `tm explain`, over each sample: a source file names the rule that applies to it.

use std::path::{Path, PathBuf};

use crate::languages::{RULE, created, initialized, over_samples, printed, ran};

#[test]
fn explain_names_the_created_rule_for_a_source_file() {
    over_samples("explain", |sample| {
        let scratch = initialized(sample)?;
        let project = scratch.path();
        created(&format!("rule/{RULE}"), sample, project)?;
        ran(&["add", RULE, "--at", "src"], project)?;
        ran(&["validate"], project)?;
        let source = format!("src/main.{}", sample.extension);
        let named = if project.join(&source).is_file() {
            source
        } else {
            first_source(&project.join("src"), &sample.extension)?
                .and_then(|path| {
                    path.strip_prefix(project)
                        .ok()
                        .map(|relative| relative.to_string_lossy().into_owned())
                })
                .ok_or("the sample has no source file")?
        };
        let printed = printed(&["explain", &named], project)?;
        if printed.contains(&format!("local/{RULE}")) {
            Ok(())
        } else {
            Err(format!(
                "tm explain {named} does not name local/{RULE}:\n{printed}"
            ))
        }
    });
}

/// The first file under `directory` with `extension`, in a walk of sorted names; a directory that cannot be listed is the
/// error, never one with no sources.
fn first_source(directory: &Path, extension: &str) -> Result<Option<PathBuf>, String> {
    let unlisted = |error: std::io::Error| format!("cannot list {}: {error}", directory.display());
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(unlisted)? {
        entries.push(entry.map_err(unlisted)?.path());
    }
    entries.sort();
    for path in &entries {
        if path.is_dir() {
            if let Some(found) = first_source(path, extension)? {
                return Ok(Some(found));
            }
        } else if path.extension().is_some_and(|found| found == extension) {
            return Ok(Some(path.clone()));
        } else {
            // A file of another extension is not the source sought.
        }
    }
    Ok(None)
}

/// Each example `tm explain --help` opens with runs in a project `tm init` has just made.
#[test]
fn the_help_examples_run_in_a_fresh_project() {
    let scratch =
        tempfile::tempdir().unwrap_or_else(|error| panic!("the scene directory: {error}"));
    let project = scratch.path();
    let repository = std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(project)
        .status()
        .is_ok_and(|status| status.success());
    assert!(repository, "git init succeeds");
    ran(&["init"], project).unwrap_or_else(|why| panic!("{why}"));
    let help = printed(&["explain", "--help"], project).unwrap_or_else(|why| panic!("{why}"));
    let examples: Vec<Vec<&str>> = help
        .lines()
        .skip_while(|line| *line != "Examples:")
        .skip(1)
        .map_while(|line| line.strip_prefix("  tm "))
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert!(
        !examples.is_empty(),
        "explain --help gives examples:\n{help}"
    );
    let failures: Vec<String> = examples
        .iter()
        .filter_map(|arguments| ran(arguments, project).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
