//! Cargo: the command line crate built from source by `cargo install`, which is this
//! registry's build, into a scratch root, and run.

use super::{cargo_install, has_version, output};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The languages this registry's build links: the features an install names. A language
/// is added here, in this one place.
const FEATURES: &[&str] = &[
    "css",
    "go",
    "html",
    "javascript",
    "json",
    "markdown",
    "python",
    "rust",
    "toml",
    "typescript",
];

/// The languages this registry's build links, as its install names them.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every registry's languages share one signature, through which npm's and PyPI's read their build files"
)]
pub(super) fn languages() -> std::io::Result<Vec<String>> {
    Ok(FEATURES
        .iter()
        .map(|feature| (*feature).to_owned())
        .collect())
}

/// Installs the command line crate under `root` as a user does, in the release profile
/// with its default features and no others, and returns the installed command: a default
/// the manifest loses is a language this install no longer links, which
/// `links_its_languages` holds the installed command to.
pub(super) fn command(root: &Path) -> std::io::Result<PathBuf> {
    cargo_install(root, "release", &[])
}

/// The crate installs from source and the command reports the crate's version.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installed = command(directory.path()).unwrap();
    assert!(has_version(
        &output(Command::new(&installed).arg("--version")).unwrap()
    ));
}

/// The dry-run publish: Cargo packages and verifies the crate as it would upload it, and
/// uploads nothing. While the crate's manifest says `publish = false`, Cargo must refuse
/// the dry run for that reason, and this test says so. Once the manifest allows
/// publishing, the dry run must pass. Native Cargo patches resolve this checkout's
/// unpublished dependency versions locally; nothing is uploaded or fetched from a
/// private registry, and the normalized package is still built by Cargo's verifier.
#[test]
fn publishes_dry_run() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .unwrap()
        .parse::<toml::Table>()
        .unwrap();
    let refused = manifest["package"].get("publish") == Some(&toml::Value::Boolean(false));
    let temporary = tempfile::tempdir().unwrap();
    let mut publish = Command::new("cargo");
    let _: &mut Command = publish
        .args(["publish", "--dry-run", "--locked", "--allow-dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    if !refused {
        let metadata = output(
            Command::new("cargo")
                .args([
                    "metadata",
                    "--format-version",
                    "1",
                    "--locked",
                    "--offline",
                    "--all-features",
                    // The workspace's own packages, the ones a patch names, and none of the registry's: a dependency of
                    // another platform alone is never downloaded on this one, and offline it could not be read.
                    "--no-deps",
                ])
                .current_dir(env!("CARGO_MANIFEST_DIR")),
        )
        .unwrap();
        let metadata: serde_json::Value = serde_json::from_str(&metadata).unwrap();
        let mut dependencies = toml::Table::new();
        for package in metadata["packages"].as_array().unwrap() {
            let name = package["name"].as_str().unwrap();
            if package["source"].is_null() && name != env!("CARGO_PKG_NAME") {
                let manifest = Path::new(package["manifest_path"].as_str().unwrap());
                drop(dependencies.insert(
                    name.to_owned(),
                    toml::Value::Table(toml::Table::from_iter([(
                        "path".to_owned(),
                        toml::Value::String(
                            manifest.parent().unwrap().to_string_lossy().into_owned(),
                        ),
                    )])),
                ));
            }
        }
        let patches = toml::toml! { patch = { crates-io = dependencies } };
        let config = temporary.path().join("dependencies.toml");
        std::fs::write(&config, patches.to_string()).unwrap();
        let _: &mut Command = publish.arg("--config").arg(config);
    }
    let output = publish.output().unwrap();
    let said = String::from_utf8_lossy(&output.stderr);
    if refused {
        assert!(
            !output.status.success() && said.contains("`package.publish`"),
            "{said}"
        );
        writeln!(
            std::io::stdout(),
            "cargo: the dry-run publish is not yet possible: the manifest says `publish = false`"
        )
        .unwrap();
    } else {
        assert!(output.status.success(), "{said}");
    }
}

/// The installed command links exactly the languages this registry's build names.
#[test]
fn links_its_languages() {
    let directory = tempfile::tempdir().unwrap();
    let installed = command(directory.path()).unwrap();
    let mut linked = super::linked(&installed).unwrap();
    linked.sort();
    let mut named = FEATURES.to_vec();
    named.sort_unstable();
    assert_eq!(linked, named);
}
