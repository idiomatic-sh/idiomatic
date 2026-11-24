//! Select the ordinary and allocation images for a certified corpus. The
//! collector and its measurement worker share an explicit state directory.
use super::measurement::phases;
use profile_tools::{self as standard, CommandLine, Observer};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const PLAN: &str = "profile-plan.toml";
const REPORT: &str = "profile.md";
const METRICS: &str = "profile.bmf";

#[derive(Deserialize, Serialize)]
struct Images {
    ordinary: PathBuf,
    heap: PathBuf,
}

/// The build pair and output directory for a profile run.
#[derive(Deserialize, Serialize)]
pub(crate) struct CorpusProfile {
    first: Images,
    other: Option<Images>,
    root: PathBuf,
    #[serde(skip)]
    metrics: RefCell<BTreeMap<String, BTreeMap<&'static str, Metric>>>,
}

// Integers are written exactly. Each tool result is a single observation.
#[derive(Serialize)]
pub(crate) struct Metric {
    value: u128,
    lower_value: u128,
    upper_value: u128,
}

impl From<u128> for Metric {
    fn from(value: u128) -> Self {
        Self {
            value,
            lower_value: value,
            upper_value: value,
        }
    }
}

/// Put a profile's internal plan in the measurement operation's state.
pub(crate) fn prepare(
    root: &Path,
    first: PathBuf,
    heap: PathBuf,
    other: Option<PathBuf>,
) -> Result<PathBuf, Error> {
    let other = other
        .map(|directory| -> Result<Images, Error> {
            let directory = directory.canonicalize()?;
            let ordinary = directory.join("worker");
            let heap = directory.join("heap/worker");
            if !(ordinary.is_file() && heap.is_file()) {
                return Err(Error::Invalid(
                    "other build must be a previous profile directory with both images".into(),
                ));
            }
            Ok(Images { ordinary, heap })
        })
        .transpose()?;
    let plan = CorpusProfile {
        first: Images {
            ordinary: first,
            heap,
        },
        other,
        root: root.to_owned(),
        metrics: RefCell::default(),
    };
    let directory = root.join(crate::domain::operations::constants::STATE);
    fs::create_dir_all(&directory)?;
    let config = toml::to_string(&plan).map_err(|error| Error::Invalid(error.to_string()))?;
    fs::write(directory.join(PLAN), config)?;
    Ok(root.join(METRICS))
}

pub(crate) fn active(directory: &Path) -> Result<Option<CorpusProfile>, Error> {
    match fs::read_to_string(directory.join(PLAN)) {
        Ok(text) => toml::from_str(&text)
            .map(Some)
            .map_err(|error| Error::Invalid(error.to_string())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

impl CorpusProfile {
    pub(crate) fn project(
        &self,
        index: usize,
        name: &str,
        mut invocation: impl FnMut(&Path, Option<&Path>) -> Result<Invocation, Error>,
    ) -> Result<(), Error> {
        let root = self.root.join(format!("project-{index}"));
        let first = root.join("first");
        let other = root.join("other");
        let mut collect = |images: &Images, destination: &Path| -> Result<_, Error> {
            let profile = standard::collect(
                destination,
                &[0],
                Duration::from_secs(360),
                |observer, path| {
                    let Invocation {
                        arguments,
                        mut environment,
                        directory,
                        ..
                    } = invocation(path, None)
                        .map_err(|error| io::Error::other(error.to_string()))?;
                    environment.insert("RAYON_NUM_THREADS".into(), "1".into());
                    let executable = if observer == Observer::Heap {
                        &images.heap
                    } else {
                        &images.ordinary
                    };
                    Ok(CommandLine {
                        executable: executable.clone(),
                        arguments,
                        environment,
                        directory,
                    })
                },
            )?;
            let diagnostic_dir = destination.join("engine");
            let trace = diagnostic_dir.join("events.jsonl");
            let Invocation {
                arguments,
                mut environment,
                directory,
                ..
            } = invocation(&diagnostic_dir, Some(&trace))?;
            environment.insert("RAYON_NUM_THREADS".into(), "1".into());
            let diagnostic = standard::observe(
                &CommandLine {
                    executable: images.ordinary.clone(),
                    arguments,
                    environment,
                    directory,
                },
                Observer::Native,
                &diagnostic_dir,
            )?;
            if !diagnostic.status.success()
                || !standard::same_stdout(&profile.native, &diagnostic, Observer::Native)?
            {
                return Err(Error::Invalid(format!(
                    "engine observer changed the ordinary output; see {}",
                    diagnostic.stderr.display()
                )));
            }
            let clock_path = phases::observation_file(&destination.join("native/state"))
                .ok_or_else(|| Error::Invalid("missing native observation directory".into()))?;
            Ok((profile, trace, phases::read(&clock_path)?))
        };
        let (profile, trace, clock) = collect(&self.first, &first)?;
        if clock.is_none() {
            return Err(Error::Invalid(format!(
                "registered benchmark {name} did not produce ordinary worker phase clocks"
            )));
        }
        let mut report = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(REPORT))?;
        writeln!(report, "\n# {name}\n")?;
        writeln!(
            report,
            "Ordinary image `{}`. Heap-compatible image `{}`.",
            self.first.ordinary.display(),
            self.first.heap.display()
        )?;
        writeln!(report, "\n## Ordinary worker phase clocks\n")?;
        phases::table(&mut report, clock, None)?;
        let operators = super::profile_evidence::linter(
            io::BufReader::new(fs::File::open(trace)?),
            &mut report,
        )?;
        standard::report(&mut report, &profile, &first)?;
        self.record(name.to_owned(), &profile, &first)?;
        if let Some(images) = &self.other {
            let (compared, trace, second_clock) = collect(images, &other)?;
            writeln!(report, "\n# {name} — second build\n")?;
            writeln!(
                report,
                "Ordinary image `{}`. Heap-compatible image `{}`.",
                images.ordinary.display(),
                images.heap.display()
            )?;
            writeln!(report, "\n## Ordinary worker phase clocks\n")?;
            phases::table(&mut report, second_clock, None)?;
            let other_operators = super::profile_evidence::linter(
                io::BufReader::new(fs::File::open(trace)?),
                &mut report,
            )?;
            standard::report(&mut report, &compared, &other)?;
            self.record(format!("{name}/other"), &compared, &other)?;
            writeln!(report, "\n## Build comparison\n")?;
            standard::compare(&mut report, (&profile, &first), (&compared, &other))?;
            if second_clock.is_some() {
                phases::table(&mut report, clock, second_clock)?;
            }
            operators.compare(&other_operators, &mut report)?;
        }
        Ok(())
    }

    fn record(&self, name: String, profile: &standard::Profile, path: &Path) -> Result<(), Error> {
        let numbers = metrics(profile, path)?;
        let _old = self.metrics.borrow_mut().insert(name, numbers);
        Ok(())
    }

    pub(crate) fn finish(&self) -> Result<(), Error> {
        let raw = serde_json::to_vec_pretty(&*self.metrics.borrow())
            .map_err(|error| Error::Invalid(error.to_string()))?;
        fs::write(self.root.join(METRICS), raw)?;
        Ok(())
    }
}

pub(crate) fn metrics(
    profile: &standard::Profile,
    path: &Path,
) -> Result<BTreeMap<&'static str, Metric>, Error> {
    let heap = standard::heap_totals(path)?;
    let mut numbers = BTreeMap::from([
        (
            "allocation-count",
            Metric::from(u128::from(heap.allocations)),
        ),
        ("allocation-bytes", Metric::from(heap.bytes)),
        (
            "peak-rss",
            Metric::from(u128::from(standard::peak_rss(profile)?)),
        ),
    ]);
    if let Some(total) = standard::instruction_total(profile, path)? {
        let _old = numbers.insert("instructions", Metric::from(u128::from(total)));
    }
    Ok(numbers)
}
