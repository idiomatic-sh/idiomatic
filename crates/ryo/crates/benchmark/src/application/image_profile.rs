//! Profile an already built image on a frozen workload.
use super::{
    profile_evidence,
    profile_workload::{self, Images, Workload},
};
use profile_tools::{CommandLine, Observer};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Deserialize)]
struct Input {
    name: String,
    image: Images,
    arguments: Vec<String>,
    #[serde(default = "default_exits")]
    exits: Vec<i32>,
    #[serde(default)]
    clock_launcher: Vec<String>,
    #[serde(default)]
    instruction_launcher: Vec<String>,
    oracle: Option<PathBuf>,
    directory: Option<PathBuf>,
    #[serde(default)]
    environment: BTreeMap<String, String>,
}

fn default_exits() -> Vec<i32> {
    vec![0]
}

fn input(path: &Path) -> Result<Input, Error> {
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
    input.image.ordinary = parent.join(input.image.ordinary).canonicalize()?;
    input.image.heap = parent.join(input.image.heap).canonicalize()?;
    input.oracle = input.oracle.map(|path| parent.join(path));
    input.directory = input.directory.map(|path| parent.join(path));
    Ok(input)
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
    let first = input(bench)?;
    let other = other.map(input).transpose()?;
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
            instruction_budget: Duration::from_secs(900),
        },
        &first.image,
        other.as_ref().map(|other| &other.image),
        &mut report,
        |image, observer, path| {
            fs::create_dir_all(path)?;
            let answer_path = path.join("answer");
            let answer = first
                .arguments
                .iter()
                .any(|arg| arg == "{answer}")
                .then_some(answer_path.clone());
            if observer.is_none() {
                fs::File::create(path.join("ryo-operators.tsv"))?;
            }
            let mut environment: BTreeMap<_, _> = first
                .environment
                .iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect();
            environment.insert("TMPDIR".into(), path.as_os_str().to_owned());
            environment.insert("RAYON_NUM_THREADS".into(), "1".into());
            Ok((
                CommandLine {
                    executable: image.to_owned(),
                    arguments: first
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
                    directory: first.directory.as_deref().unwrap_or(path).to_owned(),
                    answer,
                    workspace_prefix: None,
                    launcher: match observer {
                        Some(Observer::Instructions) => first.instruction_launcher.as_slice(),
                        Some(Observer::Heap) | None => &[],
                        _ => first.clock_launcher.as_slice(),
                    }
                    .iter()
                    .map(Into::into)
                    .collect(),
                },
                (),
            ))
        },
        |native, (), destination| {
            if let Some(expected) = &expected
                && fs::read(&native.stdout)?.as_slice() != expected
            {
                return Err(Error::Invalid(format!(
                    "archived output differs at {}",
                    native.stdout.display()
                )));
            }
            let log = fs::read_to_string(&native.stderr)?;
            let ledger = match fs::File::open(destination.join("engine/ryo-operators.tsv")) {
                Ok(file) if file.metadata()?.len() != 0 => Some(BufReader::new(file)),
                Ok(_) => None,
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            Ok(profile_evidence::archived(&log, ledger)?)
        },
    )?;
    serde_json::to_writer_pretty(output, &totals)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    writeln!(progress, "Profile tables: {}", report_path.display())?;
    Ok(())
}
