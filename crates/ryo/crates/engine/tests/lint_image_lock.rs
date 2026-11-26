//! The linter images' package keeps the workspace's dependency versions: its lockfile is a copy of the workspace's, so a package
//! both name has one version in both. A copy that went stale fails here, and `cp` of the workspace lockfile repairs it.
use std::collections::BTreeMap;
use std::path::Path;

fn versions(path: &Path) -> BTreeMap<String, Vec<String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (mut name, mut version) = (None::<String>, None::<String>);
    for line in text.lines().chain(std::iter::once("[[package]]")) {
        if line == "[[package]]" {
            if let (Some(name), Some(version)) = (name.take(), version.take()) {
                found.entry(name).or_default().push(version);
            }
        } else if let Some(value) = line.strip_prefix("name = \"") {
            name = Some(value.trim_end_matches('"').to_owned());
        } else if let Some(value) = line.strip_prefix("version = \"") {
            version = Some(value.trim_end_matches('"').to_owned());
        }
    }
    found.values_mut().for_each(|list| list.sort());
    found
}

#[test]
fn the_image_package_resolves_the_workspace_versions() {
    let engine = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = versions(&engine.join("../../Cargo.lock"));
    let image = versions(&engine.join("benches/lint_image/Cargo.lock"));
    let stale: Vec<String> = image
        .iter()
        .filter(|(name, versions)| {
            workspace.get(*name).is_some_and(|wanted| !versions.iter().all(|version| wanted.contains(version)))
        })
        .map(|(name, versions)| format!("{name} {versions:?} against {:?}", workspace[name]))
        .collect();
    assert!(stale.is_empty(), "the image lockfile is stale (cp crates/ryo/Cargo.lock over it): {stale:#?}");
}
