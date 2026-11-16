//! The ruff the recordings were made on, and every other pinned target: its pinned revision, fetched once (`fetch`) into the
//! cache the distribution test fills, so a snapshot fetched by either is the other's. Ruff is 90 MB of source, too big to commit:
//! only the `lint` bench reads the targets, never the test suite.

use std::path::{Path, PathBuf};

/// The root of the pinned ruff checkout.
///
/// # Panics
///
/// When the pin cannot be read or the revision cannot be fetched.
pub(crate) fn ruff() -> PathBuf {
    target("ruff")
}

/// The directory of a target at its pin, cloned into the cache once; the target's own directory inside its project when its pin names one.
///
/// # Panics
///
/// When the pin cannot be read or the revision cannot be fetched.
pub(crate) fn target(name: &str) -> PathBuf {
    let (root, inside) = project(name);
    root.join(inside)
}

/// The directory of the project a target is in, cloned into the cache once, and the directory of the target inside it.
///
/// # Panics
///
/// When the pin cannot be read or the revision cannot be fetched.
pub(crate) fn project(name: &str) -> (PathBuf, String) {
    // A target the distribution proves on is pinned there, once; one only the replay uses is pinned beside it.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let shipped = manifest.join(format!(
        "../distribution/registries/cargo/targets/{name}/pin.toml"
    ));
    let table: toml::Table = std::fs::read_to_string(shipped)
        .or_else(|_absent| {
            std::fs::read_to_string(
                manifest.join(format!("tests/fixtures/replay/targets/{name}/pin.toml")),
            )
        })
        .expect("the pin of a target reads")
        .parse()
        .expect("the pin of a target is TOML");
    let value = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .expect("the pin of a target names its repository and its revision")
            .to_owned()
    };
    let (repository, revision) = (value("repository"), value("revision"));
    let cache = std::env::home_dir()
        .expect("a home directory to cache targets in")
        .join(".cache/idiomatic/targets");
    let directory = super::fetch::fetched(&cache, name, &repository, &revision);
    let inside = table
        .get("directory")
        .and_then(toml::Value::as_str)
        .unwrap_or(".");
    (directory, inside.to_owned())
}
