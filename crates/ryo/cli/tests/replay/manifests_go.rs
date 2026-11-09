//! Go is the reference Go's manifests are read against: what ryo's `workspace-member` and `package-dependency` rows say of the Go
//! language's own fixture project, held to what `go list -m -json` and `go mod edit -json` say of it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::tools::{differences, relative, ryo_lines, text_at, tool_output};

/// The query of the dependencies the check compares: every column Go's own reading of the modules says.
const DEPENDENCIES: &str = "let d = select * from \"package-dependency\"; emit d subject source key(source, package, requirement, replacement, target);";

/// What Go is run with besides `PATH` and a `HOME` of its own: no module is fetched and no other toolchain is taken.
const GO: &[(&str, &str)] = &[
    ("GOPROXY", "off"),
    ("GOTOOLCHAIN", "local"),
    ("GOFLAGS", ""),
];

/// The `go.mod` of a module the repository holds, by the module's path, as `go list -m -json all` resolves it: a module of the
/// workspace, or one a `replace` points at a directory of the repository.
fn modules_of(root: &Path, listed: &[Value]) -> BTreeMap<String, String> {
    listed
        .iter()
        .filter_map(|module| {
            let local = if module.get("Main").and_then(Value::as_bool) == Some(true) {
                text_at(module, "GoMod")
            } else {
                module
                    .get("Replace")
                    .map(|replace| text_at(replace, "GoMod"))
                    .unwrap_or_default()
            };
            let manifest = relative(root, Path::new(local))?;
            Some((text_at(module, "Path").to_owned(), manifest))
        })
        .collect()
}

/// What the `replace` directives of a module, as `go mod edit -json` says them, put in the place of `path` at `version`, as ryo writes
/// it: the new path and its version, if any; empty when none does.
fn replacement_of(edited: &Value, path: &str, version: &str) -> String {
    edited
        .get("Replace")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .find(|replace| {
            let old = replace.get("Old").unwrap_or(&Value::Null);
            text_at(old, "Path") == path && [version, ""].contains(&text_at(old, "Version"))
        })
        .and_then(|replace| replace.get("New"))
        .map(|new| match text_at(new, "Version") {
            "" => text_at(new, "Path").to_owned(),
            pinned => format!("{} {pinned}", text_at(new, "Path")),
        })
        .unwrap_or_default()
}

/// What Go says of the workspace at `root`, one line for each member and each requirement of a member, as ryo keys them.
fn go_lines(root: &Path) -> BTreeSet<String> {
    let listed: Vec<Value> = serde_json::Deserializer::from_str(&tool_output(
        "go",
        &["list", "-m", "-e", "-json", "all"],
        root,
        GO,
    ))
    .into_iter::<Value>()
    .map(|module| module.expect("go lists modules as JSON"))
    .collect();
    let workspace = relative(
        root,
        Path::new(tool_output("go", &["env", "GOWORK"], root, GO).trim()),
    )
    .expect("the fixture is a workspace");
    let modules = modules_of(root, &listed);
    let mut lines = BTreeSet::new();
    for module in listed
        .iter()
        .filter(|module| module.get("Main").and_then(Value::as_bool) == Some(true))
    {
        let source =
            relative(root, Path::new(text_at(module, "GoMod"))).expect("a member's go.mod");
        lines.extend([format!("members {workspace},{source}")]);
        let edited = crate::tools::json(&tool_output(
            "go",
            &["mod", "edit", "-json"],
            Path::new(text_at(module, "Dir")),
            GO,
        ));
        for requirement in edited
            .get("Require")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let (path, version) = (
                text_at(requirement, "Path"),
                text_at(requirement, "Version"),
            );
            lines.extend([format!(
                "dependencies {source},{path},{version},{},{}",
                replacement_of(&edited, path, version),
                modules
                    .get(path)
                    .map_or(crate::base::ABSENT, String::as_str),
            )]);
        }
    }
    lines
}

#[test]
fn ryo_says_what_go_says_of_the_manifests_of_a_workspace() {
    let _beside = crate::beside();
    let root = std::fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../languages/go/tests/fixtures/manifests/workspace"),
    )
    .expect("the Go language's fixture project is there");
    let ours = ryo_lines(
        &root,
        vec![ryo_core::outbound::language::factory::LanguageConnection::from(ryo_go::linked())],
        DEPENDENCIES,
    );
    let theirs = go_lines(&root);
    let found = differences("go workspace", "go", &ours, &theirs);
    assert!(found.is_empty(), "{found:#?}");
}
