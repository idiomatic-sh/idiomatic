//! npm: the package built from the PyPI wheel's binary as the release builds it, installed offline and run.
//! The host's platform package under `platforms/` carries the wheel's binary; it runs only when named
//! (tool `smoke`), and nothing is published.

use snapbox::cmd::Command;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Runs `command`, which must succeed.
fn run(command: Command) {
    drop(command.assert().success());
}

/// Runs `command`, which must succeed and print `expected` on its standard output.
fn prints(command: Command, expected: impl snapbox::IntoData) {
    drop(command.assert().success().stdout_eq(expected));
}

/// The npm command.
const NPM: &str = if cfg!(windows) { "npm.cmd" } else { "npm" };

/// The languages npm's build links: the PyPI wheel's, in the order `--version` lists them.
const LANGUAGES: [&str; 9] = [
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

/// What an installation leaves in its scratch directory.
struct Installation {
    /// The binary the wheel holds, which the host's platform package carries.
    binary: PathBuf,
    /// The tarballs `npm pack` made.
    packed: PathBuf,
    /// The installed launcher package.
    package: PathBuf,
    /// The installed command.
    command: PathBuf,
    /// The host's platform package's name.
    platform_name: String,
    /// The binary the installed platform package holds.
    installed: PathBuf,
}

/// The standard output of `command`, which must succeed.
fn stdout(command: Command) -> String {
    String::from_utf8_lossy(&command.assert().success().get_output().stdout).into_owned()
}

/// What `--version` prints: the crate's version and, with theirs, exactly the wheel's languages.
fn version() -> String {
    let languages = LANGUAGES.map(|language| format!("{language} [..]"));
    format!(
        "tm {} ({})\n",
        env!("CARGO_PKG_VERSION"),
        languages.join(", ")
    )
}

/// The host's target triple, as rustc names it.
fn host() -> std::io::Result<String> {
    stdout(Command::new("rustc").arg("-vV"))
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other("`rustc -vV` names no host"))
}

/// Builds the host wheel from PyPI's `pyproject.toml` into `directory`, as the release builds the
/// wheel whose binary npm's packages carry, and returns it. Maturin writes the wheel into the
/// shared target directory before the build backend copies it out, so a lock file there serializes
/// the build and the copy across test processes, and PyPI's test takes the same lock.
fn wheel(directory: &Path) -> std::io::Result<PathBuf> {
    let lock =
        std::fs::File::create(Path::new(env!("CARGO_TARGET_TMPDIR")).join("wheel-build.lock"))?;
    lock.lock()?;
    let wheels = directory.join("wheels");
    run(Command::new("uv")
        .args(["build", "--wheel", "--out-dir"])
        .arg(&wheels)
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("pypi")));
    let built = entries(&wheels)?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "whl"))
        .collect::<Vec<_>>();
    let [wheel] = built.as_slice() else {
        return Err(std::io::Error::other(format!(
            "the build made {} wheels, expected one",
            built.len()
        )));
    };
    Ok(wheel.clone())
}

/// The wheel's `tm` script, unpacked under `into`: the binary the release's npm build takes.
fn wheel_binary(wheel: &Path, into: &Path) -> PathBuf {
    run(Command::new("uv")
        .args(["run", "--no-project", "python", "-m", "zipfile", "-e"])
        .arg(wheel)
        .arg(into));
    let scripts = format!(
        "idiomatic_sh_tastemaker-{}.data/scripts/tm{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::EXE_SUFFIX
    );
    into.join(scripts)
}

/// Copies the package whose `package.json` is in `source` into `into`, stamped with the crate's
/// version, license and repository (Cargo is the one version authority), beside the crate's LICENSE
/// text.
fn stage(source: &Path, into: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(into)?;
    let _: u64 = std::fs::copy(source.join("package.json"), into.join("package.json"))?;
    let _: u64 = std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("pypi/LICENSE"),
        into.join("LICENSE"),
    )?;
    run(Command::new(NPM)
        .args(["pkg", "set"])
        .arg(concat!("version=", env!("CARGO_PKG_VERSION")))
        .arg(concat!("license=", env!("CARGO_PKG_LICENSE")))
        .arg(concat!("repository=", env!("CARGO_PKG_REPOSITORY")))
        .current_dir(into));
    Ok(())
}

/// Runs npm with `arguments` in `directory`, which must succeed.
fn npm_in<S: AsRef<OsStr>>(directory: &Path, arguments: impl IntoIterator<Item = S>) {
    run(Command::new(NPM).args(arguments).current_dir(directory));
}

/// A value of the `package.json` in `directory`, as node reads it.
fn field(directory: &Path, expression: &str) -> String {
    stdout(
        Command::new("node")
            .arg("--print")
            .arg(format!("require('./package.json'){expression}"))
            .current_dir(directory),
    )
    .trim_end()
    .to_owned()
}

/// The entries of `directory`, sorted.
fn entries(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

/// Packs the host's platform package, carrying the wheel's binary, and the launcher package under
/// `directory` and installs both tarballs into a scratch prefix there, without the network.
fn install(directory: &Path) -> std::io::Result<Installation> {
    let npm = Path::new(env!("CARGO_MANIFEST_DIR")).join("npm");
    let platforms = npm.join("platforms");

    let platform = directory.join("platform");
    let source = platforms.join(host()?);
    stage(&source, &platform)?;
    let _: u64 = std::fs::copy(source.join("README.md"), platform.join("README.md"))?;
    let binary = wheel_binary(&wheel(directory)?, &directory.join("wheel"));
    let bin = platform.join("bin");
    std::fs::create_dir_all(&bin)?;
    let _: u64 = std::fs::copy(
        &binary,
        bin.join(format!("tm{}", std::env::consts::EXE_SUFFIX)),
    )?;

    let staged = directory.join("package");
    stage(&npm, &staged)?;
    let _: u64 = std::fs::copy(npm.join("launcher.js"), staged.join("launcher.js"))?;
    for target in entries(&platforms)? {
        let name = field(&target, ".name");
        let dependency = format!("optionalDependencies.{name}={}", env!("CARGO_PKG_VERSION"));
        npm_in(&staged, ["pkg", "set", dependency.as_str()]);
    }

    let packed = directory.join("packed");
    std::fs::create_dir_all(&packed)?;
    for package in [&platform, &staged] {
        npm_in(
            package,
            [
                OsStr::new("pack"),
                OsStr::new("--pack-destination"),
                packed.as_os_str(),
            ],
        );
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
        .args(entries(&packed)?));

    let (modules, command) = if cfg!(windows) {
        (prefix.join("node_modules"), prefix.join("tm.cmd"))
    } else {
        (prefix.join("lib/node_modules"), prefix.join("bin/tm"))
    };
    let platform_name = field(&platform, ".name");
    let installed = modules
        .join(&platform_name)
        .join(format!("bin/tm{}", std::env::consts::EXE_SUFFIX));
    Ok(Installation {
        binary,
        packed,
        package: modules.join("@idiomatic-sh/tastemaker"),
        command,
        platform_name,
        installed,
    })
}

/// The real command, in a scratch repository: `init`, then `check --format json`, which prints no
/// finding and ends with 0.
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
            .args(["check", "--format", "json"])
            .current_dir(project.path()),
        "{\"workspace\":\"local\",\"findings\":[],\"suppressed\":[]}\n",
    );
    Ok(())
}

/// The binary the installed platform package runs is byte for byte the wheel's `tm` script: npm
/// ships the binary PyPI ships, from the one build the release makes.
#[test]
fn the_platform_package_carries_the_wheel_binary() {
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();
    let (installed, wheel) = (&installation.installed, &installation.binary);
    let same = std::fs::read(installed).unwrap() == std::fs::read(wheel).unwrap();
    assert!(same, "{installed:?} is not the wheel's {wheel:?}");
}

/// The packages hold exactly their own files and pass the dry-run publish; they install from their
/// tarballs, the command reports the crate's version and links exactly the wheel's languages, and
/// the real command runs; the installed package names the host's platform package, at the crate's
/// version, among its optionalDependencies; and the launcher passes arguments and the exit status
/// through, and forwards SIGINT, SIGTERM and SIGHUP.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();

    // No test, target or build file ships: each tarball holds its manifest and its one payload, the
    // launcher or the binary, and the platform package its README.
    let mut shipped = Vec::new();
    for tarball in entries(&installation.packed).unwrap() {
        shipped.extend(
            stdout(Command::new("tar").arg("-tzf").arg(&tarball))
                .lines()
                .map(str::to_owned),
        );
        // The dry-run publish: npm reads the tarball as it would publish it and uploads nothing.
        // `--force` skips only npm's look at the registry's versions, so the crate's version passes
        // whether or not it is already published.
        run(Command::new(NPM)
            .args(["publish", "--dry-run", "--force", "--access", "public"])
            .arg(&tarball));
    }
    shipped.sort();
    assert_eq!(
        shipped,
        [
            "package/LICENSE".to_owned(),
            "package/LICENSE".to_owned(),
            "package/README.md".to_owned(),
            format!("package/bin/tm{}", std::env::consts::EXE_SUFFIX),
            "package/launcher.js".to_owned(),
            "package/package.json".to_owned(),
            "package/package.json".to_owned(),
        ]
    );

    let platform_name = &installation.platform_name;
    assert_eq!(
        field(
            &installation.package,
            &format!(".optionalDependencies['{platform_name}']")
        ),
        env!("CARGO_PKG_VERSION")
    );
    prints(
        Command::new(&installation.command).arg("--version"),
        version(),
    );
    probe(&installation.command).unwrap();
    // The launcher passes arguments and the exit status through: an argument the command refuses
    // fails through the launcher exactly as it fails from the binary.
    let refused = "--no-such-argument";
    let direct = std::process::Command::new(&installation.binary)
        .arg(refused)
        .output()
        .unwrap();
    let launched = std::process::Command::new(&installation.command)
        .arg(refused)
        .output()
        .unwrap();
    assert!(!direct.status.success());
    assert_eq!(launched.status.code(), direct.status.code());
    assert_eq!(launched.stderr, direct.stderr);
    #[cfg(unix)]
    assert_signal_forwarding(&installation, directory.path()).unwrap();
}

/// Replaces the installed native binary with a child that exits 73 on a signal, and holds the
/// launcher to forwarding each of SIGINT, SIGTERM and SIGHUP to it.
#[cfg(unix)]
fn assert_signal_forwarding(installation: &Installation, directory: &Path) -> std::io::Result<()> {
    // A controlled child makes signal delivery observable without relying on how long a real query
    // runs. Only this temporary install's executable is replaced. Written by a child process, never
    // by this one: a file this multi-threaded process held open for writing while another thread
    // forks would stay open in that child, and launching it would fail with "Text file busy". `cat`
    // keeps the installed file's mode, so nothing here sets one.
    run(Command::new("sh")
        .args(["-c", "cat > \"$1\"", "sh"])
        .arg(&installation.installed)
        .stdin(
            "#!/usr/bin/env node
const fs = require('node:fs');
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, () => process.exit(73));
fs.writeFileSync(process.env.DISTRIBUTION_READY, String(process.pid));
setInterval(() => {}, 1000);
",
        ));
    for signal in ["SIGINT", "SIGTERM", "SIGHUP"] {
        let ready = directory.join(signal);
        let mut launcher = std::process::Command::new(&installation.command)
            .env("DISTRIBUTION_READY", &ready)
            .spawn()?;
        let started = poll(|| Ok(ready.exists()))?;
        if started {
            run(Command::new("kill").args(["-s", signal, &launcher.id().to_string()]));
        }
        let ended = poll(|| Ok(launcher.try_wait()?.is_some()))?;
        // The teardown, best effort: the child may be gone already, and the status below decides
        // the probe.
        if let Ok(native) = std::fs::read_to_string(&ready) {
            drop(
                std::process::Command::new("kill")
                    .args(["-KILL", native.trim()])
                    .output(),
            );
        }
        if !ended {
            drop(launcher.kill());
        }
        let code = launcher.wait()?.code();
        if !started || code != Some(73_i32) {
            return Err(std::io::Error::other(format!(
                "{signal} did not reach the child (started: {started}, launcher exit: {code:?})"
            )));
        }
    }
    Ok(())
}

/// Whether `done` holds within five seconds, asked every ten milliseconds.
#[cfg(unix)]
fn poll(mut done: impl FnMut() -> std::io::Result<bool>) -> std::io::Result<bool> {
    for _ in 0..500_u32 {
        if done()? {
            return Ok(true);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    done()
}
