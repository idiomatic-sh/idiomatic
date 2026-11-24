//! Profile an already built image on a frozen workload.
use super::{
    profile_evidence,
    profile_images::{archive, cargo_images, git_text},
    profile_workload::{self, Images, Workload},
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use profile_tools::{CommandLine, Observer};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufReader, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Deserialize)]
struct Input {
    name: String,
    image: Option<ImageInput>,
    arguments: Vec<String>,
    #[serde(default = "default_exits")]
    exits: Vec<i32>,
    #[serde(default)]
    clock_launcher: Vec<String>,
    #[serde(default)]
    instruction_launcher: Vec<String>,
    #[serde(default)]
    engine: bool,
    oracle: Option<PathBuf>,
    directory: Option<PathBuf>,
    snapshot: Option<PathBuf>,
    fixture: Option<PathBuf>,
    #[serde(default)]
    environment: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImageInput {
    Saved(Images),
    Cargo {
        product: PathBuf,
        allocator_entry: PathBuf,
        binary: String,
    },
}

fn images(input: ImageInput, base: &Path) -> Result<Images, Error> {
    match input {
        ImageInput::Saved(mut images) => {
            images.ordinary = base.join(images.ordinary).canonicalize()?;
            images.heap = base.join(images.heap).canonicalize()?;
            images.diagnostic = images
                .diagnostic
                .map(|path| base.join(path).canonicalize())
                .transpose()?;
            Ok(images)
        }
        ImageInput::Cargo {
            product,
            allocator_entry,
            binary,
        } => {
            let product = base.join(product).canonicalize()?;
            let root = git_text(&product, &["rev-parse", "--show-toplevel"])?;
            let root = Path::new(root.trim());
            let revision = git_text(root, &["rev-parse", "HEAD"])?;
            let image = product
                .join("target/benchmark/profile/images")
                .join(revision.trim())
                .join(&binary);
            let ordinary = image.join("ordinary");
            let heap = image.join("heap");
            if !(ordinary.is_file() && heap.is_file()) {
                fs::create_dir_all(&image)?;
                let stage = tempfile::tempdir_in(&image)?;
                let source = stage.path().join("source");
                archive(root, revision.trim(), &source)?;
                let relative = product
                    .strip_prefix(root)
                    .map_err(|_| Error::Invalid("image source is outside its repository".into()))?;
                cargo_images(
                    &source.join(relative),
                    &allocator_entry,
                    &binary,
                    stage.path(),
                    &image,
                )?;
            }
            Ok(Images {
                ordinary,
                heap,
                diagnostic: None,
            })
        }
    }
}

fn default_exits() -> Vec<i32> {
    vec![0]
}

fn input(path: &Path) -> Result<(Input, Images), Error> {
    let file = if path.is_dir() {
        path.join("profile.toml")
    } else {
        path.to_owned()
    };
    let file = file.canonicalize()?;
    let Some(parent) = file.parent() else {
        return Err(Error::Invalid("profile manifest has no directory".into()));
    };
    let mut input: Input = toml::from_str(&fs::read_to_string(&file)?)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    input.oracle = input.oracle.map(|path| parent.join(path));
    input.fixture = input.fixture.map(|path| parent.join(path));
    if input.fixture.is_some() && (input.snapshot.is_some() || input.directory.is_some()) {
        return Err(Error::Invalid(
            "a fixture supplies its own working directory".into(),
        ));
    }
    if let Some(snapshot) = &mut input.snapshot {
        *snapshot = parent.join(&*snapshot).canonicalize()?;
        if input.directory.as_ref().is_some_and(|path| {
            path.components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        }) {
            return Err(Error::Invalid(
                "snapshot workload directory must be relative".into(),
            ));
        }
    } else {
        input.directory = input.directory.map(|path| parent.join(path));
    }
    let image = input
        .image
        .take()
        .ok_or_else(|| Error::Invalid("workload has no image".into()))?;
    Ok((input, images(image, parent)?))
}

fn isolated(input: &Input, path: &Path) -> io::Result<(PathBuf, Option<PathBuf>)> {
    let checkout = path.join("checkout");
    if let Some(fixture) = &input.fixture {
        fs::create_dir_all(&checkout)?;
        if !Command::new("cp")
            .arg("-a")
            .arg(fixture.join("."))
            .arg(&checkout)
            .status()?
            .success()
        {
            return Err(io::Error::other("could not make a cold workload fixture"));
        }
        return Ok((checkout.clone(), Some(checkout)));
    }
    let Some(source) = &input.snapshot else {
        return Ok((input.directory.as_deref().unwrap_or(path).to_owned(), None));
    };
    let status = Command::new("git")
        .args(["clone", "--quiet", "--local", "--shared"])
        .arg(source)
        .arg(&checkout)
        .stdout(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(io::Error::other(
            "could not create a cold workload checkout",
        ));
    }
    let directory = input
        .directory
        .as_ref()
        .map_or_else(|| checkout.clone(), |directory| checkout.join(directory));
    Ok((directory, Some(checkout)))
}

/// Whether this argument identifies a frozen workload manifest.
#[must_use]
pub fn registered(path: &Path) -> bool {
    path.is_file() || path.join("profile.toml").is_file()
}

/// Run the standard profile on the selected image and optionally on a second build.
///
/// # Errors
/// The workload description, image, tool, output, or oracle check fails.
pub fn profile(
    bench: &Path,
    other: Option<&Path>,
    output: &mut impl Write,
    progress: &mut impl Write,
) -> Result<(), Error> {
    let (first, first_image) = input(bench)?;
    let other = other.map(input).transpose()?;
    if let Some((build, _)) = &other
        && build.name != first.name
    {
        return Err(Error::Invalid(
            "both builds must name the same workload".into(),
        ));
    }
    let run = tempfile::Builder::new()
        .prefix("image-profile-")
        .tempdir_in(std::env::current_dir()?)?
        .keep();
    let report_path = run.join("profile.md");
    let mut report = fs::File::create(&report_path)?;
    let expected = first.oracle.as_deref().map(fs::read).transpose()?;
    let totals = profile_workload::profile(
        Workload {
            name: &first.name,
            root: &run,
            exits: &first.exits,
            has_engine: first.engine,
            instruction_budget: Duration::from_secs(900),
        },
        &first_image,
        other.as_ref().map(|(_, image)| image),
        &mut report,
        |image, observer, path, other_build| {
            fs::create_dir_all(path)?;
            let (directory, workspace_prefix) = isolated(&first, path)?;
            let invocation = if other_build {
                &other.as_ref().expect("comparison image").0
            } else {
                &first
            };
            let answer_path = path.join("answer");
            let answer = invocation
                .arguments
                .iter()
                .any(|arg| arg == "{answer}")
                .then_some(answer_path.clone());
            if observer.is_none() {
                fs::File::create(path.join("ryo-operators.tsv"))?;
            }
            let mut environment: BTreeMap<_, _> = invocation
                .environment
                .iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect();
            environment.insert("TMPDIR".into(), path.as_os_str().to_owned());
            environment.insert("RAYON_NUM_THREADS".into(), "1".into());
            let has_answer = answer.is_some();
            Ok((
                CommandLine {
                    executable: image.to_owned(),
                    arguments: invocation
                        .arguments
                        .iter()
                        .map(|arg| {
                            if arg == "{answer}" {
                                answer_path.as_os_str().to_owned()
                            } else {
                                arg.into()
                            }
                        })
                        .collect(),
                    environment,
                    directory,
                    answer,
                    workspace_prefix,
                    launcher: match observer {
                        Some(Observer::Instructions) => first.instruction_launcher.as_slice(),
                        Some(Observer::Heap) => &[],
                        _ => first.clock_launcher.as_slice(),
                    }
                    .iter()
                    .map(Into::into)
                    .collect(),
                },
                has_answer,
            ))
        },
        |native, has_answer, destination| {
            if let Some(expected) = &expected
                && fs::read(&native.stdout)?.as_slice() != expected
            {
                return Err(Error::Invalid(format!(
                    "archived output differs at {}",
                    native.stdout.display()
                )));
            }
            if !first.engine {
                return Ok(None);
            }
            let mut log = fs::read_to_string(&native.stderr)?;
            if has_answer {
                log.push('\n');
                log.push_str(&fs::read_to_string(destination.join("native/stdout"))?);
            }
            let ledger = match fs::File::open(destination.join("engine/ryo-operators.tsv")) {
                Ok(file) if file.metadata()?.len() != 0 => Some(BufReader::new(file)),
                Ok(_) => None,
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            Ok(Some(profile_evidence::archived(&log, ledger)?))
        },
    )?;
    serde_json::to_writer_pretty(output, &totals)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    writeln!(progress, "Profile tables: {}", report_path.display())?;
    Ok(())
}
