//! Pure project namespace policy over imported roots and one engine-observed entry batch.
use std::collections::{BTreeMap, BTreeSet};

use ryo::languages::{FileId, FilePath};

use super::types::{
    EntryKind, EntryName, EntryShape, FilesystemPath, ImportPath, ImportedRoot, MembershipState,
    NamespaceMembership, NamespaceMemberships, Prefix, Presence, Probe,
};
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
                path: ImportPath::from(import.path.0.clone()),
            });
        }
    }
    roots.into_iter().collect()
}

/// Dotted prefixes and their filesystem spellings, from the first component onward.
fn prefixes(path: &ImportPath) -> Vec<Prefix> {
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
        result.push(Prefix {
            dotted: ImportPath::from(dotted.clone()),
            filesystem: FilesystemPath(filesystem.clone()),
        });
    }
    result
}

fn entry_paths(search: &FilePath, filesystem: &FilesystemPath) -> [Probe; 5] {
    let filesystem = &filesystem.0;
    let probe = |name: EntryName, shape: EntryShape| Probe {
        path: beneath(search, &name),
        shape,
    };
    [
        probe(EntryName(format!("{filesystem}.py")), EntryShape::File),
        probe(EntryName(format!("{filesystem}.pyi")), EntryShape::File),
        probe(EntryName(filesystem.clone()), EntryShape::Directory),
        probe(
            EntryName(format!("{filesystem}/__init__.py")),
            EntryShape::File,
        ),
        probe(
            EntryName(format!("{filesystem}/__init__.pyi")),
            EntryShape::File,
        ),
    ]
}

/// A candidate beneath a declared root; no implicit repository or src root is supplied.
fn beneath(search: &FilePath, entry: &EntryName) -> FilePath {
    if search.0.is_empty() {
        FilePath(entry.0.as_str().into())
    } else {
        FilePath(format!("{}/{}", search.0, entry.0).into())
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
    let names: BTreeSet<FilesystemPath> = roots
        .iter()
        .flat_map(|root| {
            prefixes(&root.path)
                .into_iter()
                .map(|prefix| prefix.filesystem)
        })
        .collect();
    let mut candidates = BTreeSet::new();
    for search in search_roots {
        for name in &names {
            for probe in entry_paths(search, name) {
                candidates.insert(probe.path);
            }
        }
    }
    candidates.into_iter().collect()
}

fn classify(
    search_roots: Option<&[FilePath]>,
    filesystem: &FilesystemPath,
    candidate: &impl Fn(&FilePath, EntryShape) -> Option<Presence>,
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
            entry_paths(search, filesystem).map(|probe| candidate(&probe.path, probe.shape));
        module_present |= module == Some(Presence::Present) || stub == Some(Presence::Present);
        package_present |= directory == Some(Presence::Present)
            && (initializer == Some(Presence::Present)
                || stub_initializer == Some(Presence::Present));
        // These are observed entries, not import-resolution precedence. A
        // sibling module file cannot erase an independently observed directory.
        namespace_present |= directory == Some(Presence::Present)
            && initializer == Some(Presence::Absent)
            && stub_initializer == Some(Presence::Absent);
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
    candidate: impl Fn(&FilePath, EntryShape) -> Option<Presence>,
) -> NamespaceMemberships {
    let paths: BTreeSet<Prefix> = roots.iter().flat_map(|root| prefixes(&root.path)).collect();
    let mut classified: BTreeMap<ImportPath, Vec<EntryKind>> = paths
        .iter()
        .map(|prefix| {
            (
                prefix.dotted.clone(),
                classify(search_roots, &prefix.filesystem, &candidate),
            )
        })
        .collect();
    // A supplied child without a known parent cannot prove a project path. A
    // module file and a directory of the same name can coexist; the physical
    // child remains observed even though Python's import resolver may choose
    // the module file. This fact does not decide import resolution.
    for prefix in &paths {
        let path = &prefix.dotted;
        let Some((parent, _)) = path.0.rsplit_once('.') else {
            continue;
        };
        let parent = ImportPath::from(parent);
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
        if present(classified.get(path)) && !present(classified.get(&parent)) {
            classified.insert(path.clone(), vec![EntryKind::Unknown]);
        }
    }

    let mut rows = BTreeSet::new();
    for imported in roots {
        for prefix in prefixes(&imported.path) {
            rows.insert(ImportedRoot {
                file: imported.file,
                root: imported.root.clone(),
                path: prefix.dotted,
            });
        }
    }
    let rows = rows
        .into_iter()
        .flat_map(|key| {
            classified[&key.path].iter().map(move |&kind| {
                let state = match kind {
                    EntryKind::Module | EntryKind::Package | EntryKind::Namespace => {
                        MembershipState::Present
                    }
                    EntryKind::Absent => MembershipState::Absent,
                    EntryKind::Unknown => MembershipState::Unknown,
                };
                NamespaceMembership {
                    file: key.file,
                    root: key.root.clone(),
                    path: key.path.clone(),
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
