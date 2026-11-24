//! The same profile pipeline for every registered image and workload.
use super::profile_evidence::EngineEvidence;
use profile_tools::{self as standard, CommandLine, Observer};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Deserialize, Serialize)]
pub(crate) struct Images {
    pub ordinary: PathBuf,
    pub heap: PathBuf,
}

pub(crate) struct Workload<'a> {
    pub name: &'a str,
    pub root: &'a Path,
    pub exits: &'a [i32],
    pub instruction_budget: Duration,
}

// These BMF entries are individual observations with exact integer values.
#[derive(Serialize)]
pub(crate) struct Metric {
    value: u128,
}

type Measurements = BTreeMap<String, BTreeMap<&'static str, Metric>>;

pub(crate) fn profile<T>(
    workload: Workload<'_>,
    first: &Images,
    other: Option<&Images>,
    report: &mut impl Write,
    mut command: impl FnMut(&Path, Option<Observer>, &Path) -> Result<(CommandLine, T), Error>,
    mut engine: impl FnMut(&standard::Observation, T, &Path) -> Result<EngineEvidence, Error>,
) -> Result<Measurements, Error> {
    let mut totals = BTreeMap::new();
    let mut run = |images: &Images, folder: &str| {
        let destination = workload.root.join(folder);
        let mut ordinary = None;
        let profile = standard::collect(
            &destination,
            workload.exits,
            workload.instruction_budget,
            |observer, path| {
                let executable = if observer == Observer::Heap {
                    &images.heap
                } else {
                    &images.ordinary
                };
                let (command, request) = command(executable, Some(observer), path)
                    .map_err(|error| io::Error::other(error.to_string()))?;
                if observer == Observer::Native {
                    ordinary = Some(request);
                }
                Ok(command)
            },
        )?;
        let ordinary = ordinary.ok_or_else(|| Error::Invalid("missing ordinary request".into()))?;
        let detail_path = destination.join("engine");
        fs::create_dir_all(&detail_path)?;
        let (diagnostic, _) = command(&images.ordinary, None, &detail_path)?;
        let diagnostic = standard::observe(&diagnostic, Observer::Native, &detail_path)?;
        if !diagnostic
            .status
            .code()
            .is_some_and(|code| workload.exits.contains(&code))
            || !standard::same_stdout(&profile.native, &diagnostic, Observer::Native)?
        {
            return Err(Error::Invalid(format!(
                "operator observer changed the ordinary output; see {}",
                diagnostic.stderr.display()
            )));
        }
        let evidence = engine(&profile.native, ordinary, &destination)?;
        writeln!(report, "\n# {} — {folder} build\n", workload.name)?;
        writeln!(
            report,
            "Ordinary image `{}`. Heaptrack image `{}` uses Rust's system allocator.\n",
            images.ordinary.display(),
            images.heap.display()
        )?;
        evidence.write(report)?;
        standard::run_table(
            report,
            &[
                ("Ordinary", &profile.native),
                ("Operator observer", &diagnostic),
            ],
        )?;
        standard::report(report, &profile, &destination)?;
        let key = if folder == "first" {
            workload.name.to_owned()
        } else {
            format!("{}/other", workload.name)
        };
        let values = metrics(&profile, &destination)?;
        let _prior = totals.insert(key, values);
        Ok::<_, Error>((profile, evidence, destination))
    };
    let (profile, evidence, path) = run(first, "first")?;
    if let Some(other) = other {
        let (compared, second_evidence, second_path) = run(other, "other")?;
        writeln!(report, "\n## Build comparison\n")?;
        standard::compare(report, (&profile, &path), (&compared, &second_path))?;
        evidence.compare(&second_evidence, report)?;
    }
    Ok(totals)
}

fn metrics(
    profile: &standard::Profile,
    directory: &Path,
) -> Result<BTreeMap<&'static str, Metric>, Error> {
    let heap = standard::heap_totals(directory)?;
    let mut numbers = BTreeMap::from([
        (
            "allocation-count",
            Metric {
                value: u128::from(heap.allocations),
            },
        ),
        ("allocation-bytes", Metric { value: heap.bytes }),
        (
            "peak-rss",
            Metric {
                value: u128::from(standard::peak_rss(profile)?),
            },
        ),
    ]);
    if let Some(total) = standard::instruction_total(profile, directory)? {
        let _old = numbers.insert(
            "instructions",
            Metric {
                value: u128::from(total),
            },
        );
    }
    Ok(numbers)
}
