//! crates.io: the command line crate built from source by Cargo's install, which is how a user installs
//! it from crates.io, into a scratch root, and run. It runs only when named (tool `smoke`); nothing is
//! published. Its real command: `query run` of a comment query over one `smoke.rs`, in a scratch repository.
//! It holds the installed command to the languages the crate's default features link.

use snapbox::cmd::Command;
use std::path::{Path, PathBuf};

/// Runs `command`, which must succeed.
fn run(command: Command) {
    drop(command.assert().success());
}

/// Runs `command`, which must succeed and print `expected` on its standard output.
fn prints(command: Command, expected: impl snapbox::IntoData) {
    drop(command.assert().success().stdout_eq(expected));
}

/// The languages the crate's default features link. A language is added here, in this one place.
const FEATURES: [&str; 9] = [
    "css",
    "go",
    "html",
    "json",
    "markdown",
    "python",
    "rust",
    "toml",
    "typescript",
];

/// The comment query the probe runs: the files holding the comment `distribution-proof`.
const PROBE: &str = r#"let found = select c."file" as "file" from "comment" as c where c."text" = " distribution-proof"; emit found subject "file" key("file");"#;

/// Installs the command line crate under `root` as a user does, in the release profile with its
/// default features and no others, and returns the installed command: a default the manifest
/// loses is a language this install no longer links, which `links_its_languages` holds the
/// installed command to.
fn install(root: &Path) -> PathBuf {
    run(Command::new("cargo")
        .args(["install", "--locked", "--profile", "release", "--path"])
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--root")
        .arg(root));
    root.join("bin")
        .join(format!("ryo{}", std::env::consts::EXE_SUFFIX))
}

/// The crate installs from source, the command reports the crate's version, and one real command
/// succeeds in a scratch repository: the comment query finds the one file holding the comment.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installed = install(directory.path());
    prints(
        Command::new(&installed).arg("--version"),
        format!("ryo {}[..]\n", env!("CARGO_PKG_VERSION")),
    );
    probe(&installed).unwrap();
}

/// The real command, in a scratch repository: `query run` of [`PROBE`] over one `smoke.rs` holding
/// the comment, which answers that one file.
fn probe(installed: &Path) -> std::io::Result<()> {
    let project = tempfile::tempdir()?;
    std::fs::write(
        project.path().join("smoke.rs"),
        "// distribution-proof\nfn main() {}\n",
    )?;
    run(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project.path()));
    prints(
        Command::new(installed)
            .args(["query", "run", "--format", "json", "--text", PROBE])
            .current_dir(project.path()),
        "{\"columns\":[\"file\",\"line\",\"column\"],\"rows\":[[\"smoke.rs\",null,null]],\"count\":1,\"shown\":1,\"unread\":[]}\n",
    );
    Ok(())
}

/// The dry-run publish: Cargo packages and verifies the crate as it would upload it, and uploads
/// nothing. While the crate's manifest says `publish = false`, Cargo must refuse the dry run for
/// that reason, and this test says so. Once the manifest allows publishing, the dry run must pass.
/// Native Cargo patches resolve this checkout's unpublished dependency versions locally; nothing is
/// uploaded or fetched from a private registry, and the normalized package is still built by
/// Cargo's verifier.
#[test]
fn publishes_dry_run() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .unwrap()
        .parse::<toml::Table>()
        .unwrap();
    let refused = manifest["package"].get("publish") == Some(&toml::Value::Boolean(false));
    let temporary = tempfile::tempdir().unwrap();
    let mut publish = Command::new("cargo")
        .args(["publish", "--dry-run", "--locked", "--allow-dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    if !refused {
        let metadata = Command::new("cargo")
            .args([
                "metadata",
                "--format-version",
                "1",
                "--locked",
                // Online: the packages of the crates this one depends on include ones for other
                // platforms, which a machine that built only for itself has not downloaded.
                "--all-features",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .assert()
            .success();
        let metadata: serde_json::Value =
            serde_json::from_slice(&metadata.get_output().stdout).unwrap();
        let mut dependencies = toml::Table::new();
        for package in metadata["packages"].as_array().unwrap() {
            let name = package["name"].as_str().unwrap();
            if package["source"].is_null() && name != env!("CARGO_PKG_NAME") {
                let manifest = Path::new(package["manifest_path"].as_str().unwrap());
                let path = manifest.parent().unwrap().to_string_lossy().into_owned();
                drop(dependencies.insert(
                    name.to_owned(),
                    toml::Value::Table(toml::Table::from_iter([(
                        "path".to_owned(),
                        toml::Value::String(path),
                    )])),
                ));
            }
        }
        let patches = toml::toml! { patch = { crates-io = dependencies } };
        let config = temporary.path().join("dependencies.toml");
        std::fs::write(&config, patches.to_string()).unwrap();
        publish = publish.arg("--config").arg(config);
    }
    if refused {
        drop(
            publish
                .assert()
                .failure()
                .stderr_eq("...\n[..]`package.publish`[..]\n...\n"),
        );
    } else {
        drop(publish.assert().success());
    }
}

/// The installed command links exactly the languages the crate's default features name: its
/// provider catalog lists each (`relation describe comment --json`, `languages`, `<name>: ...`).
#[test]
fn links_its_languages() {
    let directory = tempfile::tempdir().unwrap();
    let installed = install(directory.path());
    let described = Command::new(&installed)
        .args(["relation", "describe", "comment", "--json"])
        .assert()
        .success();
    let catalog: serde_json::Value =
        serde_json::from_slice(&described.get_output().stdout).unwrap();
    let mut linked = catalog["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .and_then(|text| text.split_once(": "))
                .map(|(name, _)| name.to_owned())
                .unwrap()
        })
        .collect::<Vec<_>>();
    linked.sort();
    assert_eq!(linked, FEATURES);
}
