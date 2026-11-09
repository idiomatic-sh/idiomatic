//! The digest a corpus is identified by, and the judgement of a corpus comparison's differences against
//! the differences accepted for that corpus.
use super::constants;
use super::errors::CorpusDiscrepancyError;
use super::types::{AdmittedDifferences, ObservedDifferences, ProblemVerdict};
use crate::domain::canonical_text::{
    service::digest,
    types::{Fragment, Label},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::types::ParityProblem;
use std::collections::BTreeMap;

/// The digest of the corpus a request reads: its file list and, when sealed, its context's logical root,
/// source, auxiliary and missing entries; never its location, rules or state.
#[must_use]
pub(crate) fn corpus_digest(
    files: &[ryo::linter::RelativePath],
    context: &ryo_benchmark_contracts::domain::requests::types::ContextSeal,
) -> ArtifactDigest {
    use ryo_benchmark_contracts::domain::requests::types::ContextSeal;
    let path = |path: &std::path::Path| Fragment::Text(Label(path.to_string_lossy().into_owned()));
    let entries = |entries: &BTreeMap<std::path::PathBuf, ArtifactDigest>| {
        Fragment::Object(
            entries
                .iter()
                .map(|(file, digest)| {
                    (
                        Label(file.to_string_lossy().into_owned()),
                        Fragment::Text(Label(digest.0.clone())),
                    )
                })
                .collect(),
        )
    };
    let mut corpus = BTreeMap::from([(
        Label::from(crate::domain::discrepancy::constants::FILES),
        Fragment::List(
            files
                .iter()
                .map(|file| Fragment::Text(Label::from(file.0.as_ref())))
                .collect(),
        ),
    )]);
    if let ContextSeal::Sealed(manifest) = context {
        corpus.extend([
            (
                Label::from(constants::LOGICAL_ROOT),
                path(&manifest.logical_root),
            ),
            (
                Label::from(constants::SOURCE_FILES),
                entries(&manifest.source_files),
            ),
            (
                Label::from(constants::AUXILIARY_FILES),
                entries(&manifest.auxiliary_files),
            ),
            (
                Label::from(constants::MISSING),
                Fragment::List(
                    manifest
                        .missing
                        .iter()
                        .map(|missing| path(missing))
                        .collect(),
                ),
            ),
        ]);
    }
    digest(&Fragment::Object(corpus))
}

/// Admit accepted differences for the corpus they were written for.
///
/// # Errors
/// Refuses another corpus, a rule written twice, and a rule without a reason or with no or repeated findings.
pub(crate) fn admit_accepted(
    accepted: &ryo_benchmark_contracts::domain::reports::types::AcceptedDifferences,
    corpus: &ArtifactDigest,
) -> Result<AdmittedDifferences, CorpusDiscrepancyError> {
    if &accepted.corpus != corpus {
        return Err(CorpusDiscrepancyError::CorpusDiffers {
            accepted: accepted.corpus.0.clone(),
            actual: corpus.0.clone(),
        });
    }
    let mut admitted = AdmittedDifferences::default();
    for entry in &accepted.rules {
        if admitted.0.contains_key(&entry.rule) {
            return Err(CorpusDiscrepancyError::RuleNamedTwice(
                entry.rule.to_string(),
            ));
        }
        let findings: std::collections::BTreeSet<_> = entry.findings.iter().cloned().collect();
        if entry.reason.0.trim().is_empty()
            || findings.is_empty()
            || findings.len() != entry.findings.len()
        {
            return Err(CorpusDiscrepancyError::AcceptanceIncomplete(
                entry.rule.to_string(),
            ));
        }
        let _absent = admitted.0.insert(entry.rule.clone(), findings);
    }
    Ok(admitted)
}

/// The problem one unmatched finding of a rule raises: `<rule>: [accepted ]<side> <file>:<line>:<column> ...`.
fn problem(
    rule: &ryo::linter::RuleId,
    difference: &ryo_benchmark_contracts::domain::reports::types::LocatedDifference,
    verdict: ProblemVerdict,
) -> ParityProblem {
    use ryo_benchmark_contracts::domain::reports::types::DifferenceSide;
    let side = match difference.side {
        DifferenceSide::Extra => constants::EXTRA,
        DifferenceSide::Missing => constants::MISSING,
    };
    let (file, line, column) = (
        &difference.file.0,
        difference.start.line.0,
        difference.start.column.0,
    );
    ParityProblem(match verdict {
        ProblemVerdict::NotAccepted => {
            format!("{rule}: {side} {file}:{line}:{column} not accepted")
        }
        ProblemVerdict::NoLongerOccurs => {
            format!("{rule}: accepted {side} {file}:{line}:{column} no longer occurs")
        }
    })
}

/// Every unmatched finding the accepted differences do not name, every accepted finding that no longer
/// occurs, every end difference, and every accepted rule that is not selected; rules in selection order.
#[must_use]
pub(crate) fn corpus_problems(
    observed: &[ObservedDifferences],
    accepted: &AdmittedDifferences,
) -> Vec<ParityProblem> {
    let none = std::collections::BTreeSet::new();
    let mut problems = Vec::new();
    for rule in observed {
        let expected = accepted.0.get(&rule.rule).unwrap_or(&none);
        let mut actual = rule.differences.clone();
        actual.sort();
        let mut seen = std::collections::BTreeSet::new();
        for difference in &actual {
            // An accepted finding admits one occurrence; a repeat is not accepted.
            if !expected.contains(difference) || !seen.insert(difference) {
                problems.push(problem(&rule.rule, difference, ProblemVerdict::NotAccepted));
            }
        }
        problems.extend(
            expected
                .iter()
                .filter(|difference| actual.binary_search(difference).is_err())
                .map(|difference| problem(&rule.rule, difference, ProblemVerdict::NoLongerOccurs)),
        );
        if rule.end_differences.0 != 0 {
            problems.push(ParityProblem(format!(
                "{}: {} end positions differ",
                rule.rule, rule.end_differences.0
            )));
        }
    }
    let selected: std::collections::BTreeSet<_> = observed.iter().map(|rule| &rule.rule).collect();
    problems.extend(
        accepted
            .0
            .keys()
            .filter(|rule| !selected.contains(rule))
            .map(|rule| ParityProblem(format!("{rule}: accepted but not selected"))),
    );
    problems
}
