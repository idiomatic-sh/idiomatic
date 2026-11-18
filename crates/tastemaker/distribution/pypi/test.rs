//! `PyPI`: the wheel built as the release builds it, installed offline and run.
//! The wheel `idiomatic-sh-tastemaker` is built by the backend `pyproject.toml` names (maturin),
//! installed into a fresh virtual environment without the index, and run as `tm` and as
//! `python -m idiomatic_sh_tastemaker`. It runs only when named (tool `smoke`); nothing is
//! published.

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

/// The host's Python 3.
const PYTHON: &str = if cfg!(windows) { "python" } else { "python3" };

/// The wheel's one Python module.
const MODULE: &str = "idiomatic_sh_tastemaker";

/// The languages `PyPI`'s build links, which `pyproject.toml` names as its `features`, in the
/// order `--version` lists them.
const LANGUAGES: &[&str] = &[
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

/// What `--version` prints, whichever languages it lists.
fn version() -> String {
    format!("tm {} ([..])\n", env!("CARGO_PKG_VERSION"))
}

/// This registry's directory: the wheel's project, beside this file.
fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pypi")
}

/// Builds the host wheel from `pyproject.toml` into `directory` and returns it. Maturin writes
/// the wheel into the shared target directory (`target/wheels`, one file name per version)
/// before the build backend copies it out, so two builds at once write one file and either
/// copy can read a torn zip. A lock file in that target directory serializes the build and the
/// copy across threads and across test processes (nextest runs each test in its own), and
/// npm's test takes the same lock.
fn wheel(directory: &Path) -> std::io::Result<PathBuf> {
    let lock =
        std::fs::File::create(Path::new(env!("CARGO_TARGET_TMPDIR")).join("wheel-build.lock"))?;
    lock.lock()?;
    let wheels = directory.join("wheels");
    run(Command::new("uv")
        .args(["build", "--wheel", "--out-dir"])
        .arg(&wheels)
        .arg(project()));
    let built = std::fs::read_dir(&wheels)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
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

/// What an installation leaves in its scratch directory.
struct Installation {
    /// The wheel the build made.
    wheel: PathBuf,
    /// The environment's Python.
    python: PathBuf,
    /// The installed command.
    command: PathBuf,
}

/// Builds the wheel under `directory` and installs it into a fresh virtual environment
/// there, without the index.
fn install(directory: &Path) -> std::io::Result<Installation> {
    let wheel = wheel(directory)?;
    let environment = directory.join("environment");
    run(Command::new("uv")
        .args(["venv", "--python", PYTHON])
        .arg(&environment));
    let scripts = environment.join(if cfg!(windows) { "Scripts" } else { "bin" });
    let python = scripts.join(["python", std::env::consts::EXE_SUFFIX].concat());
    run(Command::new("uv")
        .args(["pip", "install", "--no-index", "--no-deps", "--python"])
        .arg(&python)
        .arg(&wheel));
    Ok(Installation {
        wheel,
        python,
        command: scripts.join("tm"),
    })
}

/// The real command, in a scratch repository: `init`, then `check --format json`, which prints
/// no finding and ends with 0.
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

/// The standard output of `command`, which must succeed.
fn stdout(command: Command) -> String {
    String::from_utf8_lossy(&command.assert().success().get_output().stdout).into_owned()
}

/// The wheel holds exactly its own files and passes the dry-run publish; it installs into
/// a fresh environment, both of its entry points report the crate's version and the real
/// command runs; a second, `--target` install finds the executable beside the package Python
/// imports.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();

    // No test, target or build file ships: the wheel holds the executable, the Python
    // module's two files, and the metadata and bill of materials the backend writes.
    let mut shipped = stdout(
        Command::new(PYTHON)
            .args([
                "-c",
                "import sys, zipfile; print('\\n'.join(zipfile.ZipFile(sys.argv[1]).namelist()))",
            ])
            .arg(&installation.wheel),
    )
    .lines()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    shipped.sort();
    let stem = format!("{MODULE}-{}", env!("CARGO_PKG_VERSION"));
    let executable = format!("tm{}", std::env::consts::EXE_SUFFIX);
    assert_eq!(
        shipped,
        [
            format!("{stem}.data/scripts/{executable}"),
            format!("{stem}.dist-info/METADATA"),
            format!("{stem}.dist-info/RECORD"),
            format!("{stem}.dist-info/WHEEL"),
            format!("{stem}.dist-info/licenses/LICENSE"),
            format!("{stem}.dist-info/sboms/idiomatic-sh-tastemaker.cyclonedx.json"),
            format!("{MODULE}/__init__.py"),
            format!("{MODULE}/__main__.py"),
        ]
    );
    // The dry-run publish: uv reads the wheel as it would upload it and uploads nothing.
    run(Command::new("uv")
        .args(["publish", "--dry-run"])
        .arg(&installation.wheel));

    prints(
        Command::new(&installation.command).arg("--version"),
        version(),
    );
    prints(
        Command::new(&installation.python).args(["-m", MODULE, "--version"]),
        version(),
    );
    probe(&installation.command).unwrap();
    // Installed again with --target, the package Python imports from there runs the executable beside it, not the
    // environment's own.
    let target = directory.path().join("target");
    run(Command::new("uv")
        .args(["pip", "install", "--no-index", "--no-deps", "--python"])
        .arg(&installation.python)
        .arg("--target")
        .arg(&target)
        .arg(&installation.wheel));
    let found = stdout(
        Command::new(&installation.python)
            .env("PYTHONPATH", &target)
            .args([
                "-c",
                &format!("import {MODULE}; print({MODULE}.find_tastemaker_bin())"),
            ]),
    );
    // Compared as canonical paths both sides: the temporary directory may be reached through a symbolic link (macOS's
    // /var is /private/var), which Python resolves.
    assert!(
        Path::new(found.trim())
            .canonicalize()
            .unwrap()
            .starts_with(target.canonicalize().unwrap()),
        "{found}"
    );
}

/// The build names exactly this registry's languages, and the installed command links exactly
/// those: its `--version` lists each, with its version, in parentheses.
#[test]
fn links_its_languages() {
    let project = std::fs::read_to_string(project().join("pyproject.toml"))
        .unwrap()
        .parse::<toml::Table>()
        .unwrap();
    let named = project["tool"]["maturin"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|feature| feature.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(named, LANGUAGES);
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();
    let languages = LANGUAGES
        .iter()
        .map(|language| format!("{language} [..]"))
        .collect::<Vec<_>>()
        .join(", ");
    prints(
        Command::new(&installation.command).arg("--version"),
        format!("tm {} ({languages})\n", env!("CARGO_PKG_VERSION")),
    );
}
