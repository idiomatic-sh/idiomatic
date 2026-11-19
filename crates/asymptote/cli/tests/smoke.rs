//! crates.io: the command line crate built from source by Cargo's install, which is how a user installs
//! it from crates.io, into a scratch root, and run. It runs only when named (tool `smoke`); nothing is
//! published. Its real command: `init`, then `check`, in a scratch repository.

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

/// Installs the command line crate under `root` as a user does, in the release profile with its
/// default features and no others, and returns the installed command.
fn install(root: &Path) -> PathBuf {
    run(Command::new("cargo")
        .args(["install", "--locked", "--profile", "release", "--path"])
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--root")
        .arg(root));
    root.join("bin")
        .join(format!("asymptote{}", std::env::consts::EXE_SUFFIX))
}

/// The crate installs from source, the command reports the crate's version, and one real command
/// succeeds in a scratch repository: `init` writes a workspace, and `check` finds nothing in it.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installed = install(directory.path());
    prints(
        Command::new(&installed).arg("--version"),
        concat!("asymptote ", env!("CARGO_PKG_VERSION"), "\n"),
    );
    probe(&installed).unwrap();
}

/// The real command, in a scratch repository: `init`, then `check`, which finds nothing
/// new and ends with 0.
fn probe(installed: &Path) -> std::io::Result<()> {
    let project = tempfile::tempdir()?;
    run(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project.path()));
    run(Command::new(installed)
        .arg("init")
        .current_dir(project.path()));
    prints(
        Command::new(installed)
            .arg("check")
            .current_dir(project.path()),
        "0 new, 0 resolved\n",
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
                // Online: the packages of the ryo crates this one depends on include ones for other platforms, which a
                // machine that built only for itself has not downloaded.
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
