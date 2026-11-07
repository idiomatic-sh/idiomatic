//! The `check` service: local free-function helpers, private to this use case.

use std::collections::{BTreeMap, BTreeSet};

use super::errors::CheckError;
use super::types::Listing;
use crate::domain::applications::types::{AnswerCell, CellValue, LintAnswer, LintUnknown, Plan};
use crate::domain::findings::types::{Fingerprint, Report};
use crate::domain::governance::types::Answers;
use crate::domain::linting::service::repository_path;
use crate::domain::paths::types::RepoPath;
use crate::domain::reporting::types::Reuse;
use crate::outbound::analysis::types::AnalysisReport;
use crate::outbound::check_store::types::{Checked, CheckedFile, CheckedPath, SourcedFile};
use crate::outbound::repository::interface::Repository;

/// The manifest failure of `file`, as the use case's error.
pub(super) fn manifest_error(
    file: &RepoPath,
) -> impl Fn(crate::outbound::manifest::errors::ManifestError) -> CheckError + '_ {
    move |source| CheckError::Manifest {
        file: file.0.to_string(),
        source,
    }
}

/// The lint call's rows and unknowns, their paths relative to the repository rather than to the lint call's root.
pub(super) fn lint_answers(plan: &Plan, report: AnalysisReport) -> Answers {
    let answers = report
        .findings
        .into_iter()
        .map(|found| LintAnswer {
            path: repository_path(&plan.base, found.path),
            rule: found.rule,
            cells: found
                .cells
                .into_iter()
                .map(|cell| AnswerCell {
                    value: match cell.value {
                        CellValue::File(path) => CellValue::File(repository_path(&plan.base, path)),
                        other @ (CellValue::Span(_) | CellValue::Text(_)) => other,
                    },
                    name: cell.name,
                })
                .collect(),
        })
        .collect();
    let unknowns = report
        .unknowns
        .into_iter()
        .map(|missing| LintUnknown {
            path: repository_path(&plan.base, missing.path),
            rule: missing.rule,
            message: missing.message,
        })
        .collect();
    let unread = report
        .unread
        .into_iter()
        .map(|path| repository_path(&plan.base, path))
        .collect();
    Answers {
        answers,
        unknowns,
        unread,
    }
}

/// The directory of the package the version `release` is in: one of the workspace's, or the one a package only a
/// scenario's own `tastemaker.toml` names was found in, which its name holds.
pub(super) fn package_of(
    release: &crate::domain::packages::types::Release,
    packages: &BTreeMap<
        crate::domain::packages::types::PackageName,
        crate::domain::packages::types::ResolvedPackage,
    >,
) -> Option<crate::domain::packages::types::PackageRoot> {
    packages
        .get(&release.package)
        .map(|package| package.directory.clone())
        .or_else(|| {
            release
                .package
                .0
                .split_once('@')
                .map(|(_, directory)| crate::domain::packages::types::PackageRoot(directory.into()))
        })
}

/// The files beneath `prefixes`, their symbolic links among them, and the links alone.
pub(super) fn listing(
    repository: &dyn Repository,
    prefixes: &[RepoPath],
) -> Result<Listing, CheckError> {
    let mut files = BTreeSet::new();
    let mut links = BTreeSet::new();
    for prefix in prefixes {
        let walk = repository.walk(prefix)?;
        files.extend(walk.files);
        files.extend(walk.links.iter().cloned());
        links.extend(walk.links);
    }
    Ok(Listing {
        files: files.into_iter().collect(),
        links: links.into_iter().collect(),
    })
}

/// What a whole check keeps for the next: the files as read, and what was settled of them.
pub(super) fn checked_of(
    judged: &Report,
    current: Vec<CheckedFile>,
    known: crate::domain::digests::types::Digest,
) -> Checked {
    Checked {
        sourced: judged
            .sourced
            .iter()
            .map(|(path, incoming)| SourcedFile {
                path: CheckedPath(path.0.clone()),
                incoming: *incoming,
            })
            .collect(),
        files: current,
        findings: judged.findings.clone(),
        suppressed: judged.suppressed.clone(),
        evaluated: judged.evaluated.clone(),
        known,
    }
}

/// What the last check settled, for the files that are as they were: the same digest, settled against the same rules.
pub(super) fn reusable(
    kept: &Checked,
    current: &[CheckedFile],
    known: &crate::domain::digests::types::Digest,
) -> Reuse {
    if kept.known != *known {
        return Reuse::default();
    }
    let as_they_were: BTreeMap<RepoPath, Fingerprint> = kept
        .sourced
        .iter()
        .filter(|was| {
            let digested = |files: &[CheckedFile]| {
                files
                    .iter()
                    .find(|file| file.path == was.path)
                    .map(|file| file.digest.clone())
            };
            digested(current).is_some() && digested(current) == digested(&kept.files)
        })
        .map(|was| (RepoPath(was.path.0.clone()), was.incoming))
        .collect();
    Reuse {
        findings: kept
            .findings
            .iter()
            .filter(|finding| as_they_were.contains_key(&finding.path))
            .cloned()
            .collect(),
        suppressed: kept
            .suppressed
            .iter()
            .filter(|accepted| as_they_were.contains_key(&accepted.finding.path))
            .cloned()
            .collect(),
        incoming: as_they_were,
    }
}
