//! Pure project namespace policy over imported roots and one engine-observed entry batch.
use std::collections::{BTreeMap, BTreeSet};

use ryo::languages::{FileId, FilePath};

use super::types::{EntryKind, ImportedRoot, Membership, MembershipState, NamespaceMemberships};
use crate::domain::fact_notes::types::Note;

/// First components of absolute imports, retaining the importing file's identity.
#[must_use]
pub(crate) fn imported_roots(notes: &[Option<Note>]) -> Vec<ImportedRoot> {
    let mut roots = BTreeSet::new();
    for (number, note) in notes.iter().enumerate() {
        let Some(note) = note else { continue };
        for import in &note.imports {
            if import.level.0 != 0 {
                continue;
            }
            let Some(root) = import
                .path
                .0
                .split('.')
                .next()
                .filter(|root| !root.is_empty())
            else {
                continue;
            };
            roots.insert(ImportedRoot {
                file: FileId(u32::try_from(number).unwrap_or(u32::MAX)),
                root: root.into(),
                path: import.path.0.clone().into(),
            });
        }
    }
    roots.into_iter().collect()
}

/// Dotted prefixes and their filesystem spellings, from the first component onward.
fn prefixes(path: &str) -> Vec<(String, String)> {
    let mut dotted = String::new();
    let mut filesystem = String::new();
    let mut result = Vec::new();
    for component in path.split('.').filter(|component| !component.is_empty()) {
        if !dotted.is_empty() {
            dotted.push('.');
            filesystem.push('/');
        }
        dotted.push_str(component);
        filesystem.push_str(component);
        result.push((dotted.clone(), filesystem.clone()));
    }
    result
}

fn entry_paths(search: &FilePath, filesystem: &str) -> [(FilePath, bool); 5] {
    [
        (beneath(search, &format!("{filesystem}.py")), true),
        (beneath(search, &format!("{filesystem}.pyi")), true),
        (beneath(search, filesystem), false),
        (beneath(search, &format!("{filesystem}/__init__.py")), true),
        (beneath(search, &format!("{filesystem}/__init__.pyi")), true),
    ]
}

/// A candidate beneath a declared root; no implicit repository or src root is supplied.
fn beneath(search: &FilePath, entry: &str) -> FilePath {
    if search.0.is_empty() {
        FilePath(entry.into())
    } else {
        FilePath(format!("{}/{entry}", search.0).into())
    }
}

/// Immediate entries needed to classify only the imported roots demanded by the query.
#[must_use]
pub(crate) fn candidates(
    search_roots: Option<&[FilePath]>,
    roots: &[ImportedRoot],
) -> Vec<FilePath> {
    let Some(search_roots) = search_roots else {
        return Vec::new();
    };
    let names: BTreeSet<String> = roots
        .iter()
        .flat_map(|root| {
            prefixes(&root.path)
                .into_iter()
                .map(|(_, filesystem)| filesystem)
        })
        .collect();
    let mut candidates = BTreeSet::new();
    for search in search_roots {
        for name in &names {
            for (path, _) in entry_paths(search, name) {
                candidates.insert(path);
            }
        }
    }
    candidates.into_iter().collect()
}

fn classify(
    search_roots: Option<&[FilePath]>,
    filesystem: &str,
    candidate: &impl Fn(&FilePath, bool) -> Option<bool>,
) -> EntryKind {
    let Some(search_roots) = search_roots else {
        return EntryKind::Unknown;
    };
    let mut namespace = false;
    let mut unknown = false;
    for search in search_roots {
        let [module, stub, directory, initializer, stub_initializer] =
            entry_paths(search, filesystem).map(|(path, file)| candidate(&path, file));
        if module == Some(true) || stub == Some(true) {
            return EntryKind::Module;
        }
        if directory == Some(true) && (initializer == Some(true) || stub_initializer == Some(true))
        {
            return EntryKind::Package;
        }
        if directory == Some(true)
            && module == Some(false)
            && stub == Some(false)
            && initializer == Some(false)
            && stub_initializer == Some(false)
        {
            namespace = true;
        } else if [module, stub, directory, initializer, stub_initializer]
            .into_iter()
            .any(|state| state.is_none())
        {
            unknown = true;
        }
    }
    if unknown {
        EntryKind::Unknown
    } else if namespace {
        EntryKind::Namespace
    } else {
        EntryKind::Absent
    }
}

/// Interpret only the supplied observations. Missing observations remain Unknown, never Absent.
#[must_use]
pub(crate) fn memberships(
    search_roots: Option<&[FilePath]>,
    roots: &[ImportedRoot],
    incomplete_notes: Vec<FileId>,
    candidate: impl Fn(&FilePath, bool) -> Option<bool>,
) -> NamespaceMemberships {
    let paths: BTreeSet<(String, String)> =
        roots.iter().flat_map(|root| prefixes(&root.path)).collect();
    let mut classified: BTreeMap<String, EntryKind> = paths
        .iter()
        .map(|(dotted, filesystem)| {
            (
                dotted.clone(),
                classify(search_roots, filesystem, &candidate),
            )
        })
        .collect();
    // A supplied child without its observed parent cannot prove a project path.
    for (path, _) in &paths {
        let Some((parent, _)) = path.rsplit_once('.') else {
            continue;
        };
        if matches!(
            classified.get(path),
            Some(EntryKind::Module | EntryKind::Package | EntryKind::Namespace)
        ) && !matches!(
            classified.get(parent),
            Some(EntryKind::Package | EntryKind::Namespace)
        ) {
            classified.insert(path.clone(), EntryKind::Unknown);
        }
    }
    let mut rows = BTreeSet::new();
    for imported in roots {
        for (path, _) in prefixes(&imported.path) {
            rows.insert((imported.file, imported.root.clone(), path));
        }
    }
    let rows = rows
        .into_iter()
        .map(|(file, root, path)| {
            let kind = classified.get(&path).copied().unwrap_or(EntryKind::Unknown);
            let state = match kind {
                EntryKind::Module | EntryKind::Package | EntryKind::Namespace => {
                    MembershipState::Present
                }
                EntryKind::Absent => MembershipState::Absent,
                EntryKind::Unknown => MembershipState::Unknown,
            };
            Membership {
                file,
                root,
                path: path.into_boxed_str(),
                state,
                kind,
            }
        })
        .collect();
    NamespaceMemberships {
        rows,
        incomplete_notes,
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
