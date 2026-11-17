//! The `linting` service: free functions named by verb over what a check hands the lint call and reads back.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{ListingText, Partitioned, RefusalReason};
use crate::domain::applications::types::{LintRuleId, Plan, Purpose};
use crate::domain::definitions::constants::GIT_EXCLUDE;
use crate::domain::definitions::types::{LocalPackage, PackagePath, QueryText, WorkspaceSection};
use crate::domain::files::types::MachinePath;
use crate::domain::findings::constants::QUERY;
use crate::domain::findings::service::{check_id, error};
use crate::domain::findings::types::Finding;
use crate::domain::governance::types::Governance;
use crate::domain::packages::types::{Lock, LockedPackage, PackageName};
use crate::domain::paths::service::{containment, join, relative};
use crate::domain::paths::types::{Containment, PathText, RepoPath};
use crate::domain::selectors::types::{Resolved, ResolvedSelectors};

/// Where the lint call is rooted: the plan's base, in the repository.
#[must_use]
pub fn lint_root(repository: &MachinePath, base: &RepoPath) -> MachinePath {
    MachinePath(
        std::path::Path::new(&*repository.0)
            .join(&*base.0)
            .display()
            .to_string()
            .into(),
    )
}

/// Splits the workspace's `files` between the packages of `package_dirs` that lie among them and the files it governs.
#[must_use]
pub fn partitioned(files: &[RepoPath], package_dirs: &[PackagePath]) -> Partitioned {
    let local_packages = package_dirs
        .iter()
        .filter(|found| {
            files
                .iter()
                .any(|file| containment(file, &found.path) == Containment::Inside)
        })
        .map(|found| LocalPackage {
            package: found.name.clone(),
            directory: found.path.clone(),
        })
        .collect::<Vec<_>>();
    let governed = files
        .iter()
        .filter(|file| {
            !local_packages
                .iter()
                .any(|package| containment(file, &package.directory) == Containment::Inside)
        })
        .cloned()
        .collect();
    Partitioned {
        local_packages,
        governed,
    }
}

/// The directories a walk starts from: the prefix of each pattern the workspace's `include` resolves to, the root for
/// a list of paths, sorted and once each.
#[must_use]
pub fn prefixes(include: &ResolvedSelectors) -> Vec<RepoPath> {
    let mut prefixes = include
        .0
        .iter()
        .map(|item| match item {
            Resolved::Pattern(matcher) => matcher.prefix.clone(),
            Resolved::Paths(_) => RepoPath::default(),
        })
        .collect::<Vec<_>>();
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

/// The repository's own exclude file, which shapes every walk.
#[must_use]
pub fn git_exclude() -> RepoPath {
    RepoPath(GIT_EXCLUDE.into())
}

/// The paths of `within`, given relative to where the run started, as paths of the repository.
#[must_use]
pub fn within_paths(start: &RepoPath, within: &[RepoPath]) -> Vec<RepoPath> {
    within
        .iter()
        .map(|path| join(start, PathText(&path.0)).unwrap_or_else(|| path.clone()))
        .collect()
}

/// The listing's paths one to a line, as bytes.
#[must_use]
pub fn listing_text(files: &[RepoPath]) -> ListingText {
    let mut listing = Vec::new();
    for (index, path) in files.iter().enumerate() {
        if index > 0 {
            listing.push(b'\n');
        }
        listing.extend_from_slice(path.0.as_bytes());
    }
    ListingText(listing)
}

/// The lock a load makes: the packages the workspace names directly, sorted and once each, and every package it uses.
#[must_use]
pub fn made_lock(section: &WorkspaceSection, packages: Vec<LockedPackage>) -> Lock {
    let mut dependencies = section
        .dependencies
        .iter()
        .map(|dependency| dependency.package.clone())
        .collect::<Vec<PackageName>>();
    dependencies.sort();
    dependencies.dedup();
    Lock {
        dependencies,
        packages,
    }
}

/// What ryo reads: the files the workspace governs below the lint call's root, none under a `rules-exclude`, so ryo
/// walks nothing and sees no local package and no excluded file.
#[must_use]
pub fn listed(plan: &Plan) -> Vec<RepoPath> {
    plan.files
        .iter()
        .filter(|file| !plan.excluded.contains(*file))
        .filter_map(|file| {
            relative(file, &plan.base)
                .filter(|rest| !rest.0.is_empty())
                .map(|rest| RepoPath(rest.0.into()))
        })
        .collect()
}

/// `path`, relative to the lint call's root, as a path of the repository.
#[must_use]
pub fn repository_path(base: &RepoPath, path: RepoPath) -> RepoPath {
    // Rooted at the repository, a path is already the repository's.
    if base.0.is_empty() {
        return path;
    }
    join(base, PathText(&path.0)).unwrap_or(path)
}

/// The rules whose queries this ryo refuses, `invalid` by query: each a `#query` error in `findings`, on the rule or on
/// the conditions it answers.
pub fn refusals(
    plan: &Plan,
    invalid: &BTreeMap<QueryText, RefusalReason>,
    findings: &mut Vec<Finding>,
) -> BTreeSet<LintRuleId> {
    let mut refused = BTreeSet::new();
    let anchor = plan.anchors.first().cloned().unwrap_or_default();
    for rule in &plan.rules {
        let Some(RefusalReason(reason)) = invalid.get(&rule.query) else {
            continue;
        };
        let _: bool = refused.insert(rule.id.clone());
        match plan.purposes.get(&rule.id) {
            Some(Purpose::Rule(rendering)) => findings.push(error(
                check_id(&rendering.rule.0, QUERY),
                anchor.clone(),
                format!(
                    "`{}`'s query is not one this ryo runs: {reason}",
                    rendering.rule.0
                ),
            )),
            Some(Purpose::Condition { identity, .. }) => findings.push(error(
                check_id(&identity.0, QUERY),
                anchor.clone(),
                format!(
                    "`{}`'s query is not one this ryo runs, so the conditions it answers are not: {reason}",
                    identity.0
                ),
            )),
            Some(Purpose::Comments | Purpose::Declarations(_) | Purpose::Imports(_)) | None => {}
        }
    }
    refused
}

/// The finding of a rule the lint call refused while it ran, when the rule is one of the plan's and governs a file read:
/// a `#query` error. A rule that governs none of the files read has nothing to find there, so its refusal is none.
#[must_use]
pub fn runtime_refusal(
    plan: &Plan,
    rule: &LintRuleId,
    reason: &RefusalReason,
    governance: Governance,
) -> Option<Finding> {
    let Some(Purpose::Rule(rendering)) = plan.purposes.get(rule) else {
        return None;
    };
    if governance == Governance::Idle {
        return None;
    }
    let anchor = plan.anchors.first().cloned().unwrap_or_default();
    Some(error(
        check_id(&rendering.rule.0, QUERY),
        anchor,
        format!(
            "`{}` cannot run on this ryo: {}",
            rendering.rule.0, reason.0
        ),
    ))
}
