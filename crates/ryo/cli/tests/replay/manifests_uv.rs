//! uv is the reference Python's manifests are read against: what ryo's `workspace-member` and `package-dependency` rows say of the
//! project in `tests/fixtures/manifests/uv`, held to what `uv workspace metadata` says of it from its lock file.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::tools::{
    differences, entries, fixture, json, manifest_in, relative, ryo_lines, text_at, tool_output,
};

/// The query of the dependencies the check compares: every column uv's metadata says, which resolves requirements to versions and
/// says nothing of `build-system.requires`.
const DEPENDENCIES: &str = "let d = select * from \"package-dependency\" where kind != \"build\"; emit d subject source key(source, package, kind, target);";

/// The `pyproject.toml` of the workspace member a node of uv's resolution is, none for a package from elsewhere.
fn member_of(root: &Path, node: &Value) -> Option<String> {
    let source = node.get("source")?;
    let directory = [text_at(source, "editable"), text_at(source, "virtual")]
        .into_iter()
        .find(|directory| !directory.is_empty())?;
    relative(root, Path::new(directory.trim_end_matches('/')))
        .map(|inside| manifest_in(&inside, "pyproject.toml"))
}

/// The kind ryo writes a dependency of a node of uv's resolution with: a dependency group's is `dev`, a package's and an extra's
/// `normal`.
fn kind_of(node: &Value) -> &'static str {
    if node
        .get("kind")
        .and_then(|kind| kind.get("group"))
        .is_some()
    {
        "dev"
    } else {
        "normal"
    }
}

/// What uv says of the workspace at `root`, one line for each member and each dependency, as ryo keys them.
fn uv_lines(root: &Path) -> BTreeSet<String> {
    let metadata = json(&tool_output(
        "uv",
        &[
            "workspace",
            "metadata",
            "--preview-features",
            "workspace-metadata",
            "--frozen",
            "--offline",
        ],
        root,
        &[("UV_NO_CACHE", "1"), ("UV_PYTHON_DOWNLOADS", "never")],
    ));
    let mut lines: BTreeSet<String> = metadata
        .get("members")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|member| relative(root, Path::new(text_at(member, "path"))))
        .filter(|directory| !directory.is_empty())
        .map(|directory| {
            format!(
                "members pyproject.toml,{}",
                manifest_in(&directory, "pyproject.toml")
            )
        })
        .collect();
    let resolution = metadata.get("resolution").unwrap_or(&Value::Null);
    for (_, node) in entries(&metadata, "resolution") {
        let Some(source) = member_of(root, node) else {
            continue;
        };
        for dependency in node
            .get("dependencies")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let depended = resolution
                .get(text_at(dependency, "id"))
                .unwrap_or(&Value::Null);
            let target = member_of(root, depended).unwrap_or_default();
            // An extra depends on its own package, which is no dependency the manifest writes.
            if target == source {
                continue;
            }
            lines.extend([format!(
                "dependencies {source},{},{},{target}",
                text_at(depended, "name"),
                kind_of(node)
            )]);
        }
    }
    lines
}

#[test]
fn ryo_says_what_uv_says_of_the_manifests_of_a_workspace() {
    let _beside = crate::beside();
    let root = fixture("uv");
    let ours = ryo_lines(
        &root,
        vec![ryo_core::outbound::language::factory::LanguageConnection::from(ryo_python::linked())],
        DEPENDENCIES,
    );
    let theirs = uv_lines(&root);
    let found = differences("uv workspace", "uv", &ours, &theirs);
    assert!(found.is_empty(), "{found:#?}");
}
