//! Bun is a reference `package.json` is read against: what ryo's `workspace-member` and `package-dependency` rows say of the project
//! in `tests/fixtures/manifests/bun`, held to what Bun wrote of it in its lock file, `bun.lock`, which the fixture holds as Bun
//! writes it (JSON with trailing commas; made by `bun install` in a copy of the fixture without it): the check reads it, and a second
//! one runs Bun itself over the fixture to show Bun takes the lock as it is.

use std::collections::BTreeSet;

use crate::tools::{
    differences, entries, fixture, json, manifest_in, package_json_languages, ryo_lines,
    tool_output,
};

/// The query of the dependencies the check compares: every column Bun's lock file says.
const DEPENDENCIES: &str = "let d = select * from \"package-dependency\"; emit d subject source key(source, package, kind, requirement, target);";

/// What Bun's lock file writes of a workspace package's declarations, and the kind ryo writes each with.
const DECLARED: [(&str, &str); 4] = [
    ("dependencies", "normal"),
    ("optionalDependencies", "normal"),
    ("devDependencies", "dev"),
    ("peerDependencies", "peer"),
];

/// What a package Bun resolves to a workspace package is written as, before the package's directory.
const WORKSPACE: &str = "@workspace:";

/// `text` without the commas Bun writes before a closing brace or bracket, which JSON does not allow.
fn without_trailing_commas(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let (mut quoted, mut escaped) = (false, false);
    for (place, character) in text.char_indices() {
        if quoted {
            match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => quoted = false,
                (false, _) => {}
            }
        } else {
            quoted = character == '"';
            let trailing = character == ','
                && text
                    .get(place + 1..)
                    .and_then(|rest| rest.trim_start().chars().next())
                    .is_some_and(|next| next == '}' || next == ']');
            if trailing {
                continue;
            }
        }
        kept.push(character);
    }
    kept
}

#[test]
fn ryo_says_what_bun_says_of_the_manifests_of_a_workspace() {
    let _beside = crate::beside();
    let root = fixture("bun");
    let lock = json(&without_trailing_commas(
        &std::fs::read_to_string(root.join("bun.lock")).expect("the fixture holds Bun's lock file"),
    ));
    let mut theirs = BTreeSet::new();
    for (directory, workspace) in entries(&lock, "workspaces") {
        let source = manifest_in(directory, "package.json");
        if !directory.is_empty() {
            theirs.extend([format!("members package.json,{source}")]);
        }
        for (key, kind) in DECLARED {
            for (name, requirement) in entries(workspace, key) {
                let resolved = lock
                    .get("packages")
                    .and_then(|packages| packages.get(name.as_str()))
                    .and_then(|package| package.get(0))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let target = resolved
                    .split_once(WORKSPACE)
                    .map(|(_, linked)| manifest_in(linked, "package.json"))
                    .unwrap_or_default();
                theirs.extend([format!(
                    "dependencies {source},{name},{kind},{},{target}",
                    requirement.as_str().unwrap_or_default()
                )]);
            }
        }
    }
    let mut found = Vec::new();
    for (language, linked) in package_json_languages() {
        let ours = ryo_lines(&root, vec![linked], DEPENDENCIES);
        found.extend(differences(
            &format!("bun, read by {language}"),
            "bun",
            &ours,
            &theirs,
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// Bun takes the fixture's lock file as it is: installing from it, frozen, changes nothing and refuses nothing, so the lock the
/// check reads is a lock Bun wrote and still stands behind.
#[test]
fn bun_takes_the_lock_file_of_the_fixture_as_it_is() {
    let _beside = crate::beside();
    let copy = crate::ruff::copy_of(&fixture("bun"));
    let _installed = tool_output(
        "bun",
        &["install", "--frozen-lockfile", "--lockfile-only"],
        copy.path(),
        &[],
    );
    let kept = std::fs::read_to_string(copy.path().join("bun.lock")).expect("the lock");
    let ours = std::fs::read_to_string(fixture("bun").join("bun.lock")).expect("the fixture lock");
    assert_eq!(kept, ours, "Bun leaves the lock file as it was");
}
