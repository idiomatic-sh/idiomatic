//! A measurement is certified only while its instrumentation, artifacts and sources stay as certified.
use super::{
    constants,
    types::{Certification, Contestants, JudgedRules, RunBaseline, SourceTexts},
};
use crate::domain::corpus_discrepancy::types::AdmittedDifferences;
use crate::domain::diagnostics::service::compare_with_contract;
use crate::domain::discrepancy::types::Contestant;
use crate::domain::measurements::types::{MeasurementEligibility, pair::value::Pair};
use crate::domain::suite::types::certificate::value::Certificate;
use crate::outbound::evidence::interface::EvidencePort;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactSet},
};
use benchmark_contracts::domain::measurements::types::{Measurement, MeasurementMode, RunOrder};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, ComparisonContract, EndPair, FindingPosition, LocatedFinding, RuleComparison,
    SourceByteOffset,
};
use benchmark_contracts::domain::requests::types::{CohortSelection, RunRequest};
use benchmark_contracts::domain::workloads::types::Project;
use std::path::{Path, PathBuf};

/// Require the certified inventory, instrumentation, artifacts, commands and executables before a run.
///
/// # Errors
/// Refuses any certified identity that differs from the one the contestants and the suite hold now.
pub fn admit(
    certificate: &Certificate,
    mode: &MeasurementMode,
    suite: &dyn SuitePort,
    execution: &dyn ExecutionPort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    request: &RunRequest,
) -> Result<RunBaseline, Error> {
    crate::domain::measurements::service::require_inventory(
        mode,
        certificate.coverage.authored(),
        match &request.selected_rules {
            CohortSelection::Explicit(rules) => Some(rules.as_slice()),
            CohortSelection::Frozen => None,
        },
    )?;
    if execution.profile() != certificate.execution {
        return Err(Error::Invalid(constants::CHANGED_INSTRUMENTATION.into()));
    }
    if suite.fingerprint(ArtifactSet(&certificate.paths))? != certificate.fingerprint {
        return Err(Error::Invalid(constants::CHANGED_EVIDENCE.into()));
    }
    if [
        reference.prepare(&certificate.probe)?,
        candidate.prepare(&certificate.probe)?,
    ] != certificate.profiles
    {
        return Err(Error::Invalid(constants::CHANGED_COMMAND.into()));
    }
    // Each run launches what its certified probe launched: a contestant's command may launch a sibling of the certified binary
    // (clippy's cargo-clippy beside the verified clippy-driver), which the probe's own command names.
    for (tool, expected) in [
        (reference, &certificate.profiles[0].executable),
        (candidate, &certificate.profiles[1].executable),
    ] {
        if tool.prepare(request)?.executable != *expected {
            return Err(Error::Invalid(constants::CHANGED_EXECUTABLE.into()));
        }
    }
    let sources: Vec<_> = request
        .files
        .iter()
        .map(|file| request.root.join(file.0.as_ref()))
        .collect();
    let before = suite.fingerprint(ArtifactSet(&sources))?;
    Ok(RunBaseline { sources, before })
}

/// Require both contestants' reports to agree on every finding and publish the comparison evidence.
///
/// # Errors
/// Refuses reports that differ under the diagnostics contract.
pub fn pair(
    evidence: &dyn EvidencePort,
    reference: Measurement,
    candidate: Measurement,
) -> Result<Pair, Error> {
    let comparison = compare_with_contract(
        &reference.report,
        &candidate.report,
        ComparisonContract::Diagnostics,
    )
    .map_err(|difference| {
        Error::Invalid(format!(
            "diagnostic benchmark contract failed: {difference:?}"
        ))
    })?;
    evidence.comparison(
        &reference.report,
        &candidate.report,
        comparison,
        ComparisonContract::Diagnostics,
    )?;
    Ok(Pair {
        eligibility: MeasurementEligibility::Diagnostic,
        reference,
        candidate,
    })
}

/// Pair both contestants' measurements of a corpus run without comparing them: the per-rule judge
/// (`corpus_discrepancy::judge`) decides whether they agree, against the corpus's accepted differences, and names each
/// difference by rule, file and line.
#[must_use]
fn unjudged(reference: Measurement, candidate: Measurement) -> Pair {
    Pair {
        eligibility: MeasurementEligibility::Diagnostic,
        reference,
        candidate,
    }
}

/// Require the instrumentation, certified artifacts and sources unchanged and issue the pair's eligibility.
///
/// # Errors
/// Refuses any identity that changed while the contestants ran.
pub fn conclude(
    certificate: &Certificate,
    mode: &MeasurementMode,
    suite: &dyn SuitePort,
    execution: &dyn ExecutionPort,
    baseline: &RunBaseline,
    mut pair: Pair,
) -> Result<Pair, Error> {
    if execution.profile() != certificate.execution {
        return Err(Error::Invalid(constants::CHANGED_INSTRUMENTATION.into()));
    }
    if suite.fingerprint(ArtifactSet(&certificate.paths))? != certificate.fingerprint {
        return Err(Error::Invalid(constants::CHANGED_DURING_RUN.into()));
    }
    if suite.fingerprint(ArtifactSet(&baseline.sources))? != baseline.before {
        return Err(Error::Invalid(constants::CHANGED_SOURCE.into()));
    }
    pair.eligibility = match mode {
        MeasurementMode::Benchmark { .. } => MeasurementEligibility::CompleteInventory,
        MeasurementMode::Diagnostic => MeasurementEligibility::Diagnostic,
    };
    Ok(pair)
}

/// Require the certified coverage to be complete before any contestant runs.
///
/// # Errors
/// Refuses a certificate whose audit leaves a default rule missing.
fn require_covered(certificate: &Certificate) -> Result<(), Error> {
    crate::domain::suite::service::require_complete(&certificate.coverage)
}

/// The file naming the project's accepted differences, `<bench>/profiles/evidence/<project>-accepted-differences.json`, if it
/// has one.
pub(crate) fn accepted_file(
    suite: &dyn SuitePort,
    bench: &Path,
    project: &Project,
) -> Result<Option<PathBuf>, Error> {
    let mut name = project.name.0.clone();
    name.push_str(constants::ACCEPTED_SUFFIX);
    let file = bench.join(constants::ACCEPTED_DIRECTORY).join(name);
    match suite.digest(ArtifactPath(&file)) {
        Ok(_digest) => Ok(Some(file)),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The differences accepted for the corpus `request` reads, which must be the corpus they were written for.
pub(crate) fn admitted(
    suite: &dyn SuitePort,
    file: Option<&Path>,
    request: &RunRequest,
) -> Result<AdmittedDifferences, Error> {
    let Some(file) = file else {
        return Ok(AdmittedDifferences::default());
    };
    let declared = suite.accepted_differences(ArtifactPath(file))?;
    let corpus = crate::domain::corpus_discrepancy::service::corpus_digest(
        &request.files,
        &request.context_manifest,
    );
    Ok(crate::domain::corpus_discrepancy::service::admit_accepted(
        &declared, &corpus,
    )?)
}

/// Judge one pair rule by rule against the accepted differences, refusing every unaccepted or stale difference.
pub(crate) fn judged(
    suite: &dyn SuitePort,
    request: &RunRequest,
    JudgedRules { selected, syntax }: JudgedRules<'_>,
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
            .map(|comparison| locate(suite, &request.root, &mut texts, comparison))
            .collect::<Result<Vec<_>, _>>()?;
    let problems = crate::domain::corpus_discrepancy::service::judge(
        &located,
        selected,
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
        "{} {}: {}",
        problems.len(),
        constants::UNACCEPTED,
        problems
            .iter()
            .map(|problem| problem.0.as_str())
            .collect::<Vec<_>>()
            .join(constants::DIFFERENCE_SEPARATOR)
    )))
}

/// The line and column of one byte offset of a corpus file, reading the file once.
fn position(
    suite: &dyn SuitePort,
    root: &Path,
    texts: &mut SourceTexts,
    file: &ryo::linter::RelativePath,
    offset: SourceByteOffset,
) -> Result<FindingPosition, Error> {
    let source = match texts.0.entry(file.clone()) {
        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(suite.source_text(ArtifactPath(&root.join(file.0.as_ref())))?)
        }
    };
    crate::domain::diagnostics::service::position(source, offset)
}

/// Every finding of one rule's comparison placed by line and column.
fn locate(
    suite: &dyn SuitePort,
    root: &Path,
    texts: &mut SourceTexts,
    comparison: &RuleComparison<BenchmarkFinding>,
) -> Result<RuleComparison<LocatedFinding>, Error> {
    let mut place = |finding: &BenchmarkFinding| -> Result<LocatedFinding, Error> {
        Ok(LocatedFinding {
            file: finding.file.clone(),
            start: position(suite, root, texts, &finding.file, finding.start)?,
            end: position(suite, root, texts, &finding.file, finding.end)?,
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

/// The request with its state in the contestant's own directory below the request's, so each store is its own.
pub(crate) fn own(request: &RunRequest, contestant: Contestant) -> RunRequest {
    RunRequest {
        state: request.state.join(match contestant {
            Contestant::Reference => constants::REFERENCE_STATE,
            Contestant::Candidate => constants::CANDIDATE_STATE,
        }),
        ..request.clone()
    }
}

/// Both contestants measured under the certificate through `measure`, in the requested order, each in its own state.
///
/// # Errors
/// Refuses a run the certificate does not admit, a contestant run that fails, and a pair it does not conclude.
pub(crate) fn measured(
    contestants: Contestants<'_>,
    certified: Certification<'_>,
    request: &RunRequest,
    order: RunOrder,
    measure: &dyn Fn(&dyn LinterPort, &RunRequest) -> Result<Measurement, Error>,
) -> Result<Pair, Error> {
    let baseline = admit(
        certified.certificate,
        certified.mode,
        contestants.suite,
        contestants.execution,
        contestants.reference,
        contestants.candidate,
        request,
    )?;
    require_covered(certified.certificate)?;
    let run = |linter: &dyn LinterPort, contestant| measure(linter, &own(request, contestant));
    let (reference, candidate) = match order {
        RunOrder::ReferenceFirst => (
            run(contestants.reference, Contestant::Reference)?,
            run(contestants.candidate, Contestant::Candidate)?,
        ),
        RunOrder::CandidateFirst => {
            let candidate = run(contestants.candidate, Contestant::Candidate)?;
            (
                run(contestants.reference, Contestant::Reference)?,
                candidate,
            )
        }
    };
    conclude(
        certified.certificate,
        certified.mode,
        contestants.suite,
        contestants.execution,
        &baseline,
        unjudged(reference, candidate),
    )
}
