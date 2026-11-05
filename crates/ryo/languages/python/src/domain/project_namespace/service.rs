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
) -> Vec<EntryKind> {
    let Some(search_roots) = search_roots else {
        return vec![EntryKind::Unknown];
    };
    let mut module_present = false;
    let mut package_present = false;
    let mut namespace_present = false;
    let mut unknown = false;
    for search in search_roots {
        let [module, stub, directory, initializer, stub_initializer] =
            entry_paths(search, filesystem).map(|(path, file)| candidate(&path, file));
        module_present |= module == Some(true) || stub == Some(true);
        package_present |= directory == Some(true)
            && (initializer == Some(true) || stub_initializer == Some(true));
        // These are observed entries, not import-resolution precedence. A
        // sibling module file cannot erase an independently observed directory.
        namespace_present |= directory == Some(true)
            && initializer == Some(false)
            && stub_initializer == Some(false);
        unknown |= [module, stub, directory, initializer, stub_initializer]
            .into_iter()
            .any(|state| state.is_none());
    }
    let mut kinds = Vec::new();
    for (present, kind) in [
        (module_present, EntryKind::Module),
        (package_present, EntryKind::Package),
        (namespace_present, EntryKind::Namespace),
    ] {
        if present {
            kinds.push(kind);
        }
    }
    // A positive observation does not complete the other search roots. Keep
    // uncertainty alongside it so consumers cannot turn an unobserved possible
    // module or package into a complete namespace-only answer.
    if unknown {
        kinds.push(EntryKind::Unknown);
    } else if kinds.is_empty() {
        kinds.push(EntryKind::Absent);
    }
    kinds
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
    let mut classified: BTreeMap<String, Vec<EntryKind>> = paths
        .iter()
        .map(|(dotted, filesystem)| {
            (
                dotted.clone(),
                classify(search_roots, filesystem, &candidate),
            )
        })
        .collect();
    // A supplied child without a known parent cannot prove a project path. A
    // module file and a directory of the same name can coexist; the physical
    // child remains observed even though Python's import resolver may choose
    // the module file. This fact does not decide import resolution.
    for (path, _) in &paths {
        let Some((parent, _)) = path.rsplit_once('.') else {
            continue;
        };
        let present = |kinds: Option<&Vec<EntryKind>>| {
            kinds.is_some_and(|kinds| {
                kinds.iter().any(|kind| {
                    matches!(
                        kind,
                        EntryKind::Module | EntryKind::Package | EntryKind::Namespace
                    )
                })
            })
        };
        if present(classified.get(path)) && !present(classified.get(parent)) {
            classified.insert(path.clone(), vec![EntryKind::Unknown]);
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
        .flat_map(|(file, root, path)| {
            classified[&path].iter().map(move |&kind| {
                let state = match kind {
                    EntryKind::Module | EntryKind::Package | EntryKind::Namespace => {
                        MembershipState::Present
                    }
                    EntryKind::Absent => MembershipState::Absent,
                    EntryKind::Unknown => MembershipState::Unknown,
                };
                Membership {
                    file,
                    root: root.clone(),
                    path: path.clone().into_boxed_str(),
                    state,
                    kind,
                }
            })
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
