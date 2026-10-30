//! pnpm is a reference `package.json` is read against: what ryo's `workspace-member` and `package-dependency` rows say of the project
//! in `tests/fixtures/manifests/pnpm`, held to what `pnpm ls --json` says of it from its lock file.

use std::collections::BTreeSet;
use std::path::Path;

use crate::tools::{
    differences, entries, fixture, json, manifest_in, package_json_languages, relative, ryo_lines,
    text_at, tool_output,
};

/// The query of the dependencies the check compares: every column pnpm's listing says, which writes no requirement.
const DEPENDENCIES: &str = "let d = select * from \"package-dependency\"; emit d subject source key(source, package, kind, target);";

/// What pnpm's listing writes of a project's dependencies, and the kind ryo writes each with.
const DECLARED: [(&str, &str); 3] = [
    ("dependencies", "normal"),
    ("optionalDependencies", "normal"),
    ("devDependencies", "dev"),
];

/// What pnpm says of the workspace at `root`, one line for each member and each dependency, as ryo keys them.
fn pnpm_lines(root: &Path) -> BTreeSet<String> {
    let listed = json(&tool_output(
        "pnpm",
        &[
            "ls",
            "--recursive",
            "--json",
            "--depth",
            "0",
            "--lockfile-only",
        ],
        root,
        &[],
    ));
    let mut lines = BTreeSet::new();
    for project in listed.as_array().map(Vec::as_slice).unwrap_or_default() {
        let directory = relative(root, Path::new(text_at(project, "path")))
            .expect("a project of the workspace");
        let source = manifest_in(&directory, "package.json");
        if !directory.is_empty() {
            lines.extend([format!("members package.json,{source}")]);
        }
        for (key, kind) in DECLARED {
            for (name, dependency) in entries(project, key) {
                // A project of the workspace is listed at its own directory; any other package within the store's.
                let target = relative(root, Path::new(text_at(dependency, "path")))
                    .map(|linked| manifest_in(&linked, "package.json"))
                    .unwrap_or_default();
                lines.extend([format!("dependencies {source},{name},{kind},{target}")]);
            }
        }
    }
    lines
}

#[test]
fn ryo_says_what_pnpm_says_of_the_manifests_of_a_workspace() {
    let _beside = crate::beside();
    let root = fixture("pnpm");
    let theirs = pnpm_lines(&root);
    let mut found = Vec::new();
    for (language, linked) in package_json_languages() {
        let ours = ryo_lines(&root, vec![linked], DEPENDENCIES);
        found.extend(differences(
            &format!("pnpm, read by {language}"),
            "pnpm",
            &ours,
            &theirs,
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}
