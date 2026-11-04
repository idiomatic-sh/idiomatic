//! Pure project namespace policy over imported roots and one engine-observed entry batch.
use std::collections::{BTreeMap, BTreeSet};

use ryo::languages::{FileId, FilePath};

use super::types::{ImportedRoot, Membership, MembershipState, NamespaceMemberships};
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
            });
        }
    }
    roots.into_iter().collect()
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
    let names: BTreeSet<&str> = roots.iter().map(|root| root.root.as_ref()).collect();
    let mut candidates = BTreeSet::new();
    for search in search_roots {
        for name in &names {
            candidates.insert(beneath(search, &format!("{name}.py")));
            candidates.insert(beneath(search, &format!("{name}.pyi")));
            candidates.insert(beneath(search, name));
        }
    }
    candidates.into_iter().collect()
}

/// Interpret only the supplied observations. Missing observations remain Unknown, never Absent.
#[must_use]
pub(crate) fn memberships(
    search_roots: Option<&[FilePath]>,
    roots: &[ImportedRoot],
    incomplete_notes: Vec<FileId>,
    candidate: impl Fn(&FilePath, bool) -> Option<bool>,
) -> NamespaceMemberships {
    let names: BTreeSet<&str> = roots.iter().map(|root| root.root.as_ref()).collect();
    let classified: BTreeMap<&str, MembershipState> = names
        .into_iter()
        .map(|name| {
            let state = match search_roots {
                None => MembershipState::Unknown,
                Some(search_roots) => {
                    let mut unknown = false;
                    let mut present = false;
                    for search in search_roots {
                        let paths = [
                            (beneath(search, &format!("{name}.py")), true),
                            (beneath(search, &format!("{name}.pyi")), true),
                            (beneath(search, name), false),
                        ];
                        for (path, file) in paths {
                            match candidate(&path, file) {
                                Some(true) => present = true,
                                None => unknown = true,
                                Some(false) => {}
                            }
                        }
                    }
                    if present {
                        MembershipState::Present
                    } else if unknown {
                        MembershipState::Unknown
                    } else {
                        MembershipState::Absent
                    }
                }
            };
            (name, state)
        })
        .collect();
    let rows = roots
        .iter()
        .map(|imported| Membership {
            file: imported.file,
            root: imported.root.clone(),
            state: classified
                .get(imported.root.as_ref())
                .copied()
                .unwrap_or(MembershipState::Unknown),
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
