//! The `applying` service: free functions named by verb that merge the applications matched here into the rules of the
//! lint call, with no IO and no ports. Identical applications (one release with one set of params) reaching a file merge;
//! one release reaching a file with two sets of params is a finding.

use crate::domain::findings::service::of_rule;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rayon::prelude::*;

use super::types::{Application, Applied, Group, ReachedFile};
use crate::domain::applications::types::{
    Carried, Compilers, GlobText, HandedRule, Nameable, Plan,
};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::findings::constants::COVERAGE;
use crate::domain::findings::service::{check_id, error};
use crate::domain::findings::types::{Finding, RuleId};
use crate::domain::globs::service::{compress, counted};
use crate::domain::packages::service::release_id;
use crate::domain::packages::types::Release;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::placement::types::Placement;
use crate::domain::reaching::service::reaches;
use crate::domain::reaching::types::{Selection, SelectorPair};
use crate::domain::scopes::types::Origin;

/// Each application's reached files: one rule of the lint call each.
fn grouped(applied: Applied) -> Vec<Group> {
    applied
        .0
        .into_iter()
        .map(|(application, reached)| Group {
            release: application.release,
            params: application.params,
            files: reached.0,
        })
        .collect()
}

/// The rules of the lint call for the applications matched here: each release and params, and the files it
/// reaches compressed into globs; a release reaching a file with two sets of params is a `#coverage` error.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
pub fn matched_rules(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    matched: &[Carried<'_>],
    known: &Nameable,
    matching: Compilers<'_>,
    plan: &mut Plan,
) -> Result<Vec<HandedRule>, PlacementError> {
    let reached = reaches(workspace, placement, matched, known, matching);
    let mut applied = Applied::default();
    for application in matched {
        let scope = &placement.scopes[application.scope.0];
        let key = Selection {
            scope: application.scope,
            pair: SelectorPair {
                include: &application.entry.header.include,
                exclude: &application.entry.header.exclude,
            },
        };
        let files = match &reached[&key] {
            Ok(files) => files,
            Err(failure) => return Err(failure.clone()),
        };
        if files.is_empty() {
            unreached(scope, application, plan);
            continue;
        }
        let slot = applied
            .0
            .entry(Application {
                release: application.release.clone(),
                params: application.params.clone(),
            })
            .or_default();
        slot.0.extend(files.iter().cloned());
    }
    conflicts(&mut plan.findings, &applied);
    let groups = grouped(applied);
    // A rule is evaluated where it reaches a file: one that reaches none here runs nowhere, and a scenario that proves it
    // proves nothing.
    for group in &groups {
        let _: bool = plan
            .evaluated
            .insert(of_rule(RuleId(release_id(&group.release).0.into())));
    }
    let local = workspace
        .local_packages
        .iter()
        .map(|package| package.directory.clone())
        .collect::<Vec<_>>();
    let governed = counted(&workspace.files);
    // Groups that reach the same files share their globs: each set is compressed once, the sets in parallel.
    let distinct: HashSet<&BTreeSet<RepoPath>> = groups.iter().map(|group| &group.files).collect();
    let globs: HashMap<&BTreeSet<RepoPath>, Vec<GlobText>> = distinct
        .into_par_iter()
        .map(|files| (files, compress(files, &governed, &local, &plan.base)))
        .collect();
    Ok(groups
        .iter()
        .map(|group| HandedRule {
            release: group.release.clone(),
            params: group.params.clone(),
            include: globs[&group.files].clone(),
            exclude: Vec::new(),
        })
        .collect())
}

/// An application that reaches no file. A spec's rule may find nothing in one of its instances (a part the instance does
/// not have); an entry of a `tastemaker.toml` names its files itself, so one that reaches none is a `#coverage` error.
fn unreached(
    scope: &crate::domain::scopes::types::Scope,
    application: &Carried<'_>,
    plan: &mut Plan,
) {
    if matches!(scope.origin, Origin::File) {
        let id = release_id(&application.release);
        plan.findings.push(error(
            check_id(&id.0, COVERAGE),
            scope.anchor.clone(),
            format!(
                "`{}` reaches no file: its include matches nothing here",
                id.0
            ),
        ));
    }
}

/// Marks the rules handed to ryo whole as evaluated, each where it reaches a file. ryo matches a handed rule's globs
/// itself, so only the applications of a `tastemaker.toml` are matched here as well: one that reaches no file is a
/// `#coverage` error, and a release none of whose applications reaches a file is not evaluated.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile.
#[expect(
    clippy::indexing_slicing,
    reason = "an application's scope is made by place_at over these scopes: an index past them is a bug that must crash, never a skipped #coverage finding"
)]
pub fn handed_reach(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    applications: &[Carried<'_>],
    handed: &[HandedRule],
    known: &Nameable,
    matching: Compilers<'_>,
    plan: &mut Plan,
) -> Result<(), PlacementError> {
    let releases: BTreeSet<&Release> = handed.iter().map(|rule| &rule.release).collect();
    let in_file = |application: &&Carried<'_>| {
        matches!(placement.scopes[application.scope.0].origin, Origin::File)
    };
    let own: Vec<Carried<'_>> = applications
        .iter()
        .filter(|application| releases.contains(&application.release))
        .filter(in_file)
        .cloned()
        .collect();
    let reached = reaches(workspace, placement, &own, known, matching);
    let mut silent: BTreeSet<&Release> = BTreeSet::new();
    let mut heard: BTreeSet<&Release> = BTreeSet::new();
    for application in &own {
        let scope = &placement.scopes[application.scope.0];
        let key = Selection {
            scope: application.scope,
            pair: SelectorPair {
                include: &application.entry.header.include,
                exclude: &application.entry.header.exclude,
            },
        };
        // `reaches` answers for every application it is given: a missing answer is a bug that must crash, never a reach.
        match &reached[&key] {
            Ok(files) if files.is_empty() => {
                unreached(scope, application, plan);
                let _: bool = silent.insert(&application.release);
            }
            Err(failure) => return Err(failure.clone()),
            Ok(_) => {
                let _: bool = heard.insert(&application.release);
            }
        }
    }
    // A release a spec applies is handed for its instances as well, which ryo matches whole: a rule of a spec may answer
    // for what a part lacks, reaching no file by design, so it counts as evaluated.
    let instanced: BTreeSet<&Release> = applications
        .iter()
        .filter(|application| !in_file(application))
        .map(|application| &application.release)
        .collect();
    for release in releases {
        if heard.contains(release) || instanced.contains(release) || !silent.contains(release) {
            let _: bool = plan
                .evaluated
                .insert(of_rule(RuleId(release_id(release).0.into())));
        }
    }
    Ok(())
}

/// One release reaching a file with two sets of params: the file needs its own entry.
fn conflicts(findings: &mut Vec<Finding>, applied: &Applied) {
    // Only a release applied with more than one set of params can conflict.
    let mut sets_of: BTreeMap<&Release, usize> = BTreeMap::new();
    for application in applied.0.keys() {
        *sets_of.entry(&application.release).or_default() += 1;
    }
    let mut sets: BTreeMap<ReachedFile<'_>, usize> = BTreeMap::new();
    for (application, reached) in &applied.0 {
        if sets_of
            .get(&application.release)
            .is_none_or(|count| *count < 2)
        {
            continue;
        }
        for file in &reached.0 {
            *sets
                .entry(ReachedFile {
                    release: &application.release,
                    file,
                })
                .or_default() += 1;
        }
    }
    for (reached, count) in sets {
        if count > 1 {
            let id = release_id(reached.release);
            findings.push(error(
                check_id(&id.0, COVERAGE),
                reached.file.clone(),
                format!("`{}` reaches this file with different params: give the path its own entry and exclude it from the general one", id.0),
            ));
        }
    }
}
