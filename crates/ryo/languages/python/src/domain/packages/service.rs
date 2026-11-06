//! The `packages` service: free functions named by verb that read the requirements of a package, find the packages a repository's
//! manifests hold and need, and place each file in one, with no IO and no ports. A name is normalized as PEP 503 does it: lower case,
//! each run of `-`, `_` and `.` one `-`. A requirement is a PEP 508 string, `name[extras] (>=1) ; marker`, read for its name and marker.

use ryo::languages;

use super::constants::{
    CAUSE_SEPARATOR, DIRECT, FILE_SCHEME, GIT_SCHEME, JOINER, MARKER, NO_MARKER, PARTS, ROOT, UP,
};
use super::constants::{EXTRAS, EXTRAS_END, PARENTHESES};
use super::types::{
    Cause, FilePlacement, PackageLocation, ManifestPath, Marker, PackageMembership, Normalized, PackageName,
    Packages, Parsed, Reference, Specifier, Wired, WorkspaceMembership, Written,
};
use crate::domain::patterns::constants::{HERE, SEPARATOR};
use crate::domain::patterns::service::matches_directory;
use crate::domain::patterns::types::{Directory, Matching, Part, Pattern};

/// A name as PEP 503 normalizes it.
#[must_use]
pub(crate) fn normalized(name: &PackageName) -> Normalized {
    let mut text = String::with_capacity(name.0.len());
    let mut joined = false;
    for each in name.0.chars() {
        if PARTS.contains(&each) {
            if !joined && !text.is_empty() {
                text.push(JOINER);
            }
            joined = true;
        } else {
            text.extend(each.to_lowercase());
            joined = false;
        }
    }
    if text.ends_with(JOINER) {
        let _: Option<char> = text.pop();
    }
    Normalized(text.into())
}

/// Where the URL `url` says a package comes from: a git repository for `git+`, a path for `file:`, an archive for any other.
#[must_use]
pub(crate) fn url_origin(url: Part<'_>) -> languages::DependencyOrigin {
    let url = url.0.trim();
    if url.starts_with(GIT_SCHEME) {
        languages::DependencyOrigin::Git
    } else if url.starts_with(FILE_SCHEME) {
        languages::DependencyOrigin::Path
    } else {
        languages::DependencyOrigin::Url
    }
}

/// Where a PEP 508 requirement says its package comes from: the URL of its direct reference (`name @ url`), else the registry.
#[must_use]
pub(crate) fn origin_of(requirement: &Written) -> languages::DependencyOrigin {
    requirement
        .0
        .split(MARKER)
        .next()
        .and_then(|head| head.split_once(DIRECT))
        .map_or(languages::DependencyOrigin::Registry, |(_, url)| {
            url_origin(Part(url))
        })
}

/// What a requirement names, and the marker it carries.
#[must_use]
pub(crate) fn parsed(requirement: &Written) -> Parsed {
    let (head, marker) = requirement
        .0
        .split_once(MARKER)
        .map_or((&*requirement.0, NO_MARKER), |(head, marker)| {
            (head, marker.trim())
        });
    let head = head.trim_start();
    let end = head
        .find(|each| !(char::is_ascii_alphanumeric(&each) || PARTS.contains(&each)))
        .unwrap_or(head.len());
    let written = PackageName(head.get(..end).unwrap_or(head).into());
    Parsed {
        name: normalized(&written),
        written,
        marker: Marker(marker.into()),
    }
}

/// The version a PEP 508 requirement asks for, as written: what follows its name and its extras, up to its marker, without the
/// parentheses it may be enclosed in. Empty for a requirement that asks for none, or that names a URL (`name @ url`).
#[must_use]
pub(crate) fn specifier_of(requirement: &Written) -> Specifier {
    let head = requirement
        .0
        .split_once(MARKER)
        .map_or(&*requirement.0, |(head, _)| head)
        .trim_start();
    let end = head
        .find(|each| !(char::is_ascii_alphanumeric(&each) || PARTS.contains(&each)))
        .unwrap_or(head.len());
    let rest = head.get(end..).unwrap_or_default().trim_start();
    let rest = if rest.starts_with(EXTRAS) {
        rest.split_once(EXTRAS_END)
            .map_or(<&str>::default(), |(_, after)| after)
    } else {
        rest
    }
    .trim();
    if rest.starts_with(DIRECT) {
        return Specifier(Box::default());
    }
    Specifier(rest.trim_matches(PARENTHESES).trim().into())
}

/// The directory of a manifest: all of its path but the last part; empty for a manifest at the root.
#[must_use]
pub(crate) fn directory_of(manifest: &ManifestPath) -> Directory<'_> {
    Directory(
        manifest
            .0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(directory, _)| directory),
    )
}

/// The directory above `directory`; the root above the root.
fn above(directory: Directory<'_>) -> Directory<'_> {
    Directory(
        directory
            .0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(above, _)| above),
    )
}

/// The directory `relative` names from `directory`, as a requirement's reference: located when it stays inside the repository, else
/// unresolvable.
#[must_use]
pub(crate) fn locating(directory: Directory<'_>, relative: Part<'_>) -> Reference {
    let mut parts: Vec<&str> = directory
        .0
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect();
    for part in relative.0.split(SEPARATOR) {
        if part.is_empty() || part == HERE {
            continue;
        }
        if part == UP {
            if parts.pop().is_none() {
                return Reference::Unresolvable;
            }
        } else {
            parts.push(part);
        }
    }
    let mut location = String::new();
    for (place, part) in parts.iter().enumerate() {
        if place > 0 {
            location.push(SEPARATOR);
        }
        location.push_str(part);
    }
    Reference::Located(PackageLocation(location.into()))
}

/// The path `directory` has from `base`, when it is inside it.
fn within<'a>(base: Directory<'_>, directory: Directory<'a>) -> Option<Directory<'a>> {
    if base.0.is_empty() {
        return Some(directory);
    }
    directory
        .0
        .strip_prefix(base.0)
        .and_then(|rest| rest.strip_prefix(SEPARATOR))
        .map(Directory)
}

/// The packages the workspaces hold: those whose directory a pattern of `members` names, less those a pattern of `excludes` names,
/// patterns taken from the directory of the workspace's manifest.
#[must_use]
pub(crate) fn members_of(packages: &Packages) -> Vec<PackageMembership> {
    let mut found = Vec::new();
    for workspace in &packages.packages {
        if workspace.declared.members.is_empty() {
            continue;
        }
        let base = directory_of(&workspace.manifest);
        for member in &packages.packages {
            if member.manifest == workspace.manifest {
                continue;
            }
            let Some(inside) = within(base, directory_of(&member.manifest)) else {
                continue;
            };
            let named_by = |patterns: &[Pattern]| {
                patterns
                    .iter()
                    .any(|pattern| matches_directory(pattern, inside) == Matching::Matches)
            };
            if named_by(&workspace.declared.members) && !named_by(&workspace.declared.excludes) {
                found.push(PackageMembership {
                    workspace: workspace.manifest.clone(),
                    package: member.manifest.clone(),
                });
            }
        }
    }
    found
}

/// The number of the file of the manifest `manifest` among the files of the run, which are in the order of their paths.
#[must_use]
pub(crate) fn file_of(
    files: &[languages::FilePath],
    manifest: &ManifestPath,
) -> Option<languages::FileId> {
    match files.binary_search_by(|each| each.0.cmp(&manifest.0)) {
        Ok(place) => Some(languages::FileId(u32::try_from(place).unwrap_or(u32::MAX))),
        // The search answers where the manifest would go: the run does not list it.
        Err(_) => None,
    }
}

/// The name `text` as a name made after the files were read: the manifests' texts are in no file's bytes.
pub(crate) fn named(
    symbols: &mut languages::Symbols,
    text: languages::NameText<'_>,
) -> languages::Symbol {
    languages::intern_derived(symbols, languages::ByteSlice(&[]), text)
}

/// The causes for which the project is read only in part: each manifest the tool refused, with what it said.
pub(crate) fn causes_of(packages: &Packages) -> Vec<Cause> {
    packages
        .refused
        .iter()
        .map(|refusal| {
            Cause(
                format!(
                    "{}{CAUSE_SEPARATOR}{}",
                    refusal.manifest.0, refusal.reason.0
                )
                .into(),
            )
        })
        .collect()
}

/// The members of the workspaces, as the rows of `workspace_member`.
#[must_use]
pub(crate) fn holdings_in(
    files: &[languages::FilePath],
    packages: &Packages,
) -> Wired<WorkspaceMembership> {
    let rows = members_of(packages)
        .into_iter()
        .filter_map(|membership| {
            Some(WorkspaceMembership {
                workspace: file_of(files, &membership.workspace)?,
                package: file_of(files, &membership.package)?,
            })
        })
        .collect();
    Wired {
        rows,
        refused: causes_of(packages),
    }
}

/// The package of each file of the run, as the rows of `file_package`: the one whose directory is the nearest above the file, or the
/// file's own directory.
#[must_use]
pub(crate) fn placed_in(
    files: &[languages::FilePath],
    packages: &Packages,
) -> Wired<FilePlacement> {
    let mut rows = Vec::new();
    for (place, file) in files.iter().enumerate() {
        let path = ManifestPath(file.0.clone());
        let mut directory = directory_of(&path);
        let owner = loop {
            if let Some(package) = packages
                .packages
                .iter()
                .find(|package| directory_of(&package.manifest) == directory)
            {
                break Some(package);
            }
            if directory.0.is_empty() {
                break None;
            }
            directory = above(directory);
        };
        if let Some(package) = owner
            && let Some(manifest) = file_of(files, &package.manifest)
        {
            rows.push(FilePlacement {
                path: languages::FileId(u32::try_from(place).unwrap_or(u32::MAX)),
                package: manifest,
            });
        }
    }
    Wired {
        rows,
        refused: causes_of(packages),
    }
}
