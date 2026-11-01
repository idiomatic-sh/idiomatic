//! The samples under `crates/tastemaker/examples/<language>/api` as standing acceptance. Each sample is a program with one violation planted
//! per row of the README's table and a `fixes.toml` beside its `tastemaker.toml`: the findings `tm check` reports, by rule,
//! path and line, each row's fix as edits, and the commands that build the fixed program and run its tests. Every
//! `crates/tastemaker/examples/<language>/api` is a sample, found by listing `crates/tastemaker/examples/`, so a language is added by adding its sample and
//! nothing here; one without its `fixes.toml` fails. Each sample is run as a copy, never where it is: a copy at the same place
//! in a repository of its own, and a copy that is a repository of its own, each pinned to the built `tm`: its paths are read
//! from the project's directory, wherever the repository starts.

use std::process::Command;

#[cfg(unix)]
use crate::fixes::copy;
use crate::fixes::{
    Placement, UNCOPIED, apply, documented, readable, reported, restore, said, scene,
};

/// Holds each sample's documented findings where `placement` places it, collecting a failure per sample.
fn reports_documented(placement: Placement) -> Vec<String> {
    let mut failures = Vec::new();
    for sample in readable(&mut failures) {
        let (_scratch, project) = scene(&sample, placement);
        match reported(&project) {
            Ok(found) if found == documented(&sample, None) => {}
            Ok(found) => failures.push(format!(
                "{}: tm check reports\n  {found:?}\nnot the documented\n  {:?}",
                sample.name,
                documented(&sample, None)
            )),
            Err(why) => failures.push(format!("{}: {why}", sample.name)),
        }
    }
    failures
}

/// Fixes and restores each row of each sample placed as `placement` says, collecting a failure per row.
fn rows_fix_and_restore(placement: Placement) -> Vec<String> {
    let mut failures = Vec::new();
    for sample in readable(&mut failures) {
        let (_scratch, project) = scene(&sample, placement);
        let expected = documented(&sample, None);
        for row in &sample.fixes.rows {
            let case = format!("{}: row \"{}\" fix and restore", sample.name, row.name);
            let saved = match apply(&project, row) {
                Ok(saved) => saved,
                Err(why) => {
                    failures.push(format!("{case}: {why}"));
                    continue;
                }
            };
            let fixed = documented(&sample, Some(row));
            match reported(&project) {
                Ok(found) if found == fixed => {}
                Ok(found) => failures.push(format!(
                    "{case}: after the fix tm check reports\n  {found:?}\nnot\n  {fixed:?}"
                )),
                Err(why) => failures.push(format!("{case}: after the fix, {why}")),
            }
            restore(&saved);
            match reported(&project) {
                Ok(found) if found == expected => {}
                Ok(found) => failures.push(format!(
                    "{case}: after the restore tm check reports\n  {found:?}\nnot\n  {expected:?}"
                )),
                Err(why) => failures.push(format!("{case}: after the restore, {why}")),
            }
        }
    }
    failures
}

/// Applies every row of each sample, holds the fixed sample to no finding, then runs its commands: it builds and its own tests
/// pass, and what they leave behind is the sample's own: it still reports nothing. A command whose toolchain is not installed
/// does not start, and that is a failure: the gate provides every sample's toolchain, so a sample is never passed over.
/// Collects a failure per sample.
fn fixed_programs_build_and_pass() -> Vec<String> {
    let mut failures = Vec::new();
    for sample in readable(&mut failures) {
        let (_scratch, project) = scene(&sample, Placement::Alone);
        let case = format!("{}: every row fixed", sample.name);
        if let Some(why) = sample.fixes.rows.iter().find_map(|row| {
            apply(&project, row)
                .err()
                .map(|why| format!("row \"{}\": {why}", row.name))
        }) {
            failures.push(format!("{case}: {why}"));
            continue;
        }
        match reported(&project) {
            Ok(found) if found.is_empty() => {}
            Ok(found) => failures.push(format!("{case}: tm check still reports {found:?}")),
            Err(why) => failures.push(format!("{case}: {why}")),
        }
        for command in &sample.fixes.commands {
            let ran = command.split_first().map(|(program, arguments)| {
                Command::new(crate::fixes::found(program))
                    .args(arguments)
                    .current_dir(&project)
                    .output()
            });
            match ran {
                Some(Ok(output)) if output.status.success() => {}
                Some(Ok(output)) => failures.push(format!(
                    "{case}: `{}` failed:\n{}{}",
                    command.join(" "),
                    said(&output.stdout),
                    said(&output.stderr)
                )),
                Some(Err(error)) => failures.push(format!(
                    "{case}: `{}` did not start: {error}",
                    command.join(" ")
                )),
                None => failures.push(format!("{case}: an empty command")),
            }
        }
        // What the commands leave behind (a lock, an environment, installed packages) is the sample's as its README is
        // followed: the fixed sample still reports nothing.
        match reported(&project) {
            Ok(found) if found.is_empty() => {}
            Ok(found) => failures.push(format!(
                "{case}: after its commands ran, tm check reports {found:?}"
            )),
            Err(why) => failures.push(format!("{case}: after its commands ran, {why}")),
        }
    }
    failures
}

#[test]
fn each_sample_documents_one_row_for_each_finding() {
    let mut failures = Vec::new();
    drop(readable(&mut failures));
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn each_sample_reports_its_documented_findings_inside_a_repository() {
    let failures = reports_documented(Placement::Nested);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn each_sample_reports_its_documented_findings_as_a_repository_of_its_own() {
    let failures = reports_documented(Placement::Alone);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn each_row_of_each_sample_is_fixed_and_restored_inside_a_repository() {
    let failures = rows_fix_and_restore(Placement::Nested);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn each_row_of_each_sample_is_fixed_and_restored_as_a_repository_of_its_own() {
    let failures = rows_fix_and_restore(Placement::Alone);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn each_sample_with_every_row_fixed_builds_and_passes_its_own_tests() {
    let failures = fixed_programs_build_and_pass();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// A sample whose README tables its rows lists exactly the rows of its `fixes.toml`, each as `| row | `rule` | `path` |`:
/// what the README says the sample reports is what `tm check` reports.
#[test]
fn each_readme_table_lists_exactly_the_documented_rows() {
    let mut failures = Vec::new();
    for sample in readable(&mut failures) {
        let readme = match std::fs::read_to_string(sample.directory.join("README.md")) {
            Ok(readme) => readme,
            Err(error) => {
                failures.push(format!("{}: no readable README.md: {error}", sample.name));
                continue;
            }
        };
        let tabled: std::collections::BTreeSet<&str> = readme
            .lines()
            .filter(|line| line.starts_with("| ") && line.contains("| `local/"))
            .collect();
        if tabled.is_empty() {
            continue;
        }
        let documented: std::collections::BTreeSet<String> = sample
            .fixes
            .rows
            .iter()
            .map(|row| format!("| {} | `{}` | `{}` |", row.name, row.rule, row.path))
            .collect();
        for row in &documented {
            if !tabled.contains(row.as_str()) {
                failures.push(format!("{}: README.md does not table {row}", sample.name));
            }
        }
        for line in &tabled {
            if !documented.contains(*line) {
                failures.push(format!(
                    "{}: README.md tables {line}, no row of fixes.toml",
                    sample.name
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The Tastemaker files of a sample, and what a copy leaves out: not the program's own files, so not its language.
const NOT_THE_PROGRAM: [&str; 4] = [
    "tastemaker",
    "tastemaker.toml",
    "tastemaker.lock",
    "fixes.toml",
];

/// The names and the extensions of the files of the program `directory` holds, its Tastemaker files and what a run fills
/// left out.
fn program_files(
    directory: &std::path::Path,
    names: &mut std::collections::BTreeSet<String>,
    extensions: &mut std::collections::BTreeSet<String>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("{} cannot be listed: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("{} cannot be listed: {error}", directory.display()))?
            .path();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if NOT_THE_PROGRAM.contains(&name.as_str()) || UNCOPIED.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            program_files(&path, names, extensions)?;
        } else {
            if let Some(extension) = path.extension() {
                let _: bool = extensions.insert(extension.to_string_lossy().into_owned());
            }
            let _: bool = names.insert(name);
        }
    }
    Ok(())
}

/// The `rule.toml` files beneath `directory`.
fn rule_files(directory: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .map_err(|error| format!("{} cannot be listed: {error}", next.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("{} cannot be listed: {error}", next.display()))?
                .path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.file_name().is_some_and(|name| name == "rule.toml") {
                found.push(path);
            }
        }
    }
    Ok(found)
}

/// The files the scenarios of the rule at `path` give that are no file of the program: named as none of `names` and with
/// an extension none of `extensions` is.
fn foreign_files(
    path: &std::path::Path,
    names: &std::collections::BTreeSet<String>,
    extensions: &std::collections::BTreeSet<String>,
) -> Result<Vec<String>, String> {
    let table = std::fs::read_to_string(path)
        .map_err(|error| format!("{} cannot be read: {error}", path.display()))?
        .parse::<toml::Table>()
        .map_err(|error| format!("{} does not parse: {error}", path.display()))?;
    let given = table
        .get("scenarios")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|scenario| scenario.get("given")?.get("files")?.as_table())
        .flat_map(toml::Table::keys);
    Ok(given
        .filter(|file| {
            let name = file.rsplit('/').next().unwrap_or(file);
            let extension = std::path::Path::new(name)
                .extension()
                .map(|each| each.to_string_lossy().into_owned());
            !names.contains(name) && extension.is_none_or(|each| !extensions.contains(&each))
        })
        .cloned()
        .collect())
}

/// Each scenario of each sample's own rules is written in the sample's own language and ecosystem: every file it gives is
/// named as a file of the program is, or has an extension one of its files has. A sample's package validates with the
/// language providers its own program needs, and no other.
#[test]
fn each_samples_rules_give_only_files_of_its_own_language() {
    let mut failures = Vec::new();
    for sample in readable(&mut failures) {
        let (mut names, mut extensions) = (
            std::collections::BTreeSet::new(),
            std::collections::BTreeSet::new(),
        );
        let found = program_files(&sample.directory, &mut names, &mut extensions)
            .and_then(|()| rule_files(&sample.directory.join("tastemaker/rules")))
            .and_then(|rules| {
                rules
                    .iter()
                    .map(|rule| {
                        foreign_files(rule, &names, &extensions).map(|files| {
                            files
                                .into_iter()
                                .map(|file| format!("{} gives `{file}`", rule.display()))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
            });
        match found {
            Ok(foreign) => failures.extend(foreign.into_iter().flatten().map(|each| {
                format!(
                    "{}: {each}, which is no file of the sample's own language",
                    sample.name
                )
            })),
            Err(why) => failures.push(format!("{}: {why}", sample.name)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A copy of a sample leaves out what a run writes and what a package manager fills, and makes a symbolic link again as the link it
/// is: a generated environment is not copied, and a link to a directory is no tree copied from outside the sample.
#[test]
#[cfg(unix)]
fn a_copy_of_a_sample_leaves_out_generated_environments_and_keeps_links_as_links() {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let (from, to) = (scratch.path().join("sample"), scratch.path().join("copy"));
    let outside = scratch.path().join("outside");
    std::fs::create_dir_all(from.join("src")).expect("the sample");
    std::fs::create_dir_all(&outside).expect("a directory outside the sample");
    std::fs::write(outside.join("secret.txt"), "not the sample's\n").expect("a file outside");
    std::fs::write(from.join("src/lib.txt"), "code\n").expect("a file");
    for generated in UNCOPIED {
        std::fs::create_dir_all(from.join(generated).join("inner")).expect("a generated directory");
        std::fs::write(from.join(generated).join("inner/file"), "generated\n")
            .expect("a generated file");
    }
    std::os::unix::fs::symlink(&outside, from.join("elsewhere")).expect("a link to a directory");
    std::os::unix::fs::symlink("src/lib.txt", from.join("alias.txt")).expect("a link to a file");
    copy(&from, &to).expect("the sample copied");
    for generated in UNCOPIED {
        assert!(
            !to.join(generated).exists(),
            "`{generated}` is generated and is not copied"
        );
    }
    assert_eq!(
        std::fs::read_to_string(to.join("src/lib.txt")).expect("the file copied"),
        "code\n"
    );
    for (name, target) in [
        ("elsewhere", outside.as_path()),
        ("alias.txt", std::path::Path::new("src/lib.txt")),
    ] {
        let link = to.join(name);
        assert!(
            std::fs::symlink_metadata(&link)
                .expect("the link copied")
                .file_type()
                .is_symlink(),
            "`{name}` is made again as a link, not followed"
        );
        assert_eq!(std::fs::read_link(&link).expect("where it points"), target);
    }
}
