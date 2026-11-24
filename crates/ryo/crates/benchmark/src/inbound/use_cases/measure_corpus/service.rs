//! Every project is measured in both orders, cold, retained and edited; each pair is judged rule by rule against the
//! reference, and the candidate's costs, the verification times and the growth are the bench's one document.
use super::constants::{
    CANDIDATE_EDITED, CANDIDATE_RETAINED, CLEAN_EDITED_DIRECTORY, COHORT_OUTSIDE_CERTIFICATE,
    EDITED_CACHE_CONDITION, EMPTY_CACHE_CONDITION, GROWTH_INCOMPLETE, NO_SOURCE, NOT_COMPLETE,
    PATH_SEPARATOR, REFERENCE_EDITED, REFERENCE_RETAINED, RETAINED_CACHE_CONDITION,
    RETAINED_DIRECTORY, SCENES_DIRECTORY, TRIAL_PREFIX, TRIALS,
};
use super::types::{ComparisonName, CorpusBoundaries, CorpusMeasurement};
use crate::application::certified_measurement::service::{
    accepted_file, admitted, judged, measured, own,
};
use crate::application::certified_measurement::types::{Certification, JudgedRules};
use crate::application::measurement::service::{
    cost as measurement_cost, finding_counts, observe, run as execute,
};
use crate::application::project_request::service::request;
use crate::application::project_request::types::ProjectCorpus;
use crate::domain::discrepancy::types::Contestant;
use crate::domain::measurements::types::MeasurementEligibility;
use crate::domain::source_validity::constants::PREFLIGHT_DIRECTORY;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet, ArtifactText};
use ryo_benchmark_contracts::domain::costs::types::{ProjectCost, ProjectCosts, StoreSize};
use ryo_benchmark_contracts::domain::measurements::types::{Measurement, RunOrder};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, RunRequest, WorkerRequest,
};
use ryo_benchmark_contracts::domain::workloads::types::Project;
use std::path::PathBuf;
/// Refuse a cohort outside the certificate, measure every project of the corpus, then publish the samples of an accounted run or say that a diagnostic one is not a metric.
pub(super) fn measure_corpus(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let contestants = boundaries.contestants;
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
        contestants.suite,
        boundaries.host,
        corpus.bench,
        &selected,
    )?;
    let profile = crate::application::corpus_profile::active(corpus.cache)?;
    let mut costs = Vec::new();
    let mut failures = Vec::new();
    for (index, project) in corpus.projects.iter().enumerate() {
        let profiling = profile.as_ref().map(|profile| (profile, index));
        match measure_project(
            boundaries,
            corpus,
            project,
            &selected,
            profiling,
            observations,
        ) {
            Ok(Some(cost)) => costs.push(cost),
            Ok(None) => {}
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
    if let Some(profile) = profile {
        return profile.finish();
    }
    let evidence = boundaries.evidence;
    evidence.costs(corpus.language.id, &ProjectCosts(costs))?;
    let timings = evidence.read_verification(
        &crate::application::inventory_verification::service::times_files(
            corpus.verification.directory,
            corpus.verification.rules,
        ),
    )?;
    if !timings.0.is_empty() {
        evidence.verification(corpus.language.id, &timings)?;
    }
    if let crate::domain::growth::types::CorpusGrowth::Grown(source) = corpus.growth {
        let scenes = contestants
            .suite
            .ensure_directory(ArtifactPath(&corpus.cache.join(SCENES_DIRECTORY)))?;
        let plan = crate::application::growth::service::plan(contestants.suite, source)?;
        let lint = |request: &WorkerRequest| lint(boundaries.engine, request);
        let grown = crate::application::growth::service::grow(
            &lint,
            contestants.suite,
            boundaries.meter,
            ArtifactPath(&scenes),
            &plan,
        )?;
        evidence.growth(corpus.language.id, &grown)?;
    }
    evidence.finish()
}

/// One growth lint, as the worker runs a request: prepare, load, configure and execute; refused unless it completes.
fn lint(
    engine: &dyn crate::outbound::engine::interface::EnginePort,
    request: &WorkerRequest,
) -> Result<(), Error> {
    let prepared = crate::application::worker_request::service::prepare(request.clone())?;
    let session = engine.load(&prepared.request)?;
    let configured =
        crate::application::worker_request::service::configure(&prepared.request, session)?;
    if configured.execute()?.report.execution_metadata
        == ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete
    {
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
    profiling: Option<(&crate::application::corpus_profile::CorpusProfile, usize)>,
    observations: &mut dyn std::io::Write,
) -> Result<Option<ProjectCost>, Error> {
    let language = &corpus.language;
    let contestants = boundaries.contestants;
    let certified = Certification {
        certificate: corpus.certificate,
        mode: corpus.mode,
    };
    let rules = JudgedRules {
        selected,
        syntax: language.invalid_syntax,
    };
    let project_corpus = ProjectCorpus {
        cohort: corpus.cohort,
        rule_context: corpus.rule_context,
        search_roots: corpus.search_roots,
        context_mode: corpus.context_mode,
    };
    let measure = |linter: &dyn crate::outbound::linter::interface::LinterPort,
                   request: &RunRequest| {
        execute(contestants.context, linter, contestants.execution, request)
    };
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
    // Nothing left to lint would pass every comparison: refused before the reference runs, and after it refuses every file.
    if files.is_empty() {
        return Err(Error::Invalid(NO_SOURCE.into()));
    }
    let preflight = request(
        contestants.context,
        &project_corpus,
        workspace.root(),
        &files,
        &tracked_paths,
        corpus.cache.join(PREFLIGHT_DIRECTORY).join(&project.name.0),
    )?;
    let parsed = measure(contestants.reference, &preflight)?;
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
    if files.is_empty() {
        return Err(Error::Invalid(NO_SOURCE.into()));
    }
    let paths: Vec<_> = files
        .iter()
        .map(|file| workspace.root().join(file.0.as_ref()))
        .collect();
    let bytes = contestants.suite.total_size(ArtifactSet(&paths))?.0;
    writeln!(
        observations,
        "{}: measured source files={} bytes={bytes} digest={} input-profile={}",
        project.name,
        files.len(),
        contestants.suite.fingerprint(ArtifactSet(&paths))?,
        language.valid_profile.0
    )?;
    if let Some((profile, index)) = profiling {
        profile.project(
            index,
            &project.name.0,
            contestants.candidate,
            |state, trace| {
                use ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On;
                let mut candidate = request(
                    contestants.context,
                    &project_corpus,
                    workspace.root(),
                    &files,
                    &tracked_paths,
                    state.join("state"),
                )?;
                candidate.performance.diagnostics.execution = On;
                if trace.is_some() {
                    candidate.performance.diagnostics.steps = On;
                }
                let mut invocation = contestants.candidate.prepare(&candidate)?;
                if let Some(trace) = trace {
                    use ryo_benchmark_contracts::domain::requests::constants::DIAGNOSTIC_RYO;
                    let command = invocation.arguments.first_mut().ok_or_else(|| {
                        Error::Invalid("candidate has no diagnostic command".into())
                    })?;
                    *command = DIAGNOSTIC_RYO.into();
                    invocation.arguments.push(trace.as_os_str().to_owned());
                }
                Ok((invocation, candidate))
            },
        )?;
        return Ok(None);
    }
    let accepted_file = accepted_file(contestants.suite, corpus.bench, project)?;
    let mut runs: [Vec<Measurement>; 3] = Default::default();
    let mut store = None;
    for order in [RunOrder::ReferenceFirst, RunOrder::CandidateFirst]
        .into_iter()
        .cycle()
        .take(TRIALS)
    {
        // A new owned directory makes both retained and clean baselines fresh,
        // even if an earlier trial left caches behind in the caller's root.
        let trial = contestants
            .suite
            .retained_directory(ArtifactPath(corpus.cache), ArtifactText(TRIAL_PREFIX))?
            .0;
        let initial = request(
            contestants.context,
            &project_corpus,
            workspace.root(),
            &files,
            &tracked_paths,
            trial.join(RETAINED_DIRECTORY),
        )?;
        let accepted = admitted(contestants.suite, accepted_file.as_deref(), &initial)?;
        let cold = measured(contestants, certified, &initial, order, &measure)?;
        if store.is_none() {
            store = Some(
                contestants
                    .suite
                    .directory_size(ArtifactPath(&own(&initial, Contestant::Candidate).state))?,
            );
        }
        writeln!(
            observations,
            "{}: exact matching findings per rule: {}",
            project.name,
            finding_counts(&cold.reference().report)
        )?;
        judged(
            contestants.suite,
            &initial,
            rules,
            &accepted,
            &cold,
            observations,
        )?;
        let retained = measured(contestants, certified, &initial, order, &measure)?;
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
            contestants.suite,
            &initial,
            rules,
            &accepted,
            &retained,
            observations,
        )?;
        boundaries
            .workspace
            .apply_patch(&mut workspace, &corpus.frozen_workload.join(&project.patch))?;
        let edited_request = request(
            contestants.context,
            &project_corpus,
            workspace.root(),
            &files,
            &tracked_paths,
            initial.state.clone(),
        )?;
        let clean_request = RunRequest {
            state: trial.join(CLEAN_EDITED_DIRECTORY),
            ..edited_request.clone()
        };
        let edited = measured(contestants, certified, &edited_request, order, &measure);
        let clean_reference = measure(
            contestants.reference,
            &own(&clean_request, Contestant::Reference),
        );
        let clean_candidate = measure(
            contestants.candidate,
            &own(&clean_request, Contestant::Candidate),
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
            contestants.suite,
            &edited_request,
            rules,
            // The edit changes one file of the corpus the differences were accepted for: they hold for every file it leaves.
            &accepted,
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
                    &ryo_benchmark_contracts::domain::measurements::types::SampleName(format!(
                        "{}/{contestant}/{condition}",
                        project.name
                    )),
                    measurement,
                    order,
                )?;
            }
            condition_runs.push(measured_pair.candidate().clone());
        }
    }
    let [cold, warm, edited] = runs;
    Ok(Some(ProjectCost {
        project: project.name.clone(),
        cold: measurement_cost(&cold)?,
        warm: measurement_cost(&warm)?,
        edited: measurement_cost(&edited)?,
        store: store.unwrap_or(StoreSize::default()),
    }))
}

/// Require two reports to agree in full, naming what was compared when they differ.
fn agree(what: ComparisonName, first: &RunReport, second: &RunReport) -> Result<(), Error> {
    crate::domain::diagnostics::service::compare_conditions(first, second)
        .map_err(|difference| Error::Invalid(format!("{} mismatch: {difference:?}", what.0)))
}
