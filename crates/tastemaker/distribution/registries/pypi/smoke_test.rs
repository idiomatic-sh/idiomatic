//! `PyPI`: the wheel `idiomatic-sh-tastemaker`, built by the backend `pyproject.toml`
//! names (maturin), which is this registry's build, installed into a fresh virtual
//! environment without the index, run as `tm` and as
//! `python -m idiomatic_sh_tastemaker`.

use super::{has_version, output, registry_directory, run, strings};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The host's Python 3.
const PYTHON: &str = if cfg!(windows) { "python" } else { "python3" };

/// The wheel's one Python module.
const MODULE: &str = "idiomatic_sh_tastemaker";

/// The languages `PyPI`'s build links, which `pyproject.toml` names as its `features`.
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
    /// The wheel the build made.
    wheel: PathBuf,
    /// The environment's Python.
    python: PathBuf,
    /// The installed command.
    command: PathBuf,
}

/// The file holding this registry's build: the wheel and the languages it links.
fn build_file() -> PathBuf {
    registry_directory("pypi").join("pyproject.toml")
}

/// Builds the wheel under `directory` and installs it into a fresh virtual environment
/// there, without the index.
fn install(directory: &Path) -> std::io::Result<Installation> {
    // Maturin writes the wheel into the shared target directory (`target/wheels`, one
    // file name per version) before the build backend copies it out, so two builds at
    // once write one file and either copy can read a torn zip. A lock file in that
    // target directory serializes the build and the copy across threads and across test
    // processes (nextest runs each test in its own), as the npm build's lock does.
    let lock =
        std::fs::File::create(Path::new(env!("CARGO_TARGET_TMPDIR")).join("pypi-smoke-test.lock"))?;
    lock.lock()?;
    let wheels = directory.join("wheels");
    run(Command::new("uv")
        .args(["build", "--wheel", "--out-dir"])
        .arg(&wheels)
        .arg(registry_directory("pypi")))?;
    let wheels = std::fs::read_dir(&wheels)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "whl"))
        .collect::<Vec<_>>();
    let [wheel] = wheels.as_slice() else {
        return Err(std::io::Error::other(format!(
            "the build made {} wheels, expected one",
            wheels.len()
        )));
    };
    let wheel = wheel.clone();

    let environment = directory.join("environment");
    run(Command::new("uv")
        .args(["venv", "--python", PYTHON])
        .arg(&environment))?;
    let scripts = environment.join(if cfg!(windows) { "Scripts" } else { "bin" });
    let python = scripts.join("python");
    run(Command::new("uv")
        .args(["pip", "install", "--no-index", "--no-deps", "--python"])
        .arg(&python)
        .arg(&wheel))?;
    Ok(Installation {
        wheel,
        python,
        command: scripts.join("tm"),
    })
}

/// The languages `PyPI`'s build links: the `features` of `pyproject.toml`.
pub(super) fn languages() -> std::io::Result<Vec<String>> {
    strings(&build_file(), &["tool", "maturin", "features"])
}

/// Builds and installs the wheel under `directory` and returns the installed command.
pub(super) fn command(directory: &Path) -> std::io::Result<PathBuf> {
    install(directory).map(|installation| installation.command)
}

/// The wheel holds exactly its own files and passes the dry-run publish; it installs into
/// a fresh environment and both of its entry points report the crate's version; a second,
/// `--target` install finds the executable beside the package Python imports.
#[test]
fn installs_and_runs() {
    let directory = tempfile::tempdir().unwrap();
    let installation = install(directory.path()).unwrap();

    // No test, target or build file ships: the wheel holds the executable, the Python
    // module's two files, and the metadata and bill of materials the backend writes.
    let mut shipped = output(
        Command::new(PYTHON)
            .args([
                "-c",
                "import sys, zipfile; print('\\n'.join(zipfile.ZipFile(sys.argv[1]).namelist()))",
            ])
            .arg(&installation.wheel),
    )
    .unwrap()
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
        .arg(&installation.wheel))
    .unwrap();

    assert!(has_version(
        &output(Command::new(&installation.command).arg("--version")).unwrap()
    ));
    assert!(has_version(
        &output(Command::new(&installation.python).args(["-m", MODULE, "--version"])).unwrap()
    ));
    // Installed again with --target, the package Python imports from there runs the executable beside it, not the
    // environment's own.
    let target = directory.path().join("target");
    run(Command::new("uv")
        .args(["pip", "install", "--no-index", "--no-deps", "--python"])
        .arg(&installation.python)
        .arg("--target")
        .arg(&target)
        .arg(&installation.wheel))
    .unwrap();
    let found = output(
        Command::new(&installation.python)
            .env("PYTHONPATH", &target)
            .args([
                "-c",
                &format!("import {MODULE}; print({MODULE}.find_tastemaker_bin())"),
            ]),
    )
    .unwrap();
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
