// ignore: idiomatic.arch/file-length - the npm smoke test is rendered from the distribution spec's template with Tastemaker's linked languages, one platform package each, and the template gives it one file.
//! npm: the package `@idiomatic-sh/tastemaker` and one package per platform, each
//! holding the binary this registry's build makes (`dist-workspace.toml`, beside this
//! file), stamped with the crate's version by `npm pkg set`, packed with `npm pack`,
//! installed into a scratch prefix without the network, and run.

use super::{has_version, host, output, registry_directory, run, strings};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The npm command.
const NPM: &str = if cfg!(windows) { "npm.cmd" } else { "npm" };

/// The operating systems npm names (`process.platform`).
const NPM_OS: &[&str] = &["linux", "darwin", "win32", "freebsd"];

/// The CPUs npm names (`process.arch`).
const NPM_CPU: &[&str] = &["x64", "arm64", "arm", "ia32", "riscv64", "ppc64", "s390x"];

/// The languages npm's build links, which `dist-workspace.toml` names as its `features`.
const LANGUAGES: &[&str] = &[
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

/// What an installation leaves in its scratch directory.
struct Installation {
    /// The binary the build made, as the host's archive holds it.
    binary: PathBuf,
    /// The host's platform package, staged and stamped.
    platform: PathBuf,
    /// The tarballs `npm pack` made.
    packed: PathBuf,
    /// The installed launcher package.
    package: PathBuf,
    /// The installed command.
    command: PathBuf,
}

/// The file holding this registry's build: its platforms and the languages it links.
fn build_file() -> PathBuf {
    registry_directory("npm").join("dist-workspace.toml")
}

/// Builds the host's archive with cargo-dist, unpacks it into `into` and returns the
/// binary in it. cargo-dist writes its archives to `target/distrib` beside
/// `dist-workspace.toml`, whatever Cargo's own target directory is. A lock file there
/// serializes the build and unpacking across test processes, since nextest runs each test
/// in its own.
fn host_binary(into: &Path) -> std::io::Result<PathBuf> {
    let host = host()?;
    let distrib = registry_directory("npm").join("target/distrib");
    std::fs::create_dir_all(&distrib)?;
    let lock = std::fs::File::create(distrib.join("tastemaker-smoke-test.lock"))?;
    lock.lock()?;
    run(Command::new("cargo")
        .args([
            "dist",
            "build",
            "--artifacts=local",
            "--target",
            host.as_str(),
        ])
        .current_dir(registry_directory("npm")))?;
    let name = format!("idiomatic-sh-tastemaker-{host}");
    let archive = format!("{name}{}", if cfg!(windows) { ".zip" } else { ".tar.gz" });
    run(Command::new("tar")
        .arg("-xf")
        .arg(distrib.join(archive))
        .arg("-C")
        .arg(into))?;
    let binary = format!("tm{}", std::env::consts::EXE_SUFFIX);
    let nested = into.join(&name).join(&binary);
    Ok(if nested.exists() {
        nested
    } else {
        into.join(binary)
    })
}

/// Copies the package whose `package.json` is in `source` into `into`, stamped with the
/// crate's version, license and repository (Cargo is the one version authority), beside the
/// crate's LICENSE text.
fn stage(source: &Path, into: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(into)?;
    let _: u64 = std::fs::copy(source.join("package.json"), into.join("package.json"))?;
    let _: u64 = std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE"),
        into.join("LICENSE"),
    )?;
    run(Command::new(NPM)
        .args(["pkg", "set"])
        .arg(concat!("version=", env!("CARGO_PKG_VERSION")))
        .arg(concat!("license=", env!("CARGO_PKG_LICENSE")))
        .arg(concat!("repository=", env!("CARGO_PKG_REPOSITORY")))
        .current_dir(into))?;
    Ok(())
}

/// A value of the `package.json` in `directory`, as node reads it.
fn field(directory: &Path, expression: &str) -> std::io::Result<String> {
    Ok(output(
        Command::new("node")
            .arg("--print")
            .arg(format!("require('./package.json'){expression}"))
            .current_dir(directory),
    )?
    .trim_end()
    .to_owned())
}

/// The tarballs in `packed`.
fn tarballs(packed: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut tarballs = std::fs::read_dir(packed)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    tarballs.sort();
    Ok(tarballs)
}

/// Packs the host's platform package and the launcher package under `directory` and
/// installs both tarballs into a scratch prefix there, without the network.
fn install(directory: &Path) -> std::io::Result<Installation> {
    let npm = registry_directory("npm");
    let platforms = npm.join("platforms");

    let platform = directory.join("platform");
    stage(&platforms.join(host()?), &platform)?;
    let binary = host_binary(directory)?;
    let bin = platform.join("bin");
    std::fs::create_dir_all(&bin)?;
    let _: u64 = std::fs::copy(&binary, bin.join(binary.file_name().unwrap_or_default()))?;

    let staged = directory.join("package");
    stage(&npm, &staged)?;
    let _: u64 = std::fs::copy(npm.join("launcher.js"), staged.join("launcher.js"))?;
    for target in strings(&build_file(), &["dist", "targets"])? {
        let name = field(&platforms.join(&target), ".name")?;
        run(Command::new(NPM)
            .args(["pkg", "set"])
            .arg(format!(
                "optionalDependencies.{name}={}",
                env!("CARGO_PKG_VERSION")
            ))
            .current_dir(&staged))?;
    }

    let packed = directory.join("packed");
    std::fs::create_dir_all(&packed)?;
    for source in [&platform, &staged] {
        run(Command::new(NPM)
            .arg("pack")
            .arg("--pack-destination")
            .arg(&packed)
            .current_dir(source))?;
    }
    let prefix = directory.join("prefix");
    run(Command::new(NPM)
        .args([
            "install",
            "--global",
            "--offline",
            "--no-audit",
            "--no-fund",
        ])
        .arg("--prefix")
        .arg(&prefix)
        .args(tarballs(&packed)?))?;

    let modules = if cfg!(windows) {
        prefix.join("node_modules")
    } else {
        prefix.join("lib/node_modules")
    };
    let command = if cfg!(windows) {
        prefix.join("tm.cmd")
    } else {
        prefix.join("bin/tm")
    };
    Ok(Installation {
        binary,
        platform,
        packed,
        package: modules.join("@idiomatic-sh/tastemaker"),
        command,
    })
}

/// The languages npm's build links: the `features` of `dist-workspace.toml`.
pub(super) fn languages() -> std::io::Result<Vec<String>> {
    strings(&build_file(), &["dist", "features"])
}

/// Installs the packages under `directory` and returns the installed command.
pub(super) fn command(directory: &Path) -> std::io::Result<PathBuf> {
    install(directory).map(|installation| installation.command)
}

/// Every platform the build names has a platform package, and every platform package
/// selects its machine with npm's own `os` and `cpu` names.
#[test]
fn every_target_has_a_platform_package() {
    let platforms = registry_directory("npm").join("platforms");
    let mut packaged = std::fs::read_dir(&platforms)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    packaged.sort();
    let mut built = strings(&build_file(), &["dist", "targets"]).unwrap();
    built.sort();
    assert_eq!(packaged, built);
    for target in packaged {
        let directory = platforms.join(&target);
        for (selector, known) in [("os", NPM_OS), ("cpu", NPM_CPU)] {
            let value = field(&directory, &format!(".{selector}.join()")).unwrap();
            assert!(
                known.contains(&value.as_str()),
                "{target}: npm has no {selector} {value}"
            );
        }
    }
}

/// The build names at least the host's platform, and the host's archive holds a binary
/// that runs and reports the crate's version.
#[test]
fn the_host_archive_runs() {
    assert!(
        strings(&build_file(), &["dist", "targets"])
            .unwrap()
            .contains(&host().unwrap())
    );
    let directory = tempfile::tempdir().unwrap();
    let binary = host_binary(directory.path()).unwrap();
    assert!(has_version(
        &output(Command::new(binary).arg("--version")).unwrap()
    ));
}

/// The packages hold exactly their own files and pass the dry-run publish; they install
/// from their tarballs and the command reports the crate's version; the installed package
/// names the host's platform package, at the crate's version, among its
/// optionalDependencies; and the launcher passes arguments and the exit status through.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();

    // No test, target or build file ships: each tarball holds its manifest and its one
    // payload, the launcher or the binary.
    let mut shipped = Vec::new();
    for tarball in tarballs(&installation.packed).unwrap() {
        shipped.extend(
            output(Command::new("tar").arg("-tzf").arg(&tarball))
                .unwrap()
                .lines()
                .map(str::to_owned),
        );
        // The dry-run publish: npm reads the tarball as it would publish it and uploads
        // nothing.
        run(Command::new(NPM)
            .args(["publish", "--dry-run", "--access", "public"])
            .arg(&tarball))
        .unwrap();
    }
    shipped.sort();
    let binary = installation.binary.file_name().unwrap().to_string_lossy();
    assert_eq!(
        shipped,
        [
            "package/LICENSE".to_owned(),
            "package/LICENSE".to_owned(),
            format!("package/bin/{binary}"),
            "package/launcher.js".to_owned(),
            "package/package.json".to_owned(),
            "package/package.json".to_owned(),
        ]
    );

    let platform_name = field(&installation.platform, ".name").unwrap();
    assert_eq!(
        field(
            &installation.package,
            &format!(".optionalDependencies['{platform_name}']")
        )
        .unwrap(),
        env!("CARGO_PKG_VERSION")
    );
    assert!(has_version(
        &output(Command::new(&installation.command).arg("--version")).unwrap()
    ));
    // The launcher passes arguments and the exit status through: an argument the command
    // refuses fails through the launcher exactly as it fails from the binary.
    let refused = "--no-such-argument";
    let direct = Command::new(&installation.binary)
        .arg(refused)
        .output()
        .unwrap();
    let launched = Command::new(&installation.command)
        .arg(refused)
        .output()
        .unwrap();
    assert!(!direct.status.success());
    assert_eq!(launched.status.code(), direct.status.code());
    assert_eq!(launched.stderr, direct.stderr);
    #[cfg(unix)]
    assert_signal_forwarding(&installation, &platform_name, directory.path()).unwrap();
}

/// Replaces the installed native binary with a child that exits 73 on a signal, and holds the
/// launcher to forwarding each of SIGINT, SIGTERM and SIGHUP to it.
#[cfg(unix)]
fn assert_signal_forwarding(
    installation: &Installation,
    platform_name: &str,
    directory: &Path,
) -> std::io::Result<()> {
    // A controlled child makes signal delivery observable without relying on how long
    // a real query runs. Only this temporary install's executable is replaced.
    let installed_binary = installation
        .package
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| std::io::Error::other("installed package has no modules directory"))?
        .join(platform_name)
        .join("bin/tm");
    std::fs::write(
        &installed_binary,
        "#!/usr/bin/env node
const fs = require('node:fs');
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, () => process.exit(73));
fs.writeFileSync(process.env.DISTRIBUTION_READY, String(process.pid));
setInterval(() => {}, 1000);
",
    )?;
    for signal in ["SIGINT", "SIGTERM", "SIGHUP"] {
        let ready = directory.join(signal);
        let mut launcher = Command::new(&installation.command)
            .env("DISTRIBUTION_READY", &ready)
            .spawn()?;
        for _ in 0..500_u32 {
            if ready.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if !ready.exists() {
            // Best-effort teardown of a probe that never started: the error below is what is reported.
            drop(launcher.kill());
            drop(launcher.wait());
            return Err(std::io::Error::other("the signal probe did not start"));
        }
        let native_pid = std::fs::read_to_string(&ready)?;
        run(Command::new("kill").args(["-s", signal, &launcher.id().to_string()]))?;
        let mut status = None;
        for _ in 0..500_u32 {
            status = launcher.try_wait()?;
            if status.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // The teardown: the child may already be gone, which its kill's exit says; the status check below is what
        // decides the probe, and the kill's exit tells a failure whether the child outlived the signal.
        let teardown = Command::new("kill")
            .args(["-KILL", native_pid.trim()])
            .output()?;
        if status.is_none() {
            drop(launcher.kill());
            drop(launcher.wait());
        }
        if status.and_then(|status| status.code()) != Some(73_i32) {
            let child = if teardown.status.success() {
                "the child outlived it"
            } else {
                "the child was already gone"
            };
            return Err(std::io::Error::other(format!(
                "{signal} did not reach the child: {child}"
            )));
        }
    }
    Ok(())
}

/// The build names exactly this registry's languages, and the installed command links
/// exactly those.
#[test]
fn links_its_languages() {
    let mut stated = LANGUAGES.to_vec();
    stated.sort_unstable();
    let mut named = languages().unwrap();
    named.sort();
    assert_eq!(named, stated);
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();
    let mut linked = super::linked(&installation.command).unwrap();
    linked.sort();
    assert_eq!(linked, stated);
}
