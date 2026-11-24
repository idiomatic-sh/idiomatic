//! Select the ordinary and allocation images for a certified corpus. The
//! collector and its measurement worker share an explicit state directory.
use super::{
    measurement::phases,
    profile_evidence,
    profile_workload::{self, Images, Metric, Workload},
};
use crate::outbound::linter::interface::LinterPort;
use profile_tools::CommandLine;
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, Invocation, ProcessExitCode, ProcessOutput, ResourceObservation,
    TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{self, BufReader},
    path::{Path, PathBuf},
    time::Duration,
};

const PLAN: &str = "profile-plan.toml";
const REPORT: &str = "profile.md";
const METRICS: &str = "profile.bmf";

/// The build pair and output directory for a profile run.
#[derive(Deserialize, Serialize)]
pub(crate) struct CorpusProfile {
    first: Images,
    other: Option<Images>,
    root: PathBuf,
    #[serde(skip)]
    metrics: RefCell<BTreeMap<String, BTreeMap<&'static str, Metric>>>,
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
        candidate: &dyn LinterPort,
        mut invocation: impl FnMut(&Path, Option<&Path>) -> Result<(Invocation, RunRequest), Error>,
    ) -> Result<(), Error> {
        let root = self.root.join(format!("project-{index}"));
        let mut report = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(REPORT))?;
        let totals = profile_workload::profile(
            Workload {
                name,
                root: &root,
                exits: &[0],
                instruction_budget: Duration::from_secs(360),
            },
            &self.first,
            self.other.as_ref(),
            &mut report,
            |image, observer, path| {
                let trace = path.join("events.jsonl");
                let (invoked, request) = invocation(path, observer.is_none().then_some(&trace))?;
                let Invocation {
                    arguments,
                    mut environment,
                    directory,
                    ..
                } = invoked;
                environment.insert("RAYON_NUM_THREADS".into(), "1".into());
                Ok((
                    CommandLine {
                        executable: image.to_owned(),
                        arguments,
                        environment,
                        directory,
                    },
                    request,
                ))
            },
            |ordinary, request, destination| {
                let result = candidate.decode(
                    &request,
                    ProcessOutput {
                        signal: TerminationSignal::Exited,
                        status: ProcessExitCode(0),
                        stdout: CapturedBytes(fs::read(&ordinary.stdout)?),
                        stderr: CapturedBytes(fs::read(&ordinary.stderr)?),
                        wall: ordinary.elapsed,
                        resources: ResourceObservation::WallOnly,
                    },
                )?;
                crate::domain::diagnostics::service::validate_coverage(&request.files, &result)?;
                let clock = phases::read(&destination.join("native/worker-phases.tsv"))?
                    .ok_or_else(|| Error::Invalid(format!(
                        "registered benchmark {name} did not produce ordinary worker phase clocks"
                    )))?;
                let ledger =
                    BufReader::new(fs::File::open(destination.join("engine/events.jsonl"))?);
                Ok(profile_evidence::worker(clock, result.coverage, ledger)?)
            },
        )?;
        self.metrics.borrow_mut().extend(totals);
        Ok(())
    }

    pub(crate) fn finish(&self) -> Result<(), Error> {
        let raw = serde_json::to_vec_pretty(&*self.metrics.borrow())
            .map_err(|error| Error::Invalid(error.to_string()))?;
        fs::write(self.root.join(METRICS), raw)?;
        Ok(())
    }
}
