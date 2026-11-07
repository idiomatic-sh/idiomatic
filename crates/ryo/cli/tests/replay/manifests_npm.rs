//! npm is a reference `package.json` is read against: what ryo's `workspace-member` and `package-dependency` rows say of the project
//! in `tests/fixtures/manifests/npm`, held to what `npm ls --json` and `npm query` say of it from its lock file.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::tools::{
    differences, entries, fixture, json, manifest_in, package_json_languages, ryo_lines, text_at,
    tool_output,
};

/// The query of the dependencies the check compares: every column npm's listing says.
const DEPENDENCIES: &str = "let d = select * from \"package-dependency\"; emit d subject source key(source, package, kind, requirement, target);";

/// What `resolved` begins with for a package npm links from a directory of the repository, relative to `node_modules`.
const LINKED: &str = "file:../";

/// What npm's listing writes of a node's own declarations, and the kind ryo writes each with: `_dependencies` holds the optional ones
/// too, which are `normal`.
const DECLARED: [(&str, &str); 3] = [
    ("_dependencies", "normal"),
    ("devDependencies", "dev"),
    ("peerDependencies", "peer"),
];

/// The directory of the repository a node of the listing is linked from, none when it comes from elsewhere.
fn linked_from(node: &Value) -> Option<&str> {
    text_at(node, "resolved").strip_prefix(LINKED)
}

/// What npm says of the project at `root`, one line for each member and each dependency, as ryo keys them.
fn npm_lines(root: &Path) -> BTreeSet<String> {
    let offline = [("npm_config_offline", "true")];
    let listed = json(&tool_output(
        "npm",
        &["ls", "--json", "--long", "--all", "--package-lock-only"],
        root,
        &offline,
    ));
    let workspaces = json(&tool_output(
        "npm",
        &["query", ".workspace", "--package-lock-only"],
        root,
        &offline,
    ));
    let mut lines: BTreeSet<String> = workspaces
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|member| {
            format!(
                "members package.json,{}",
                manifest_in(text_at(member, "location"), "package.json")
            )
        })
        .collect();
    let top = entries(&listed, "dependencies");
    // The root declares, and so does each package the repository links into it.
    let sources = std::iter::once(("", &listed)).chain(
        top.iter()
            .filter_map(|(_, node)| linked_from(node).map(|directory| (directory, *node))),
    );
    for (directory, node) in sources {
        let source = manifest_in(directory, "package.json");
        for (key, kind) in DECLARED {
            for (name, requirement) in entries(node, key) {
                // The node of the package depended on: the package's own, else the one the root holds for every package.
                let target = node
                    .get("dependencies")
                    .and_then(|nested| nested.get(name.as_str()))
                    .filter(|nested| nested.get("resolved").is_some())
                    .or_else(|| listed.get("dependencies")?.get(name.as_str()))
                    .and_then(linked_from)
                    .map(|linked| manifest_in(linked, "package.json"))
                    .unwrap_or_default();
                lines.extend([format!(
                    "dependencies {source},{name},{kind},{},{target}",
                    requirement.as_str().unwrap_or_default()
                )]);
            }
        }
    }
    lines
}

#[test]
fn ryo_says_what_npm_says_of_the_manifests_of_a_workspace() {
    let _beside = crate::beside();
    let root = fixture("npm");
    let theirs = npm_lines(&root);
    let mut found = Vec::new();
    for (language, linked) in package_json_languages() {
        let ours = ryo_lines(&root, vec![linked], DEPENDENCIES);
        found.extend(differences(
            &format!("npm, read by {language}"),
            "npm",
            &ours,
            &theirs,
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}
