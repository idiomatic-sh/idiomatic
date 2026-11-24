//! The collector's adapter for the registered real check workloads.
use crate::application::profile_evidence;
use profile_tools::{self as standard, CommandLine, Observer};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{self, Write},
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
    let mut totals = BTreeMap::new();
    for (name, workspace) in [("root", source.clone()), ("ryo", source.join("crates/ryo"))] {
        let command = registered_command(&workspace)?;
        let first = directory.join(name).join("first");
        let budget = Duration::from_secs(if name == "root" { 900 } else { 480 });
        let (profile, diagnostic) = check(&image, &workspace, &command, &first, budget)?;
        let first_phases = fs::read_to_string(&profile.native.stderr)?;
        writeln!(written, "\n# {name}\n")?;
        profile_evidence::phases(&first_phases, &mut written)?;
        standard::run_table(
            &mut written,
            &[
                ("Ordinary", &profile.native),
                ("Operator observer", &diagnostic),
            ],
        )?;
        let operators = write_operators(&first, &mut written)?;
        standard::report(&mut written, &profile, &first)?;
        totals.insert(
            name.to_owned(),
            super::corpus_profile::metrics(&profile, &first)?,
        );
        if let Some(other) = &other {
            let second = directory.join(name).join("other");
            let (compared, diagnostic) = check(other, &workspace, &command, &second, budget)?;
            let second_phases = fs::read_to_string(&compared.native.stderr)?;
            writeln!(written, "\n# {name} — second build\n")?;
            profile_evidence::phases(&second_phases, &mut written)?;
            standard::run_table(
                &mut written,
                &[
                    ("Ordinary", &compared.native),
                    ("Operator observer", &diagnostic),
                ],
            )?;
            let other_operators = write_operators(&second, &mut written)?;
            standard::report(&mut written, &compared, &second)?;
            standard::compare(&mut written, (&profile, &first), (&compared, &second))?;
            profile_evidence::phase_delta(&first_phases, &second_phases, &mut written)?;
            operators.compare(&other_operators, &mut written)?;
            totals.insert(
                format!("{name}/other"),
                super::corpus_profile::metrics(&compared, &second)?,
            );
        }
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

fn write_operators(
    directory: &Path,
    output: &mut impl Write,
) -> io::Result<profile_evidence::OperatorProfile> {
    let file = directory.join("engine/ryo-operators.tsv");
    profile_evidence::operators(io::BufReader::new(fs::File::open(file)?), output)
}

fn check(
    images: &Path,
    workspace: &Path,
    arguments: &[OsString],
    destination: &Path,
    budget: Duration,
) -> io::Result<(standard::Profile, standard::Observation)> {
    let cache = workspace.join(".tastemaker_cache");
    let invocation = |observer| -> io::Result<CommandLine> {
        if cache.exists() {
            fs::remove_dir_all(&cache)?;
        }
        let command = arguments
            .iter()
            .cloned()
            .chain(["--workers".into(), "1".into()]);
        Ok(CommandLine {
            executable: images.join(if observer == Observer::Heap {
                "heap"
            } else {
                "ordinary"
            }),
            arguments: command.collect(),
            environment: BTreeMap::from([("RAYON_NUM_THREADS".into(), "1".into())]),
            directory: workspace.to_owned(),
        })
    };
    let profile = standard::collect(destination, &[0, 1], budget, |observer, _| {
        invocation(observer)
    })?;
    let engine = destination.join("engine");
    fs::create_dir_all(&engine)?;
    let mut diagnostic = invocation(Observer::Native)?;
    diagnostic.arguments.push("--timings=chrome".into());
    let _existing = diagnostic
        .environment
        .insert("TMPDIR".into(), engine.as_os_str().to_owned());
    let observed = standard::observe(&diagnostic, Observer::Native, &engine)?;
    if !observed.status.code().is_some_and(|status| status <= 1)
        || !standard::same_stdout(&profile.native, &observed, Observer::Native)?
    {
        return Err(io::Error::other(format!(
            "operator observer changed the ordinary output; see {}",
            observed.stderr.display()
        )));
    }
    Ok((profile, observed))
}

fn registered_command(workspace: &Path) -> Result<Vec<OsString>, Error> {
    let registry = fs::read_to_string(workspace.join("asymptote.toml"))?
        .parse::<toml::Value>()
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
    let pin = git_text(repo, &["show", &format!("{revision}:tastemaker.toml")])?
        .parse::<toml::Value>()
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
