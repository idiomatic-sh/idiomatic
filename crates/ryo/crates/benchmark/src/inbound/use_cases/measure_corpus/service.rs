//! Every project is measured in both orders, cold, retained and edited; each pair is judged rule by rule against the
//! reference, and the candidate's costs, the verification times and the growth are the bench's one document.
use super::constants::{
    ACCEPTED_DIRECTORY, ACCEPTED_SUFFIX, CANDIDATE_EDITED, CANDIDATE_FIRST, CANDIDATE_RETAINED,
    CANDIDATE_STATE, CLEAN_EDITED_DIRECTORY, COHORT_OUTSIDE_CERTIFICATE, EDITED_CACHE_CONDITION,
    EMPTY_CACHE_CONDITION, GROWTH_INCOMPLETE, NO_RESOURCES, NO_SOURCE, NOT_COMPLETE, ONE_FINDING,
    PATH_SEPARATOR, REFERENCE_EDITED, REFERENCE_FIRST, REFERENCE_RETAINED, REFERENCE_STATE,
    RETAINED_CACHE_CONDITION, RETAINED_DIRECTORY, SCENES_DIRECTORY, TRIAL_PREFIX, TRIALS,
    UNACCEPTED,
};
use super::types::{
    ComparisonName, CorpusBoundaries, CorpusMeasurement, FindingCounts, SourceTexts,
};
use crate::application::certified_measurement::service::{
    admit, conclude, require_covered, unjudged,
};
use crate::application::measurement::service::run as execute;
use crate::application::project_request::types::ProjectCorpus;
use crate::domain::corpus_discrepancy::types::AdmittedDifferences;
use crate::domain::measurements::types::{MeasurementEligibility, pair::value::Pair};
use crate::domain::source_validity::constants::PREFLIGHT_DIRECTORY;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet, ArtifactText};
use ryo_benchmark_contracts::domain::costs::types::{
    ProjectCost, ProjectCosts, RunCost, StoreSize,
};
use ryo_benchmark_contracts::domain::measurements::types::{
    Measurement, ResourceObservation, RunOrder,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, EndPair, ExecutionMetadata, FindingPosition, LocatedFinding, RuleComparison,
    RunReport, SourceByteOffset,
};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, RunRequest, WorkerRequest,
};
use ryo_benchmark_contracts::domain::workloads::types::Project;
use std::path::{Path, PathBuf};

/// Refuse a cohort outside the certificate, measure every project of the corpus, then publish the samples of an accounted run or say that a diagnostic one is not a metric.
pub(super) fn measure_corpus(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let covered = corpus.certificate.coverage.authored();
    if let CohortSelection::Explicit(rules) = corpus.cohort
        && !rules.iter().all(|rule| covered.contains(rule))
    {
        return Err(Error::Invalid(COHORT_OUTSIDE_CERTIFICATE.into()));
    }
    let selected: Vec<ryo::linter::RuleId> = match corpus.cohort {
        CohortSelection::Explicit(rules) => rules.clone(),
        CohortSelection::Frozen => covered.iter().cloned().collect(),
    };
    crate::application::input_metadata::service::admit_rules(
        boundaries.suite,
        boundaries.host,
        corpus.bench,
        &selected,
    )?;
    let mut costs = Vec::new();
    let mut failures = Vec::new();
    for project in &corpus.projects {
        match measure_project(boundaries, corpus, project, &selected, observations) {
            Ok(cost) => costs.push(cost),
            Err(error) => {
                writeln!(
                    observations,
                    "{}: corpus comparison failed: {error}",
                    project.name
                )?;
                failures.push(project.name.clone());
            }
        }
    }
    if !failures.is_empty() {
        return Err(Error::Invalid(format!(
            "corpus comparisons failed: {failures:?}"
        )));
    }
    let evidence = boundaries.evidence;
    evidence.costs(corpus.language.id, &ProjectCosts(costs))?;
    let timings = evidence.read_verification(corpus.verification)?;
    if !timings.0.is_empty() {
        evidence.verification(corpus.language.id, &timings)?;
    }
    if let crate::domain::growth::types::CorpusGrowth::Grown(source) = corpus.growth {
        let plan = crate::application::growth::service::plan(boundaries.suite, source)?;
        let scenes = boundaries
            .suite
            .ensure_directory(ArtifactPath(&corpus.cache.join(SCENES_DIRECTORY)))?;
        let lint = |request: &WorkerRequest| lint(boundaries, request);
        let grown = crate::application::growth::service::grow(
            &lint,
            boundaries.suite,
            boundaries.meter,
            ArtifactPath(&scenes),
            &plan,
        )?;
        evidence.growth(corpus.language.id, &grown)?;
    }
    evidence.finish()
}

/// One growth lint, as the worker runs a request: prepare, load, configure and execute; refused unless it completes.
fn lint(boundaries: &CorpusBoundaries<'_>, request: &WorkerRequest) -> Result<(), Error> {
    let prepared = crate::application::worker_request::service::prepare(request.clone())?;
    let session = boundaries.engine.load(&prepared.request)?;
    let configured =
        crate::application::worker_request::service::configure(&prepared.request, session)?;
    if configured.execute()?.report.execution_metadata == ExecutionMetadata::Complete {
        Ok(())
    } else {
        Err(Error::Invalid(GROWTH_INCOMPLETE.into()))
    }
}

/// Measure one project in both orders, cold, retained and edited; judge every pair against the reference; and give the
/// candidate's cost.
fn measure_project(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    project: &Project,
    selected: &[ryo::linter::RuleId],
    observations: &mut dyn std::io::Write,
) -> Result<ProjectCost, Error> {
    let language = &corpus.language;
    let mut workspace = boundaries.workspace.prepare(project)?;
    let discovered = boundaries.discovery.files(workspace.root())?;
    let (mut files, excluded): (Vec<_>, Vec<_>) = discovered.into_iter().partition(|file| {
        language
            .suffixes
            .iter()
            .any(|suffix| file.0.ends_with(suffix.0))
    });
    writeln!(
        observations,
        "{}: matched {} source scope {} files; excluded default-discovered formats {} files (source-kind capability gap): {}",
        project.name,
        language.display.0,
        files.len(),
        excluded.len(),
        excluded
            .iter()
            .map(|path| path.0.as_ref())
            .collect::<Vec<_>>()
            .join(PATH_SEPARATOR)
    )?;
    let tracked = boundaries.workspace.tracked_files(&workspace)?.0;
    let tracked_paths: Vec<_> = tracked.iter().map(PathBuf::from).collect();
    if files.iter().any(|file| {
        !tracked
            .iter()
            .any(|tracked| tracked.as_str() == file.0.as_ref())
    }) {
        return Err(Error::Invalid(format!(
            "{} discovered untracked source",
            language.reference_display.0
        )));
    }
    let preflight = request(
        boundaries,
        corpus,
        workspace.root(),
        &files,
        &tracked_paths,
        corpus.cache.join(PREFLIGHT_DIRECTORY).join(&project.name.0),
    )?;
    let parsed = run_linter(boundaries, boundaries.reference, &preflight)?;
    let refused = crate::domain::source_validity::service::exclude_refused(
        &parsed.report,
        language.invalid_syntax,
        &mut files,
    );
    for finding in &refused {
        writeln!(
            observations,
            "{}: excluded invalid {} (pinned {} parser), path={} range={}..{}; frontend parser capability gap",
            project.name,
            language.display.0,
            language.reference_display.0,
            finding.file.0,
            finding.start.0,
            finding.end.0
        )?;
    }
    // A project with nothing left to lint would pass every comparison and measure nothing.
    if files.is_empty() {
        return Err(Error::Invalid(NO_SOURCE.into()));
    }
    let paths: Vec<_> = files
        .iter()
        .map(|file| workspace.root().join(file.0.as_ref()))
        .collect();
    let bytes = boundaries.suite.total_size(ArtifactSet(&paths))?.0;
    writeln!(
        observations,
        "{}: measured source files={} bytes={bytes} digest={} input-profile={}",
        project.name,
        files.len(),
        boundaries.suite.fingerprint(ArtifactSet(&paths))?,
        language.valid_profile.0
    )?;
    let accepted_file = accepted_file(boundaries, corpus, project)?;
    let mut runs: [Vec<Measurement>; 3] = Default::default();
    let mut store = None;
    for order in [RunOrder::ReferenceFirst, RunOrder::CandidateFirst]
        .into_iter()
        .cycle()
        .take(TRIALS)
    {
        // A new owned directory makes both retained and clean baselines fresh,
        // even if an earlier trial left caches behind in the caller's root.
        let trial = boundaries
            .suite
            .retained_directory(ArtifactPath(corpus.cache), ArtifactText(TRIAL_PREFIX))?
            .0;
        let initial = request(
            boundaries,
            corpus,
            workspace.root(),
            &files,
            &tracked_paths,
            trial.join(RETAINED_DIRECTORY),
        )?;
        let accepted = admitted(
            boundaries,
            accepted_file.as_deref(),
            &initial,
            Strictness::Pinned,
        )?;
        let cold = measured(boundaries, corpus, &initial, order)?;
        if store.is_none() {
            store = Some(
                boundaries
                    .suite
                    .directory_size(ArtifactPath(&initial.state.join(CANDIDATE_STATE)))?,
            );
        }
        writeln!(
            observations,
            "{}: exact matching findings per rule: {}",
            project.name,
            finding_counts(&cold.reference().report)
        )?;
        judged(
            boundaries,
            &initial,
            (selected, language.invalid_syntax),
            &accepted,
            &cold,
            observations,
        )?;
        let retained = measured(boundaries, corpus, &initial, order)?;
        agree(
            ComparisonName(REFERENCE_RETAINED),
            &cold.reference().report,
            &retained.reference().report,
        )?;
        agree(
            ComparisonName(CANDIDATE_RETAINED),
            &cold.candidate().report,
            &retained.candidate().report,
        )?;
        judged(
            boundaries,
            &initial,
            (selected, language.invalid_syntax),
            &accepted,
            &retained,
            observations,
        )?;
        boundaries
            .workspace
            .apply_patch(&mut workspace, &corpus.frozen_workload.join(&project.patch))?;
        let edited_request = request(
            boundaries,
            corpus,
            workspace.root(),
            &files,
            &tracked_paths,
            initial.state.clone(),
        )?;
        let clean_request = RunRequest {
            state: trial.join(CLEAN_EDITED_DIRECTORY),
            ..edited_request.clone()
        };
        let edited = measured(boundaries, corpus, &edited_request, order);
        let clean_reference = run_linter(
            boundaries,
            boundaries.reference,
            &own(&clean_request, REFERENCE_STATE),
        );
        let clean_candidate = run_linter(
            boundaries,
            boundaries.candidate,
            &own(&clean_request, CANDIDATE_STATE),
        );
        let edited_accepted = admitted(
            boundaries,
            accepted_file.as_deref(),
            &edited_request,
            Strictness::WhereNamed,
        );
        boundaries.workspace.reset(&mut workspace)?;
        let edited = edited?;
        agree(
            ComparisonName(REFERENCE_EDITED),
            &edited.reference().report,
            &clean_reference?.report,
        )?;
        agree(
            ComparisonName(CANDIDATE_EDITED),
            &edited.candidate().report,
            &clean_candidate?.report,
        )?;
        judged(
            boundaries,
            &edited_request,
            (selected, language.invalid_syntax),
            &edited_accepted?,
            &edited,
            observations,
        )?;
        for ((condition, measured_pair), condition_runs) in [
            (EMPTY_CACHE_CONDITION, cold),
            (RETAINED_CACHE_CONDITION, retained),
            (EDITED_CACHE_CONDITION, edited),
        ]
        .into_iter()
        .zip(&mut runs)
        {
            if measured_pair.eligibility() != MeasurementEligibility::CompleteInventory {
                return Err(Error::Invalid(NOT_COMPLETE.into()));
            }
            for (contestant, measurement) in [
                (language.reference_display.0, measured_pair.reference()),
                (
                    ryo_benchmark_contracts::domain::reports::constants::CANDIDATE_NAME,
                    measured_pair.candidate(),
                ),
            ] {
                writeln!(
                    observations,
                    "findings: project={} contestant={contestant} condition={condition} counts={}",
                    project.name,
                    finding_counts(&measurement.report)
                )?;
                observe(
                    observations,
                    &format!("{}/{contestant}/{condition}", project.name),
                    measurement,
                    order,
                )?;
            }
            condition_runs.push(measured_pair.candidate().clone());
        }
    }
    let [cold, warm, edited] = runs;
    Ok(ProjectCost {
        project: project.name.clone(),
        cold: cost(&cold)?,
        warm: cost(&warm)?,
        edited: cost(&edited)?,
        store: store.unwrap_or(StoreSize::default()),
    })
}

/// The candidate's CPU, wall and peak of one condition across the trials.
fn cost(runs: &[Measurement]) -> Result<RunCost, Error> {
    let mut cost = RunCost {
        cpu: Vec::new(),
        wall: Vec::new(),
        peak: Vec::new(),
    };
    for run in runs {
        let ResourceObservation::Collected(resources) = run.resources else {
            return Err(Error::Invalid(NO_RESOURCES.into()));
        };
        cost.cpu.push(resources.cpu);
        cost.wall.push(run.wall);
        cost.peak.push(resources.peak_rss_bytes);
    }
    Ok(cost)
}

/// The file naming the project's accepted differences, `<bench>/profiles/evidence/<project>-accepted-differences.json`, if it
/// has one.
fn accepted_file(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    project: &Project,
) -> Result<Option<PathBuf>, Error> {
    let mut name = project.name.0.clone();
    name.push_str(ACCEPTED_SUFFIX);
    let file = corpus.bench.join(ACCEPTED_DIRECTORY).join(name);
    match boundaries.suite.digest(ArtifactPath(&file)) {
        Ok(_digest) => Ok(Some(file)),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Whether accepted differences written for another corpus refuse the run.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Strictness {
    /// They do: the project's own corpus must be the one they were written for.
    Pinned,
    /// They do not: they hold only where they name this corpus (the edited sources are another corpus).
    WhereNamed,
}

/// The differences accepted for the corpus `request` reads.
fn admitted(
    boundaries: &CorpusBoundaries<'_>,
    file: Option<&Path>,
    request: &RunRequest,
    strictness: Strictness,
) -> Result<AdmittedDifferences, Error> {
    let Some(file) = file else {
        return Ok(AdmittedDifferences::default());
    };
    let declared = boundaries.suite.accepted_differences(ArtifactPath(file))?;
    let corpus = crate::domain::corpus_discrepancy::service::corpus_digest(
        &request.files,
        &request.context_manifest,
    );
    if strictness == Strictness::WhereNamed && declared.corpus != corpus {
        return Ok(AdmittedDifferences::default());
    }
    Ok(crate::domain::corpus_discrepancy::service::admit_accepted(
        &declared, &corpus,
    )?)
}

/// Judge one pair rule by rule against the accepted differences, refusing every unaccepted or stale difference.
fn judged(
    boundaries: &CorpusBoundaries<'_>,
    request: &RunRequest,
    (selected, syntax): (&[ryo::linter::RuleId], &ryo::linter::RuleId),
    accepted: &AdmittedDifferences,
    measured_pair: &Pair,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let (reference, candidate) = (
        &measured_pair.reference().report,
        &measured_pair.candidate().report,
    );
    let mut texts = SourceTexts::default();
    let located =
        crate::domain::diagnostics::service::compare_rules(selected, reference, candidate)
            .iter()
            .map(|comparison| locate(boundaries, &request.root, &mut texts, comparison))
            .collect::<Result<Vec<_>, _>>()?;
    let problems = crate::domain::corpus_discrepancy::service::judge(
        &located,
        selected.len(),
        accepted,
        [reference, candidate],
        syntax,
    );
    if problems.is_empty() {
        return Ok(());
    }
    for problem in &problems {
        writeln!(observations, "{}", problem.0)?;
    }
    Err(Error::Invalid(format!(
        "{} {UNACCEPTED}: {}",
        problems.len(),
        problems
            .iter()
            .map(|problem| problem.0.as_str())
            .collect::<Vec<_>>()
            .join(PATH_SEPARATOR)
    )))
}

/// The line and column of one byte offset of a corpus file, reading the file once.
fn position(
    boundaries: &CorpusBoundaries<'_>,
    root: &Path,
    texts: &mut SourceTexts,
    file: &ryo::linter::RelativePath,
    offset: SourceByteOffset,
) -> Result<FindingPosition, Error> {
    let source = match texts.0.entry(file.clone()) {
        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::btree_map::Entry::Vacant(entry) => entry.insert(
            boundaries
                .suite
                .source_text(ArtifactPath(&root.join(file.0.as_ref())))?,
        ),
    };
    crate::domain::diagnostics::service::position(source, offset)
}

/// Every finding of one rule's comparison placed by line and column.
fn locate(
    boundaries: &CorpusBoundaries<'_>,
    root: &Path,
    texts: &mut SourceTexts,
    comparison: &RuleComparison<BenchmarkFinding>,
) -> Result<RuleComparison<LocatedFinding>, Error> {
    let mut place = |finding: &BenchmarkFinding| -> Result<LocatedFinding, Error> {
        Ok(LocatedFinding {
            file: finding.file.clone(),
            start: position(boundaries, root, texts, &finding.file, finding.start)?,
            end: position(boundaries, root, texts, &finding.file, finding.end)?,
        })
    };
    let missing = comparison
        .missing
        .iter()
        .map(&mut place)
        .collect::<Result<_, _>>()?;
    let extra = comparison
        .extra
        .iter()
        .map(&mut place)
        .collect::<Result<_, _>>()?;
    let end_differences = comparison
        .end_differences
        .iter()
        .map(|pair| {
            Ok(EndPair {
                candidate: place(&pair.candidate)?,
                reference: place(&pair.reference)?,
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(RuleComparison {
        rule: comparison.rule.clone(),
        matched: comparison.matched,
        missing,
        extra,
        end_differences,
    })
}

/// Require two reports to agree in full, naming what was compared when they differ.
fn agree(what: ComparisonName, first: &RunReport, second: &RunReport) -> Result<(), Error> {
    crate::domain::diagnostics::service::compare_conditions(first, second)
        .map_err(|difference| Error::Invalid(format!("{} mismatch: {difference:?}", what.0)))
}

/// Run one linter over the request through the shared measurement.
fn run_linter(
    boundaries: &CorpusBoundaries<'_>,
    linter: &dyn LinterPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    execute(boundaries.context, linter, boundaries.execution, request)
}

/// The request with its state in the contestant's own directory below the request's, so each store is its own.
fn own(request: &RunRequest, contestant: &str) -> RunRequest {
    RunRequest {
        state: request.state.join(contestant),
        ..request.clone()
    }
}

/// Both contestants measured under the certificate, in the requested order, each in its own state.
fn measured(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    let baseline = admit(
        corpus.certificate,
        corpus.mode,
        boundaries.suite,
        boundaries.execution,
        boundaries.reference,
        boundaries.candidate,
        request,
    )?;
    require_covered(corpus.certificate)?;
    let reference_request = own(request, REFERENCE_STATE);
    let candidate_request = own(request, CANDIDATE_STATE);
    let (reference, candidate) = match order {
        RunOrder::ReferenceFirst => (
            run_linter(boundaries, boundaries.reference, &reference_request)?,
            run_linter(boundaries, boundaries.candidate, &candidate_request)?,
        ),
        RunOrder::CandidateFirst => {
            let candidate = run_linter(boundaries, boundaries.candidate, &candidate_request)?;
            (
                run_linter(boundaries, boundaries.reference, &reference_request)?,
                candidate,
            )
        }
    };
    let measured_pair = unjudged(reference, candidate);
    conclude(
        corpus.certificate,
        corpus.mode,
        boundaries.suite,
        boundaries.execution,
        &baseline,
        measured_pair,
    )
}

/// One run of the corpus project over its explicit files, built by the one project request builder.
fn request(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    root: &Path,
    files: &[ryo::linter::RelativePath],
    tracked: &[PathBuf],
    state: PathBuf,
) -> Result<RunRequest, Error> {
    crate::application::project_request::service::request(
        boundaries.context,
        &ProjectCorpus {
            cohort: corpus.cohort,
            rule_context: corpus.rule_context,
            search_roots: corpus.search_roots,
            context_mode: corpus.context_mode,
        },
        root,
        files,
        tracked,
        state,
    )
}

/// Count the findings of each rule, duplicates included.
fn finding_counts(report: &RunReport) -> FindingCounts<'_> {
    let mut counts = std::collections::BTreeMap::new();
    for diagnostic in &report.diagnostics {
        *counts.entry(&diagnostic.rule).or_insert(0_usize) += ONE_FINDING;
    }
    FindingCounts(counts)
}

/// Write one run's scalar measurements and the explicit execution order.
fn observe(
    output: &mut dyn std::io::Write,
    name: &str,
    measurement: &Measurement,
    order: RunOrder,
) -> Result<(), Error> {
    let order = match order {
        RunOrder::ReferenceFirst => REFERENCE_FIRST,
        RunOrder::CandidateFirst => CANDIDATE_FIRST,
    };
    match measurement.resources {
        ResourceObservation::Collected(resources) => writeln!(
            output,
            "sample: name={name} order={order} wall_seconds={} cpu_seconds={} rss_bytes={}",
            measurement.wall.as_secs_f64(),
            resources.cpu.as_secs_f64(),
            resources.peak_rss_bytes.0
        )?,
        ResourceObservation::WallOnly => writeln!(
            output,
            "sample: name={name} order={order} wall_seconds={} resources=absent",
            measurement.wall.as_secs_f64()
        )?,
    }
    Ok(())
}
