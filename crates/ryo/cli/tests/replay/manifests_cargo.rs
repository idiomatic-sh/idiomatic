//! Cargo is the reference Rust's manifests are read against: what ryo's `workspace-member` and `package-dependency` rows say of each
//! target, held to what `cargo metadata` says of the same manifests.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::base;
use crate::tools::{differences, json, relative, ryo_lines, text_at, tool_result};

/// The query of the dependencies the check compares: every column Cargo's metadata also says.
const DEPENDENCIES: &str = "let d = select * from \"package-dependency\"; emit d subject source key(source, target, origin, binding, package, kind, condition, optional, exact);";

/// Every `Cargo.toml` below `directory`, relative to `root`, leaving out the version control's and the build's directories.
fn manifests_below(root: &Path, directory: &Path, found: &mut Vec<String>) {
    let entries = std::fs::read_dir(directory).expect("a directory of the target is read");
    for entry in entries.map(|entry| entry.expect("an entry of a directory of the target is read"))
    {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != ".git" && name != "target" && name != "node_modules" {
                manifests_below(root, &path, found);
            }
            continue;
        }
        if name == "Cargo.toml"
            && let Some(inside) = relative(root, &path)
        {
            found.push(inside);
        }
    }
}

/// What `cargo metadata` says of the workspace `manifest` belongs to, with `options`, run with only `PATH` and a `CARGO_HOME` of `home`;
/// what Cargo said when it refuses.
fn metadata(root: &Path, manifest: &str, options: &[&str], home: &str) -> Result<Value, String> {
    let at = root.join(manifest);
    let at_text = at.to_string_lossy().into_owned();
    let mut arguments = vec![
        "metadata",
        "--format-version",
        "1",
        "--manifest-path",
        &at_text,
    ];
    arguments.extend_from_slice(options);
    tool_result(
        env!("CARGO"),
        &arguments,
        at.parent().unwrap_or(root),
        &[("CARGO_HOME", home)],
    )
    .map(|text| json(&text))
}

/// The items of the JSON array `value` holds at `key`, none when it is absent.
fn items<'value>(value: &'value Value, key: &str) -> &'value [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The manifest of the package of the repository `dependency` of `package` resolves to in `resolved`: none when it resolves to a
/// package of a registry or a git repository, or was not resolved.
fn resolved_target(
    root: &Path,
    resolved: Option<&Value>,
    package: &Value,
    dependency: &Value,
) -> Option<String> {
    let resolved = resolved?;
    let node = resolved
        .get("resolve")
        .map(|resolve| items(resolve, "nodes"))
        .unwrap_or_default()
        .iter()
        .find(|node| node.get("id") == package.get("id"))?;
    items(node, "deps").iter().find_map(|each| {
        let target = items(resolved, "packages")
            .iter()
            .find(|candidate| candidate.get("id") == each.get("pkg"))?;
        let local = target.get("source").is_none_or(Value::is_null);
        if text_at(target, "name") != text_at(dependency, "name") || !local {
            return None;
        }
        relative(root, Path::new(text_at(target, "manifest_path")))
    })
}

/// Where Cargo says a dependency's package comes from, as ryo writes it: its directory, else the git repository or the registry its
/// source names.
fn origin_of(dependency: &Value) -> &'static str {
    if dependency.get("path").is_some_and(Value::is_string) {
        "path"
    } else if text_at(dependency, "source").starts_with("git+") {
        "git"
    } else {
        "registry"
    }
}

/// Whether Cargo's reading of a requirement admits one version only, as ryo writes it: a comparator `=` with a major, a minor and a
/// patch.
fn exactness_of(requirement: &str) -> &'static str {
    let pinned = requirement.split(',').any(|comparator| {
        comparator.trim().strip_prefix('=').is_some_and(|version| {
            version
                .split(['-', '+'])
                .next()
                .unwrap_or_default()
                .split('.')
                .count()
                == 3
        })
    });
    if pinned { "true" } else { "false" }
}

/// The line of a dependency of the package at `source` on the package at `target`, empty when the repository does not hold it, as
/// ryo keys it.
fn dependency_line(source: &str, target: &str, dependency: &Value) -> String {
    let name = text_at(dependency, "name");
    let binding = dependency
        .get("rename")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let kind = match text_at(dependency, "kind") {
        "dev" => "dev",
        "build" => "build",
        _ => "normal",
    };
    let optional = if dependency.get("optional").and_then(Value::as_bool) == Some(true) {
        "true"
    } else {
        "false"
    };
    format!(
        "dependencies {source},{target},{},{binding},{name},{kind},{},{optional},{}",
        origin_of(dependency),
        text_at(dependency, "target"),
        exactness_of(text_at(dependency, "req")),
    )
}

/// What Cargo says of the manifests `manifests` of the repository at `root`, one line for each member and each dependency, as ryo keys
/// them: each workspace read once, by the first of its manifests, the shallowest first.
fn cargo_lines(root: &Path, manifests: &[String], home: &str) -> BTreeSet<String> {
    let mut candidates: Vec<&String> = manifests.iter().collect();
    candidates.sort_by_key(|path| {
        (
            Path::new(path.as_str()).components().count(),
            (*path).clone(),
        )
    });
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let mut lines = BTreeSet::new();
    for manifest in candidates {
        if covered.contains(manifest) {
            continue;
        }
        // A manifest Cargo refuses declares nothing to compare: ryo's rows of it, if any, are the difference.
        let Ok(found) = metadata(root, manifest, &["--no-deps"], home) else {
            continue;
        };
        // Cargo resolves a dependency that names no directory, offline and from the lock file: to a package of the repository when
        // `[patch]` or `[replace]` say so.
        let unpathed = items(&found, "packages").iter().any(|package| {
            items(package, "dependencies")
                .iter()
                .any(|each| each.get("path").is_none_or(Value::is_null))
        });
        let resolved =
            match unpathed.then(|| metadata(root, manifest, &["--offline", "--locked"], home)) {
                Some(Ok(resolved)) => Some(resolved),
                // Cargo refusing to resolve offline from the lock file is the answer that nothing is resolved.
                Some(Err(_)) | None => None,
            };
        let workspace = relative(
            root,
            &Path::new(text_at(&found, "workspace_root")).join("Cargo.toml"),
        );
        covered.extend(workspace.iter().cloned());
        for package in items(&found, "packages") {
            let Some(source) = relative(root, Path::new(text_at(package, "manifest_path"))) else {
                continue;
            };
            covered.extend([source.clone()]);
            if let Some(workspace) = &workspace {
                lines.extend([format!("members {workspace},{source}")]);
            }
            for dependency in items(package, "dependencies") {
                let target = dependency
                    .get("path")
                    .and_then(Value::as_str)
                    .map_or_else(
                        || resolved_target(root, resolved.as_ref(), package, dependency),
                        |path| relative(root, &Path::new(path).join("Cargo.toml")),
                    )
                    .unwrap_or_default();
                lines.extend([dependency_line(&source, &target, dependency)]);
            }
        }
    }
    lines
}

#[test]
fn ryo_says_what_cargo_says_of_the_manifests_of_every_target() {
    let _beside = crate::beside();
    // Cargo, run with its environment cleared and an empty `CARGO_HOME`: the tools it needs to be found are all it is given.
    let home = tempfile::tempdir().expect("a directory").keep();
    let home = home.to_string_lossy().into_owned();
    let mut found_differences = Vec::new();
    for name in ["ruff", "serde", "tokio"] {
        let root =
            std::fs::canonicalize(base::snapshot::target(name)).expect("the target is there");
        let mut manifests = Vec::new();
        manifests_below(&root, &root, &mut manifests);
        let ours = ryo_lines(&root, Vec::new(), DEPENDENCIES);
        let theirs = cargo_lines(&root, &manifests, &home);
        eprintln!(
            "{name}: {} manifests, {} lines",
            manifests.len(),
            theirs.len()
        );
        found_differences.extend(differences(name, "cargo", &ours, &theirs));
    }
    assert!(found_differences.is_empty(), "{found_differences:#?}");
}
