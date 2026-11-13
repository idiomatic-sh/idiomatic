//! Both contestants run once through the shared measurement; each configured rule is compared and judged.
use super::constants::{
    PROJECT_FAILED, PROJECT_SEPARATOR, PROJECTS_FAILED, SELECTION_DIFFERS, UNACCEPTED,
    UNTRACKED_SOURCE,
};
use super::types::{
    AcceptedDifferencesFile, Corpus, CorpusParity, ParityBoundaries, ProjectsParity, SourceTexts,
};
use crate::application::measurement::{service::run_with, types::Pinning};
use crate::domain::corpus_discrepancy::types::ObservedDifferences;
use crate::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::reports::constants::CANDIDATE_NAME;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, ContestantName, DifferenceSide, EndPair, FindingCount, FindingPosition,
    LocatedDifference, LocatedFinding, ParityReport, RuleComparison, SourceByteOffset,
};
use ryo_benchmark_contracts::domain::requests::types::CohortSelection;

use std::collections::BTreeSet;

/// Compare one built request, or every project of a language's benchmark, each published under its own name; a
/// project that fails is reported and the others still run.
pub(super) fn compare_corpus(
    boundaries: &ParityBoundaries<'_>,
    parity: &CorpusParity<'_>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let projects = match parity.corpus {
        Corpus::Request { request, accepted } => {
            return compare_request(boundaries, parity, request, accepted, parity.reference, observations);
        }
        Corpus::Projects(projects) => projects,
    };
    let mut failures = Vec::new();
    for project in projects.projects {
        let name = [project.name.0.as_str(), parity.reference.0].join(PROJECT_SEPARATOR);
        let result = compare_project(boundaries, parity, &projects, project, ContestantName(&name), observations);
        if let Err(error) = result {
            writeln!(observations, "{}: {PROJECT_FAILED}: {error}", project.name)?;
            failures.push(project.name.0.clone());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{PROJECTS_FAILED}: {failures:?}")))
    }
}

/// Check out one project, take the sources the reference discovers with the language's suffixes, build its request and
/// compare it, judged by the project's accepted-differences file when it has one.
fn compare_project(
    boundaries: &ParityBoundaries<'_>,
    parity: &CorpusParity<'_>,
    projects: &ProjectsParity<'_>,
    project: &ryo_benchmark_contracts::domain::workloads::types::Project,
    name: ContestantName<'_>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let workspace = boundaries.workspace.prepare(project)?;
    let files = boundaries
        .discovery
        .files(workspace.root())?
        .into_iter()
        .filter(|file| projects.suffixes.iter().any(|suffix| file.0.ends_with(suffix.0)))
        .collect::<Vec<_>>();
    let tracked = boundaries.workspace.tracked_files(&workspace)?.0;
    if files.iter().any(|file| !tracked.iter().any(|path| path.as_str() == file.0.as_ref())) {
        return Err(Error::Invalid(UNTRACKED_SOURCE.into()));
    }
    let request = crate::application::project_request::service::request(
        boundaries.context,
        &crate::application::project_request::types::ProjectCorpus {
            cohort: projects.cohort,
            rule_context: projects.rule_context,
            search_roots: projects.search_roots,
            context_mode: projects.context_mode,
        },
        workspace.root(),
        &files,
        &tracked.iter().map(std::path::PathBuf::from).collect::<Vec<_>>(),
        projects.state.join(&project.name.0),
    )?;
    let mut file_name = project.name.0.clone();
    file_name.push_str(projects.accepted_suffix.0);
    let accepted_file = projects.accepted.join(file_name);
    let accepted = match boundaries.suite.digest(ArtifactPath(&accepted_file)) {
        Ok(_digest) => AcceptedDifferencesFile::Declared(&accepted_file),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => AcceptedDifferencesFile::Absent,
        Err(error) => return Err(error),
    };
    compare_request(boundaries, parity, &request, accepted, name, observations)
}

/// Refuse the wrong rule set or corpus, run both contestants, compare and judge each rule, and publish.
fn compare_request(
    boundaries: &ParityBoundaries<'_>,
    parity: &CorpusParity<'_>,
    request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    accepted_file: AcceptedDifferencesFile<'_>,
    reference_name: ContestantName<'_>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let selected = select(request, parity.configured)?;
    let corpus = crate::domain::corpus_discrepancy::service::corpus_digest(
        &request.files,
        &request.context_manifest,
    );
    let accepted = match accepted_file {
        AcceptedDifferencesFile::Declared(file) => {
            crate::domain::corpus_discrepancy::service::admit_accepted(
                &boundaries.suite.accepted_differences(ArtifactPath(file))?,
                &corpus,
            )?
        }
        AcceptedDifferencesFile::Absent => Default::default(),
    };
    // Findings do not depend on thread count, so both contestants run at default threads.
    let reference = run_with(
        boundaries.context,
        boundaries.reference,
        boundaries.execution,
        request,
        Pinning::Untimed,
    )?;
    let candidate = run_with(
        boundaries.context,
        boundaries.candidate,
        boundaries.execution,
        request,
        Pinning::Untimed,
    )?;
    let mut texts = SourceTexts::default();
    let mut rules = Vec::new();
    let mut observed = Vec::new();
    let mut outside = Vec::new();
    for comparison in crate::domain::diagnostics::service::compare_rules(
        selected,
        &reference.report,
        &candidate.report,
    ) {
        let located = locate(boundaries, &request.root, &mut texts, &comparison)?;
        if rules.len() == selected.len() {
            outside.push(located);
            continue;
        }
        observed.push(ObservedDifferences {
            rule: located.rule.clone(),
            differences: differences(DifferenceSide::Missing, &located.missing)
                .chain(differences(DifferenceSide::Extra, &located.extra))
                .collect(),
            end_differences: FindingCount(located.end_differences.len()),
        });
        rules.push(located);
    }
    let mut problems =
        crate::domain::corpus_discrepancy::service::corpus_problems(&observed, &accepted);
    problems.extend(
        crate::domain::corpus_discrepancy::service::outside_problems(&outside, parity.syntax),
    );
    problems.extend(crate::domain::corpus_discrepancy::service::skip_problems(
        &reference.report,
        &candidate.report,
        parity.syntax,
    ));
    let identical = rules
        .iter()
        .filter(|rule| rule.missing.is_empty() && rule.extra.is_empty())
        .count();
    writeln!(
        observations,
        "{} parity: {identical} identical of {} rules; {} unaccepted",
        reference_name.0,
        rules.len(),
        problems.len()
    )?;
    for problem in &problems {
        writeln!(observations, "{}", problem.0)?;
    }
    let unaccepted = problems.len();
    boundaries.evidence.parity(&ParityReport {
        reference: reference_name,
        candidate: ContestantName(CANDIDATE_NAME),
        rules,
        problems,
    })?;
    if unaccepted == 0 {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{unaccepted} {UNACCEPTED}")))
    }
}

/// The request's selected rules, when they are exactly the configured rules, each once.
fn select<'a>(
    request: &'a ryo_benchmark_contracts::domain::requests::types::RunRequest,
    configured: &[ryo::linter::RuleId],
) -> Result<&'a [ryo::linter::RuleId], Error> {
    let selected = match &request.selected_rules {
        CohortSelection::Explicit(rules) => rules.as_slice(),
        CohortSelection::Frozen => &[],
    };
    let distinct: BTreeSet<_> = selected.iter().collect();
    let configured: BTreeSet<_> = configured.iter().collect();
    if distinct.len() == selected.len() && distinct == configured {
        Ok(selected)
    } else {
        Err(Error::Invalid(format!(
            "{SELECTION_DIFFERS}: {} configured, {} selected; unselected {:?}, unconfigured {:?}",
            configured.len(),
            selected.len(),
            configured.difference(&distinct).collect::<Vec<_>>(),
            distinct.difference(&configured).collect::<Vec<_>>(),
        )))
    }
}

/// The unmatched findings of one side, as the accepted differences pin them.
fn differences(
    side: DifferenceSide,
    findings: &[LocatedFinding],
) -> impl Iterator<Item = LocatedDifference> + '_ {
    findings.iter().map(move |finding| LocatedDifference {
        side,
        file: finding.file.clone(),
        start: finding.start,
    })
}

/// The line and column of one byte offset of a corpus file, reading the file once.
fn position(
    boundaries: &ParityBoundaries<'_>,
    root: &std::path::Path,
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
    boundaries: &ParityBoundaries<'_>,
    root: &std::path::Path,
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
