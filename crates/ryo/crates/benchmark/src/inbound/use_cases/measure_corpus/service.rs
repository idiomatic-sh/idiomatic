//! Every project is measured in both orders and compared cold, retained and edited.
use super::constants::{
    CANDIDATE_EDITED, CANDIDATE_FIRST, CANDIDATE_NAME, CANDIDATE_RETAINED, CLEAN_EDITED_DIRECTORY,
    EDITED_CACHE_CONDITION, EMPTY_CACHE_CONDITION, FINDING_SEPARATOR, ONE_FINDING, PATH_SEPARATOR,
    PREFLIGHT_DIRECTORY, REFERENCE_EDITED, REFERENCE_FIRST, REFERENCE_RETAINED,
    RETAINED_CACHE_CONDITION, RETAINED_DIRECTORY, TRIAL_PREFIX, TRIALS,
};
use super::types::{
    ComparisonName, CorpusBoundaries, CorpusMeasurement, FindingCounts, SourceProfile,
};
use crate::application::certified_measurement::service::{admit, conclude, pair, require_covered};
use crate::application::measurement::service::run as execute;
use crate::domain::measurements::types::{pair::value::Pair, sample::value::BenchmarkSample};
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet, ArtifactText};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::{
    Measurement, ResourceObservation, RunOrder, SampleName,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::{
    BenchmarkCoverage, ContextSeal, FileMetadataDeclaration, InputDeclaration, RunRequest,
    StagedSources,
};
use ryo_benchmark_contracts::domain::workloads::types::{ConfigurationOverride, Project};
use std::path::{Path, PathBuf};

/// Measure every project of the corpus, then publish the samples of an accounted run or say that a diagnostic one is not a metric.
pub(super) fn measure_corpus(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut samples = Vec::new();
    let mut failures = Vec::new();
    for project in &corpus.projects {
        match measure_project(boundaries, corpus, project, observations) {
            Ok(observed) => samples.extend(observed),
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
    // The evidence boundary refuses an empty observation set; never bypass that guard.
    if corpus.scope == BenchmarkCoverage::Accounted {
        crate::application::publication::service::publish(boundaries.evidence, &samples)?;
    } else {
        writeln!(
            observations,
            "diagnostic-only: scoped samples are not benchmark metrics"
        )?;
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "corpus comparisons failed: {failures:?}"
        )))
    }
}

/// Measure one project in both orders, cold, retained and edited, and write what was observed.
fn measure_project(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    project: &Project,
    observations: &mut dyn std::io::Write,
) -> Result<Vec<BenchmarkSample>, Error> {
    let language = &corpus.language;
    let mut project_samples = Vec::new();
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
    if corpus.profile == SourceProfile::Valid {
        let preflight = request(
            boundaries,
            corpus,
            workspace.root(),
            &files,
            &tracked_paths,
            corpus.cache.join(PREFLIGHT_DIRECTORY).join(&project.name.0),
        )?;
        let parsed = run_linter(boundaries, boundaries.reference, &preflight)?;
        let rejected = || {
            parsed
                .report
                .diagnostics
                .iter()
                .filter(|finding| finding.rule == *language.invalid_syntax)
        };
        let invalid: std::collections::BTreeSet<_> =
            rejected().map(|finding| finding.file.clone()).collect();
        for finding in rejected() {
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
        files.retain(|file| !invalid.contains(file));
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
        if corpus.profile == SourceProfile::Valid {
            language.valid_profile.0
        } else {
            language.full_profile.0
        }
    )?;
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
        let cold = measured(boundaries, corpus, &initial, order)?;
        writeln!(
            observations,
            "{}: exact matching findings per rule: {}",
            project.name,
            finding_counts(&cold.reference().report)
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
        boundaries
            .workspace
            .apply_patch(&mut workspace, &corpus.frozen_workload.join(&project.patch))?;
        let edited_request = RunRequest {
            context_manifest: seal_context(
                boundaries,
                corpus.context_mode,
                workspace.root(),
                &files,
                &tracked_paths,
            )?,
            ..initial.clone()
        };
        let clean_request = RunRequest {
            state: trial.join(CLEAN_EDITED_DIRECTORY),
            ..edited_request.clone()
        };
        let edited = measured(boundaries, corpus, &edited_request, order);
        let clean_reference = run_linter(boundaries, boundaries.reference, &clean_request);
        let clean_candidate = run_linter(boundaries, boundaries.candidate, &clean_request);
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
        for (condition, measured_pair) in [
            (EMPTY_CACHE_CONDITION, cold),
            (RETAINED_CACHE_CONDITION, retained),
            (EDITED_CACHE_CONDITION, edited),
        ] {
            let contestants = [language.reference_name.0, CANDIDATE_NAME];
            let named = measured_pair.samples(contestants.map(|contestant| {
                SampleName(format!("{}/{contestant}/{condition}", project.name))
            }));
            for (contestant, sample) in contestants.into_iter().zip(named) {
                writeln!(
                    observations,
                    "findings: project={} contestant={contestant} condition={condition} counts={}",
                    project.name,
                    finding_counts(&sample.measurement().report)
                )?;
                observe_sample(observations, &sample, order)?;
                project_samples.push(sample);
            }
        }
    }
    Ok(project_samples)
}

/// Require two reports to agree in full, naming what was compared when they differ.
fn agree(what: ComparisonName, first: &RunReport, second: &RunReport) -> Result<(), Error> {
    crate::domain::diagnostics::service::compare(first, second)
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

/// Both contestants measured under the certificate, in the requested order.
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
    let (reference, candidate) = match order {
        RunOrder::ReferenceFirst => (
            run_linter(boundaries, boundaries.reference, request)?,
            run_linter(boundaries, boundaries.candidate, request)?,
        ),
        RunOrder::CandidateFirst => {
            let candidate = run_linter(boundaries, boundaries.candidate, request)?;
            (
                run_linter(boundaries, boundaries.reference, request)?,
                candidate,
            )
        }
    };
    let measured_pair = pair(boundaries.evidence, reference, candidate)?;
    conclude(
        corpus.certificate,
        corpus.mode,
        boundaries.suite,
        boundaries.execution,
        &baseline,
        measured_pair,
    )
}

/// One run of the corpus project over its explicit files, with its own state directory.
fn request(
    boundaries: &CorpusBoundaries<'_>,
    corpus: &CorpusMeasurement<'_>,
    root: &Path,
    files: &[ryo::linter::RelativePath],
    tracked: &[PathBuf],
    state: PathBuf,
) -> Result<RunRequest, Error> {
    Ok(RunRequest {
        performance: Default::default(),
        selected_rules: corpus.cohort.clone(),
        rule_observations: InputDeclaration::Undeclared,
        rule_context: crate::domain::rule_context::service::resolve(
            corpus.rule_context,
            &ConfigurationOverride::Inherited,
        )?,
        project_search_roots: corpus.search_roots.clone(),
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: seal_context(boundaries, corpus.context_mode, root, files, tracked)?,
        root: root.to_owned(),
        files: files.to_vec(),
        state,
    })
}

/// Select the declared generic corpus mode and delegate all observation to the context boundary.
fn seal_context(
    boundaries: &CorpusBoundaries<'_>,
    mode: ProjectContextMode,
    root: &Path,
    files: &[ryo::linter::RelativePath],
    tracked: &[PathBuf],
) -> Result<ContextSeal, Error> {
    match mode {
        ProjectContextMode::Isolated => Ok(ContextSeal::Unsealed),
        ProjectContextMode::ListedDisk => boundaries
            .context
            .seal_existing(
                root,
                files,
                crate::outbound::context::types::CorpusFiles(tracked),
            )
            .map(ContextSeal::Sealed),
    }
}

/// Count the findings of each rule, duplicates included.
fn finding_counts(report: &RunReport) -> FindingCounts<'_> {
    let mut counts = std::collections::BTreeMap::new();
    for diagnostic in &report.diagnostics {
        *counts.entry(&diagnostic.rule).or_insert(0_usize) += ONE_FINDING;
    }
    FindingCounts(counts)
}

/// Write scalar child measurements and the explicit execution order.
fn observe_sample(
    output: &mut dyn std::io::Write,
    sample: &BenchmarkSample,
    order: RunOrder,
) -> Result<(), Error> {
    let order = match order {
        RunOrder::ReferenceFirst => REFERENCE_FIRST,
        RunOrder::CandidateFirst => CANDIDATE_FIRST,
    };
    match sample.measurement().resources {
        ResourceObservation::Collected(resources) => writeln!(
            output,
            "sample: name={} order={order} wall_seconds={} cpu_seconds={} rss_bytes={}",
            sample.name().0,
            sample.measurement().wall.as_secs_f64(),
            resources.cpu.as_secs_f64(),
            resources.peak_rss_bytes.0
        )?,
        ResourceObservation::WallOnly => writeln!(
            output,
            "sample: name={} order={order} wall_seconds={} resources=absent",
            sample.name().0,
            sample.measurement().wall.as_secs_f64()
        )?,
    }
    Ok(())
}
