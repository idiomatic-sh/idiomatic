//! Product-neutral preparation of a target checkout, before any run of the command.

use super::super::{output, run};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// The TOML file `file`, parsed.
fn table(file: &Path) -> std::io::Result<toml::Table> {
    std::fs::read_to_string(file)?
        .parse()
        .map_err(std::io::Error::other)
}

/// The text `key` holds in `target`'s `pin.toml`.
fn pinned(target: &Path, key: &str) -> std::io::Result<String> {
    table(&target.join("pin.toml"))?
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other(format!("its pin has no `{key}`")))
}

/// Caches an immutable checkout and verifies its exact pinned revision before reuse.
pub(crate) fn snapshot(target: &Path) -> std::io::Result<PathBuf> {
    let revision = pinned(target, "revision")?;
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let cache = std::env::home_dir()
        .ok_or_else(|| std::io::Error::other("no home directory to cache targets in"))?
        .join(".cache/idiomatic/targets");
    std::fs::create_dir_all(&cache)?;
    let lock = std::fs::File::create(cache.join("lock"))?;
    lock.lock()?;
    let directory = cache.join(format!("{name}-{revision}"));
    if !directory.exists() {
        let partial = cache.join(format!("{name}-{revision}.partial"));
        if partial.exists() {
            std::fs::remove_dir_all(&partial)?;
        }
        std::fs::create_dir_all(&partial)?;
        let held = Command::new("git")
            .args(["cat-file", "-e"])
            .arg([revision.as_str(), "^{commit}"].concat())
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()?
            .status
            .success();
        if held {
            let checkout = output(
                Command::new("git")
                    .args(["rev-parse", "--show-toplevel"])
                    .current_dir(env!("CARGO_MANIFEST_DIR")),
            )?;
            run(Command::new("git")
                // Byte for byte and links as links, whatever the platform's git would convert.
                .args([
                    "clone",
                    "--quiet",
                    "--no-checkout",
                    "--config",
                    "core.autocrlf=false",
                    "--config",
                    "core.symlinks=true",
                ])
                .arg(checkout.trim_end())
                .arg(&partial))?;
        } else {
            run(Command::new("git").args(["init", "--quiet"]).arg(&partial))?;
            for setting in [
                ["core.autocrlf", "false"],
                ["core.symlinks", "true"],
                ["core.longpaths", "true"],
            ] {
                run(Command::new("git")
                    .arg("config")
                    .args(setting)
                    .current_dir(&partial))?;
            }
            let repository = pinned(target, "repository")?;
            // Never wait on a prompt for credentials: a repository or revision that cannot be fetched fails at once.
            run(Command::new("git")
                .args(["fetch", "--quiet", "--depth", "1"])
                .arg(&repository)
                .arg(&revision)
                .env("GIT_TERMINAL_PROMPT", "0")
                .current_dir(&partial))
            .map_err(|error| {
                std::io::Error::other(format!(
                    "fetching the target `{name}` failed: repository {repository}, revision {revision}: {error}"
                ))
            })?;
        }
        run(Command::new("git")
            .args(["checkout", "--quiet", "--detach"])
            .arg(&revision)
            .current_dir(&partial))?;
        std::fs::rename(&partial, &directory)?;
    }
    let head = output(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&directory),
    )?;
    if head.trim_end() != revision {
        return Err(std::io::Error::other(format!(
            "{} is not at {revision}",
            directory.display()
        )));
    }
    Ok(directory)
}

/// `value` as a path inside the directory it is joined to: refused when it is empty or any
/// component is a root, a prefix, `.` or `..`, so a pin cannot name a place outside its tree.
pub(super) fn relative(value: &str) -> std::io::Result<&Path> {
    let path = Path::new(value);
    if value.is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(std::io::Error::other(format!(
            "not a contained relative path: {value}"
        )));
    }
    Ok(path)
}

/// Copies only ordinary files/directories; symlinks cannot escape either boundary.
#[expect(
    clippy::filetype_is_file,
    reason = "a regular file is exactly what is meant: everything that is neither one nor a directory (a symlink, a socket, a device) is refused"
)]
fn copy(source: &Path, destination: &Path) -> std::io::Result<()> {
    let entries = ignore::WalkBuilder::new(source)
        .hidden(false)
        .ignore(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .parents(false)
        .follow_links(false)
        .build();
    for entry in entries {
        let entry = entry.map_err(std::io::Error::other)?;
        let kind = entry
            .file_type()
            .ok_or_else(|| std::io::Error::other("missing file type"))?;
        if !kind.is_file() && !kind.is_dir() {
            return Err(std::io::Error::other(format!(
                "dependency contains a nonordinary file: {}",
                entry.path().display()
            )));
        }
        let suffix = entry
            .path()
            .strip_prefix(source)
            .map_err(std::io::Error::other)?;
        let target = if suffix.as_os_str().is_empty() {
            destination.to_path_buf()
        } else {
            destination.join(suffix)
        };
        if kind.is_dir() {
            std::fs::create_dir_all(target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let _: u64 = std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// No existing component may be a symlink, and no existing destination is overwritten.
fn destination(project: &Path, path: &Path) -> std::io::Result<PathBuf> {
    let mut destination = project.to_path_buf();
    for part in path.components() {
        destination.push(part);
        match std::fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => (),
            Ok(_) => {
                return Err(std::io::Error::other(
                    "dependency destination is not a directory",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    if destination.exists() {
        return Err(std::io::Error::other(
            "dependency destination already exists",
        ));
    }
    Ok(destination)
}

/// Binds each of the pin's `dependencies` into `project`: a directory is copied to its own
/// place under `external` and linked at its destination, a file is copied there, and each
/// source must lie inside `repository`.
pub(super) fn dependencies(
    repository: &Path,
    project: &Path,
    external: &Path,
    pin: &toml::Table,
) -> std::io::Result<()> {
    let Some(dependencies) = pin.get("dependencies") else {
        return Ok(());
    };
    let dependencies = dependencies
        .as_array()
        .ok_or_else(|| std::io::Error::other("dependencies must be an array"))?;
    let repository = repository.canonicalize()?;
    for (index, dependency) in dependencies.iter().enumerate() {
        let fields = dependency
            .as_table()
            .ok_or_else(|| std::io::Error::other("dependency must be a table"))?;
        if fields.len() != 2 {
            return Err(std::io::Error::other(
                "dependency needs only source and destination",
            ));
        }
        let path = |key: &str| {
            fields
                .get(key)
                .and_then(toml::Value::as_str)
                .ok_or_else(|| std::io::Error::other(format!("dependency needs string {key}")))
                .and_then(relative)
        };
        let source = repository.join(path("source")?).canonicalize()?;
        if !source.starts_with(&repository) {
            return Err(std::io::Error::other(
                "dependency source must be inside the repository",
            ));
        }
        let destination = destination(project, path("destination")?)?;
        if source.is_dir() {
            let isolated = external.join(index.to_string());
            if isolated.exists() {
                return Err(std::io::Error::other("external dependency already exists"));
            }
            copy(&source, &isolated)?;
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            bind(&isolated, &destination)?;
        } else {
            copy(&source, &destination)?;
        }
    }
    Ok(())
}

/// Binds `destination` to the isolated copy of an external dependency, as a symbolic link.
#[cfg(unix)]
fn bind(isolated: &Path, destination: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(isolated, destination)
}

/// Binds `destination` to the isolated copy of an external dependency, as a symbolic link to a directory or a file.
#[cfg(windows)]
fn bind(isolated: &Path, destination: &Path) -> std::io::Result<()> {
    if isolated.is_dir() {
        std::os::windows::fs::symlink_dir(isolated, destination)
    } else {
        std::os::windows::fs::symlink_file(isolated, destination)
    }
}

/// A file system without symbolic links cannot bind an external dependency.
#[cfg(not(any(unix, windows)))]
fn bind(_isolated: &Path, _destination: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "external directory bindings require symlink support",
    ))
}

/// Binds the dependencies of `target`'s pin into `project`, their sources read from this
/// repository's checkout and their isolated copies kept under `external`.
pub(super) fn prepare(project: &Path, target: &Path, external: &Path) -> std::io::Result<()> {
    let repository = output(
        Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(env!("CARGO_MANIFEST_DIR")),
    )?;
    dependencies(
        Path::new(repository.trim_end()),
        project,
        external,
        &table(&target.join("pin.toml"))?,
    )
}

/// A fresh copy of `snapshot` under `scratch`, with the target's dependencies bound: the copy's root and
/// the project the command runs in.
pub(crate) fn checkout(
    target: &Path,
    snapshot: &Path,
    scratch: &Path,
) -> std::io::Result<(PathBuf, PathBuf)> {
    let tree = scratch.join("tree");
    run(Command::new("git")
        .args([
            "clone",
            "--quiet",
            "--local",
            "--config",
            "core.autocrlf=false",
            "--config",
            "core.symlinks=true",
        ])
        .arg(snapshot)
        .arg(&tree))?;
    let directory = pinned(target, "directory")?;
    let within = if directory == "." {
        Path::new(&directory)
    } else {
        relative(&directory)?
    };
    let project = tree.join(within).canonicalize()?;
    if !project.starts_with(tree.canonicalize()?) {
        return Err(std::io::Error::other("project escapes target tree"));
    }
    prepare(&project, target, &scratch.join("dependencies"))?;
    Ok((tree, project))
}
