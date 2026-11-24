//! The collector's adapter for the registered real check workloads.
use super::{
    profile_evidence,
    profile_workload::{self, Images, Workload},
};
use profile_tools::CommandLine;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

/// Whether the bench directory supplies the pinned live check build.
#[must_use]
pub fn registered(bench: &Path) -> bool {
    bench.join("project/live/link-release.py").is_file()
}

/// Profile the Root and Ryo CI commands over the same cold source snapshot.
///
/// # Errors
/// A registered source or image is absent, a tool fails, or reports differ.
pub fn profile(
    bench: &Path,
    target: &Path,
    other: Option<&Path>,
    output: &mut impl Write,
    progress: &mut impl Write,
) -> Result<(), Error> {
    let bench = bench.canonicalize()?;
    let repo = git_text(&bench, &["rev-parse", "--show-toplevel"])?;
    let repo = Path::new(repo.trim());
    let revision = git_text(repo, &["rev-parse", "HEAD"])?;
    let root = target.join("benchmark/check/profile");
    fs::create_dir_all(&root)?;
    let image = root.join("images").join(revision.trim());
    let other = other
        .map(|directory| -> Result<_, Error> {
            let directory = directory.canonicalize()?;
            if !(directory.join("ordinary").is_file() && directory.join("heap").is_file()) {
                return Err(Error::Invalid(
                    "other build must contain both saved Tastemaker images".into(),
                ));
            }
            Ok(directory)
        })
        .transpose()?;
    let cached = image.join("ordinary").is_file() && image.join("heap").is_file();
    if !cached {
        build(repo, &bench, &image, revision.trim())?;
    }
    let directory = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(&root)?
        .keep();
    let source = directory.join("workspace");
    run(Command::new("git")
        .args(["clone", "--quiet", "--local", "--shared"])
        .arg(repo)
        .arg(&source))?;
    run(Command::new("git").current_dir(&source).args([
        "checkout",
        "--quiet",
        "--detach",
        revision.trim(),
    ]))?;
    let mut written = fs::File::create(directory.join("profile.md"))?;
    writeln!(written, "# Real check at {}\n", revision.trim())?;
    writeln!(
        written,
        "The ordinary release allocator supplies the phase clocks, samples and instructions. The same source built with Rust's system allocator supplies Heaptrack data.\n"
    )?;
    writeln!(written, "First build images: `{}`.\n", image.display())?;
    if let Some(other) = &other {
        writeln!(written, "Second build images: `{}`.\n", other.display())?;
    }
    let ordinary = Images {
        ordinary: image.join("ordinary"),
        heap: image.join("heap"),
        diagnostic: None,
    };
    let other = other.map(|directory| Images {
        ordinary: directory.join("ordinary"),
        heap: directory.join("heap"),
        diagnostic: None,
    });
    let mut totals = BTreeMap::new();
    for (name, workspace) in [("root", source.clone()), ("ryo", source.join("crates/ryo"))] {
        let arguments = registered_command(&workspace)?;
        let root = directory.join(name);
        let relative = workspace
            .strip_prefix(&source)
            .map_err(|_| Error::Invalid("workload is outside the checkout".into()))?;
        let budget = Duration::from_secs(if name == "root" { 900 } else { 480 });
        totals.extend(profile_workload::profile(
            Workload {
                name,
                root: &root,
                exits: &[0, 1],
                has_engine: true,
                instruction_budget: budget,
            },
            &ordinary,
            other.as_ref(),
            &mut written,
            |image, observer, path| {
                fs::create_dir_all(path)?;
                let copy = path.join("workspace");
                run(Command::new("git")
                    .args(["clone", "--quiet", "--local", "--shared"])
                    .arg(&source)
                    .arg(&copy))?;
                run(Command::new("git").current_dir(&copy).args([
                    "checkout",
                    "--quiet",
                    "--detach",
                    revision.trim(),
                ]))?;
                let command: Vec<_> = arguments
                    .iter()
                    .cloned()
                    .chain(["--workers".into(), "1".into()])
                    .collect();
                let mut environment = BTreeMap::from([("RAYON_NUM_THREADS".into(), "1".into())]);
                if observer.is_none() {
                    fs::File::create(path.join("ryo-operators.tsv"))?;
                    let _prior = environment.insert("TMPDIR".into(), path.as_os_str().to_owned());
                }
                Ok((
                    CommandLine {
                        executable: image.to_owned(),
                        answer: None,
                        workspace_prefix: Some(copy.clone()),
                        launcher: Vec::new(),
                        arguments: command,
                        environment,
                        directory: copy.join(relative),
                    },
                    (),
                ))
            },
            |native, (), destination| {
                let log = fs::read_to_string(&native.stderr)?;
                let ledger = BufReader::new(fs::File::open(
                    destination.join("engine/ryo-operators.tsv"),
                )?);
                Ok(Some(profile_evidence::live(&log, Some(ledger))?))
            },
        )?);
    }
    serde_json::to_writer_pretty(output, &totals)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    writeln!(
        progress,
        "Profile tables: {}",
        directory.join("profile.md").display()
    )?;
    writeln!(progress, "Profile images: {}", image.display())?;
    Ok(())
}

fn registered_command(workspace: &Path) -> Result<Vec<OsString>, Error> {
    let registry =
        toml::from_str::<toml::Value>(&fs::read_to_string(workspace.join("asymptote.toml"))?)
            .map_err(|error| Error::Invalid(error.to_string()))?;
    let tool = registry
        .get("tools")
        .and_then(toml::Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|item| item.get("id").and_then(toml::Value::as_str) == Some("tastemaker"))
        });
    let run = tool
        .and_then(|tool| tool.get("run"))
        .and_then(toml::Value::as_array)
        .ok_or_else(|| Error::Invalid("workspace has no registered Tastemaker check".into()))?;
    if run.first().and_then(toml::Value::as_str) != Some("tm") {
        return Err(Error::Invalid("registered check does not launch tm".into()));
    }
    run.iter()
        .skip(1)
        .map(|argument| {
            argument
                .as_str()
                .map(Into::into)
                .ok_or_else(|| Error::Invalid("non-text check argument".into()))
        })
        .collect()
}

fn build(repo: &Path, bench: &Path, image: &Path, revision: &str) -> Result<(), Error> {
    fs::create_dir_all(image)?;
    let stage = tempfile::tempdir_in(image)?;
    let pin = toml::from_str::<toml::Value>(&git_text(
        repo,
        &["show", &format!("{revision}:tastemaker.toml")],
    )?)
    .map_err(|error| Error::Invalid(error.to_string()))?;
    let version = pin
        .get("tastemaker")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::Invalid("missing pinned Tastemaker version".into()))?;
    let tag = format!("idiomatic-sh-tastemaker-v{version}");
    let source = stage.path().join("source");
    let release = stage.path().join("release");
    archive(repo, revision, &source)?;
    archive(repo, &tag, &release)?;
    let bench = bench
        .strip_prefix(repo)
        .map_err(|_| Error::Invalid("bench is outside its source checkout".into()))?;
    run(Command::new("python3")
        .args(["-I"])
        .arg(source.join(bench).join("project/live/link-release.py"))
        .arg(&release))?;
    let product = release.join("crates/tastemaker");
    let target = stage.path().join("target");
    let make = || {
        let mut command = Command::new("cargo");
        command
            .current_dir(&product)
            .env("CARGO_PROFILE_BENCH_DEBUG", "line-tables-only")
            .args([
                "build",
                "--offline",
                "--profile",
                "bench",
                "--bin",
                "tm",
                "--target-dir",
            ])
            .arg(&target);
        command
    };
    run(&mut make())?;
    let ordinary = stage.path().join("ordinary");
    fs::copy(target.join("release/tm"), &ordinary)?;
    let entry = product.join("cli/src/main.rs");
    let mut text = fs::read_to_string(&entry)?;
    for declaration in [
        "static GLOBAL: mimalloc::MiMalloc",
        "static GLOBAL: tikv_jemallocator::Jemalloc",
    ] {
        let (_, after) = text
            .split_once(declaration)
            .ok_or_else(|| Error::Invalid("pinned allocator declaration moved".into()))?;
        let start = text
            .find(declaration)
            .ok_or_else(|| Error::Invalid("allocator declaration".into()))?;
        let docs = text
            .get(..start)
            .and_then(|prefix| prefix.rfind("/// The allocator"))
            .ok_or_else(|| Error::Invalid("allocator declaration has unexpected shape".into()))?;
        let end = text.len() - after.len()
            + after
                .find('\n')
                .ok_or_else(|| Error::Invalid("allocator declaration is incomplete".into()))?;
        text.replace_range(docs..end, "");
    }
    fs::write(entry, text)?;
    run(&mut make())?;
    let heap = stage.path().join("heap");
    fs::copy(target.join("release/tm"), &heap)?;
    fs::rename(ordinary, image.join("ordinary"))?;
    fs::rename(heap, image.join("heap"))?;
    Ok(())
}

fn archive(repo: &Path, revision: &str, directory: &Path) -> Result<(), Error> {
    fs::create_dir_all(directory)?;
    let archive = directory.join("source.tar");
    run(Command::new("git")
        .current_dir(repo)
        .arg("archive")
        .arg("-o")
        .arg(&archive)
        .args([revision, "crates", "rust-toolchain.toml"]))?;
    run(Command::new("tar")
        .current_dir(directory)
        .arg("-xf")
        .arg(&archive))?;
    fs::remove_file(archive)?;
    Ok(())
}

fn git_text(directory: &Path, arguments: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(Error::Invalid(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| Error::Invalid(error.to_string()))
}

fn run(command: &mut Command) -> Result<(), Error> {
    let status = command
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{command:?} exited {status}")))
    }
}
