//! The `workspaces` service: which workspace holds a package, and which packages a workspace holds, with no IO and no ports. What the
//! files hold is what the pass has been told; what it has not been told, it wants (`Looking`).

use std::collections::BTreeSet;

use super::constants::{HERE, PATTERN, PENDING, SEPARATOR, UP};
use super::errors::WorkspacesError;
use super::types::{Configured, Exclusion, Globbing, Looking, Reason, SegmentMatch};
use crate::domain::dependencies::service::declared_of;
use crate::domain::dependencies::types::Shared;
use crate::domain::paths::service::{directory_of, join, manifest_in, settlement_of, told, within};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{Entry, EntryKind, EntryName, Matching, Settlement};
use crate::domain::paths::types::{Lying, Relative};
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::Pattern;
use crate::domain::targets::types::WorkspaceSection;

/// The manifest at `path`, as it was told; why it cannot be read when it could not be, and a stand-in while it is wanted.
pub(crate) fn load(looking: &mut Looking, path: &RepoPath) -> Result<Manifest, Reason> {
    told(&mut looking.manifests, path, || Err(Reason(PENDING.into())))
}

/// What the directory `directory` holds, as it was told.
pub(crate) fn entries_of(looking: &mut Looking, directory: &RepoPath) -> Vec<Entry> {
    told(&mut looking.directories, directory, Vec::new)
}

/// Whether `name` matches `pattern`, as it was told.
pub(crate) fn matching_of(looking: &mut Looking, pattern: &Pattern, name: &EntryName) -> Matching {
    told(
        &mut looking.matches,
        &SegmentMatch {
            pattern: pattern.clone(),
            name: name.clone(),
        },
        || Matching::Differs,
    )
}

/// The configurations above `directory`, nearest first, as they were told.
pub(crate) fn configurations_above(looking: &mut Looking, directory: &RepoPath) -> Vec<Configured> {
    told(&mut looking.configurations, directory, Vec::new)
}

/// Whether the text has a pattern's characters.
fn is_pattern(text: &Pattern) -> Globbing {
    if text.contains(PATTERN) {
        Globbing::Glob
    } else {
        Globbing::Plain
    }
}

/// The manifest of the workspace the package at `manifest` is in, when there is one: its own when it has a `[workspace]`, the one
/// its `package.workspace` names, else the nearest above it that has one.
#[must_use]
pub(crate) fn root_of(looking: &mut Looking, path: &RepoPath) -> Option<RepoPath> {
    // A manifest that cannot be read is refused where the packages are read; here it names no workspace.
    let Ok(manifest) = load(looking, path) else {
        return None;
    };
    if matches!(manifest.workspace, Section::Present(_)) {
        return Some(path.clone());
    }
    if let Section::Present(package) = &manifest.package
        && let Section::Present(named) = &package.workspace
    {
        return join(&directory_of(path), &Relative(named.0.clone()))
            .map(|directory| manifest_in(&directory));
    }
    let mut directory = directory_of(path);
    while !directory.is_empty() {
        directory = directory_of(&directory);
        let candidate = manifest_in(&directory);
        if load(looking, &candidate).is_ok_and(|each| matches!(each.workspace, Section::Present(_)))
        {
            return Some(candidate);
        }
    }
    None
}

/// Whether the workspace in `root` leaves the directory `path` out.
#[must_use]
pub(crate) fn excluded(
    workspace: &WorkspaceSection,
    root: &RepoPath,
    path: &RepoPath,
) -> Exclusion {
    let left_out = workspace
        .exclude
        .iter()
        .filter_map(|each| join(root, &Relative(each.0.clone())))
        .any(|each| within(path, &each) == Lying::Within);
    let named = workspace
        .members
        .iter()
        .filter(|each| is_pattern(each) == Globbing::Plain)
        .filter_map(|each| join(root, &Relative(each.0.clone())))
        .any(|each| within(path, &each) == Lying::Within);
    if left_out && !named {
        Exclusion::Excluded
    } else {
        Exclusion::Included
    }
}

/// The directories below `directory` that a pattern of parts cut at `/` names: each part is a pattern that does not cross a `/`.
fn expand(looking: &mut Looking, directory: &RepoPath, pattern: &Pattern) -> Vec<RepoPath> {
    let mut found = vec![directory.clone()];
    for part in pattern
        .split(SEPARATOR)
        .filter(|part| !part.is_empty() && *part != HERE)
    {
        let mut next = Vec::new();
        for parent in &found {
            let part = Pattern(part.into());
            if *part.0 == *UP {
                next.extend(join(parent, &Relative(part.0.clone())));
            } else if is_pattern(&part) == Globbing::Glob {
                for entry in entries_of(looking, parent) {
                    if entry.kind == EntryKind::Directory
                        && matching_of(looking, &part, &entry.name) == Matching::Matches
                    {
                        next.extend(join(parent, &Relative(entry.name.0.clone())));
                    }
                }
            } else {
                next.extend(join(parent, &Relative(part.0.clone())));
            }
        }
        found = next;
    }
    found
}

/// The manifests of the members of the workspace whose manifest is `root`: the ones `members` names, the root package, and the
/// packages the members depend on by path from inside the workspace's directory.
///
/// # Errors
///
/// `Unreadable` when the root cannot be read, `Member` when a member cannot.
pub(crate) fn members_of(
    looking: &mut Looking,
    root: &RepoPath,
) -> Result<BTreeSet<RepoPath>, WorkspacesError> {
    let table = load(looking, root).map_err(|reason| WorkspacesError::Unreadable {
        manifest: root.clone(),
        reason,
    })?;
    let Section::Present(workspace) = &table.workspace else {
        return Ok(BTreeSet::from([root.clone()]));
    };
    let directory = directory_of(root);
    // A virtual manifest, with no package of its own, is no member of its workspace.
    let mut members = match table.package {
        Section::Absent => BTreeSet::new(),
        Section::Present(_) => BTreeSet::from([root.clone()]),
    };
    for pattern in &workspace.members {
        match is_pattern(pattern) {
            Globbing::Glob => {
                for found in expand(looking, &directory, pattern) {
                    let manifest = manifest_in(&found);
                    if excluded(workspace, &directory, &found) == Exclusion::Included
                        && load(looking, &manifest).is_ok()
                    {
                        members.extend([manifest]);
                    }
                }
            }
            Globbing::Plain => members.extend(
                join(&directory, &Relative(pattern.0.clone())).map(|found| manifest_in(&found)),
            ),
        }
    }
    // Packages the members depend on by path, from inside the workspace's directory, are members too.
    let mut queue: Vec<RepoPath> = members.iter().cloned().collect();
    while let Some(member) = queue.pop() {
        let each = load(looking, &member).map_err(|reason| WorkspacesError::Member {
            manifest: member.clone(),
            reason,
        })?;
        // A package `[patch]` puts in a directory is not made a member by it, as Cargo does not.
        let shared = Shared {
            directory: &directory,
            dependencies: &workspace.dependencies,
            patches: &[],
        };
        for dependency in declared_of(
            &directory_of(&member),
            &each,
            shared,
            &mut looking.platforms,
        ) {
            let Some(target) = dependency.target else {
                continue;
            };
            let inside = directory_of(&target);
            if within(&inside, &directory) == Lying::Within
                && !members.contains(&target)
                && excluded(workspace, &directory, &inside) == Exclusion::Included
                && load(looking, &target)
                    .is_ok_and(|found| matches!(found.package, Section::Present(_)))
            {
                members.extend([target.clone()]);
                queue.push(target);
            }
        }
    }
    Ok(members)
}

/// Whether a pass met anything it was not told: any of the things it asks about.
#[must_use]
pub(crate) fn settled(looking: &Looking) -> Settlement {
    let all = [
        settlement_of(&looking.manifests),
        settlement_of(&looking.directories),
        settlement_of(&looking.matches),
        settlement_of(&looking.configurations),
        settlement_of(&looking.platforms),
    ];
    if all.iter().all(|each| *each == Settlement::Settled) {
        Settlement::Settled
    } else {
        Settlement::Wanting
    }
}
