//! The `closing` service: free functions named by verb over the closed directories of a placement, with no IO and no ports: the facts
//! the closed query decides the strays from, and the findings its answers come to.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    ClosedDirectory, ClosedOwner, Closings, DirectoryDepth, EnclosedFile, GroupNumber, Nested,
    Occupancy, OccupiedDirectory, Prefix, Stray, StrayNumber,
};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::types::ClosedChoice;
use crate::domain::findings::constants::CLOSED;
use crate::domain::findings::service::{check_id, error, finding, said};
use crate::domain::findings::types::{Finding, Place};
use crate::domain::paths::service::{beneath, containment, parent};
use crate::domain::paths::types::{Containment, RepoPath};
use crate::domain::scopes::constants::RULES_EXCLUDE_HELP;
use crate::domain::scopes::service::{composition_of, owns, places_files};
use crate::domain::scopes::types::{FilePlacing, Origin, PlacedFile, Scope, ScopeIndex};

/// A directory as the prefix of the paths beneath it: the path and a `/`, none for the root.
#[must_use]
pub fn prefix_of(directory: &RepoPath) -> Prefix {
    if directory.0.is_empty() {
        Prefix(Box::default())
    } else {
        Prefix(format!("{}/", directory.0).into())
    }
}

/// How many directories `directory` is below the root.
fn depth_of(directory: &RepoPath) -> DirectoryDepth {
    if directory.0.is_empty() {
        DirectoryDepth(0)
    } else {
        DirectoryDepth(u32::try_from(directory.0.split('/').count()).unwrap_or(u32::MAX))
    }
}

/// The directories each of the scopes `owners` placed files in, as the files in `placed` say.
fn occupancies(
    placed: &BTreeMap<RepoPath, PlacedFile>,
    owners: &[ScopeIndex],
    group: GroupNumber,
) -> Vec<Occupancy> {
    let mut occupied: BTreeSet<OccupiedDirectory> = BTreeSet::new();
    for (file, placement) in placed {
        if owners.contains(&placement.scope) {
            let _: bool = occupied.insert(OccupiedDirectory {
                owner: placement.scope,
                directory: parent(file).unwrap_or_default(),
            });
        }
    }
    occupied
        .into_iter()
        .map(|OccupiedDirectory { owner, directory }| Occupancy {
            group,
            owner,
            prefix: prefix_of(&directory),
            depth: depth_of(&directory),
        })
        .collect()
}

/// What placement leaves of the closed directories: which scopes close each, the files beneath it, the files templates place and
/// the `tastemaker.toml` files, the instances nested in it and the directories its scopes placed files in. Which of the files are
/// strays, and whose, is the closed query's. A scope that places no file closes nothing.
#[must_use]
pub fn facts(
    workspace: &LoadedWorkspace,
    scopes: &[Scope],
    placed: &BTreeMap<RepoPath, PlacedFile>,
) -> Closings {
    let mut closings = Closings::default();
    let mut closed: BTreeMap<&RepoPath, Vec<ScopeIndex>> = BTreeMap::new();
    for (index, scope) in scopes.iter().enumerate() {
        if scope.closed == ClosedChoice::Closed
            && places_files(workspace, composition_of(workspace, &scope.composition))
                == FilePlacing::PlacesFiles
        {
            let _: bool = closings
                .evaluated
                .insert(check_id(&scope.identity.0, CLOSED));
            closed
                .entry(&scope.dir)
                .or_default()
                .push(ScopeIndex(index));
        }
    }
    // Only an instance that owns its directory takes it out of the closed one above, as `nest` decides.
    let instances = scopes
        .iter()
        .skip(1)
        .filter(|scope| owns(workspace, scope) == FilePlacing::PlacesFiles)
        .map(|scope| &scope.dir)
        .collect::<BTreeSet<_>>();
    closings.own = scopes
        .iter()
        .filter(|scope| matches!(scope.origin, Origin::File))
        .map(|scope| scope.anchor.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    closings.placed = placed.keys().cloned().collect();
    let mut stray = 0_u32;
    for (number, (directory, owners)) in closed.into_iter().enumerate() {
        let group = GroupNumber(u32::try_from(number).unwrap_or(u32::MAX));
        closings.directories.push(ClosedDirectory {
            group,
            directory: directory.clone(),
            owners: owners
                .iter()
                .filter_map(|owner| {
                    scopes.get(owner.0).map(|scope| ClosedOwner {
                        scope: *owner,
                        identity: scope.identity.clone(),
                    })
                })
                .collect(),
        });
        for file in beneath(&workspace.files, directory) {
            closings.candidates.push(EnclosedFile {
                stray: StrayNumber(stray),
                group,
                path: file.clone(),
                home: prefix_of(&parent(file).unwrap_or_default()),
            });
            stray = stray.saturating_add(1);
        }
        for instance in instances.iter().filter(|instance| {
            **instance != directory && containment(instance, directory) == Containment::Inside
        }) {
            closings.nested.push(Nested {
                group,
                prefix: prefix_of(instance),
            });
        }
        closings
            .occupied
            .extend(occupancies(placed, &owners, group));
    }
    closings
}

/// The findings of the strays the closed query decided, in the order of the files: each file, named after the scope the query chose,
/// is an error; one under a `rules-exclude` says that key keeps rules off a path and places nothing.
#[must_use]
pub fn findings(
    closings: &Closings,
    excluded: &BTreeSet<RepoPath>,
    strays: &[Stray],
) -> Vec<Finding> {
    let mut ordered = strays.to_vec();
    ordered.sort();
    let mut found = Vec::new();
    for each in ordered {
        let Some(candidate) = closings
            .candidates
            .get(usize::try_from(each.stray.0).unwrap_or(usize::MAX))
        else {
            continue;
        };
        let Some(group) = closings
            .directories
            .get(usize::try_from(candidate.group.0).unwrap_or(usize::MAX))
        else {
            continue;
        };
        let Some(owning) = group
            .owners
            .iter()
            .find(|owner| owner.scope == each.owner)
            .or_else(|| group.owners.last())
        else {
            continue;
        };
        let identity_text = &owning.identity.0;
        let message = format!(
            "`{}` is not placed by any template, and `{identity_text}` at `{}` is closed",
            candidate.path.0,
            if group.directory.0.is_empty() {
                crate::domain::scenario_files::constants::HERE
            } else {
                &group.directory.0
            }
        );
        if excluded.contains(&candidate.path) {
            found.push(finding(
                check_id(identity_text, CLOSED),
                candidate.path.clone(),
                Place::Whole,
                message,
                said(RULES_EXCLUDE_HELP.to_owned()),
            ));
        } else {
            found.push(error(
                check_id(identity_text, CLOSED),
                candidate.path.clone(),
                message,
            ));
        }
    }
    found
}
