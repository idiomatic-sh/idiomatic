//! The `links` service: free functions named by verb that link the files of a repository: the module tree of a package is drawn, and
//! the paths each file writes are resolved against the trees, with no IO and no ports. How the work spreads over the machine is the use
//! case's: each function here works on one package, one file or one note.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Drawn, Edge, Examination, Link, Linked, Packaged, Placed};
use crate::domain::fact_notes::types::Naming;
use crate::domain::module_tree::constants::SOURCE;
use crate::domain::module_tree::service::{resolve, tree};
use crate::domain::module_tree::types::{Cleanliness, Facts, Gap, ModulePath, Segment, Tree};
use crate::domain::package_index::constants::MANIFEST;
use crate::domain::package_index::service::{index, kept_files, package_of};
use crate::domain::package_index::types::{Index, Package, Place};
use crate::domain::paths::service::lexical_join;
use crate::domain::paths::types::Relative;
use crate::domain::paths::types::RepoPath;
use ryo::languages::{self, FilePath};

/// The place of a file among the repository's, as the number the rows hold it by.
#[must_use]
pub(crate) fn file_id(place: Place) -> languages::FileId {
    languages::FileId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// Where the path of a link is written.
#[must_use]
pub(crate) fn link_span(link: &Link) -> languages::Span {
    languages::Span {
        start: link.start,
        end: link.end,
    }
}

/// The package of each file of the repository, from what the manifests declared: the innermost one that holds it, by its manifest.
/// Nothing when the project has no manifest of the language.
#[must_use]
pub(crate) fn packaged_in(input: &languages::ManifestInput<'_, Linked>) -> Option<Packaged> {
    let packages = crate::domain::link_state::service::packages_in(input.resolved)?;
    let files = &input.request.files;
    let kept = kept_files(files, &packages.outputs);
    let index = index(&kept, &packages.roots);
    let placed = files
        .iter()
        .enumerate()
        .filter(|(_, file)| kept.contains(&RepoPath(file.0.clone())))
        .filter_map(|(place, file)| {
            let directory = package_of(&index, &RepoPath(file.0.clone()))?;
            Some(Placed {
                place: Place(place),
                package: lexical_join(directory, &Relative(MANIFEST.into())),
            })
        })
        .collect();
    Some(Packaged {
        files: placed,
        refused: packages.refused,
    })
}

/// The place of `path` among `files`, which are in the order of their places and in the order of their paths.
#[must_use]
pub(crate) fn place_of(files: &[FilePath], path: &RepoPath) -> Option<Place> {
    files
        .binary_search_by(|each| (*each.0).cmp(&*path.0))
        .ok()
        .map(Place)
}

/// The module trees of the crate roots of one package, drawn from the facts of its files: the package's directory and its trees.
/// `facts` are the facts of each source file, by its place.
#[must_use]
pub(crate) fn draw(
    kept: &BTreeSet<RepoPath>,
    files: &[FilePath],
    facts: &[Option<Facts>],
    package: &Package,
) -> Drawn {
    let facts_of = |path: &RepoPath| {
        place_of(files, path)
            .and_then(|place| facts.get(place.0))
            .and_then(Option::as_ref)
    };
    Drawn {
        directory: package.directory.clone(),
        trees: package
            .roots
            .iter()
            .map(|root| tree(kept, &package.directory, root, &facts_of))
            .collect::<Vec<Tree>>(),
    }
}

/// The modules that could not be followed in any of the trees, each once.
#[must_use]
pub(crate) fn gaps_in(trees: &BTreeMap<RepoPath, Vec<Tree>>) -> Vec<Gap> {
    let mut gaps: Vec<Gap> = trees
        .values()
        .flatten()
        .flat_map(|each| each.gaps.iter().cloned())
        .collect();
    gaps.sort();
    gaps.dedup();
    gaps
}

/// What linking came to, from the gaps and the examination of each file, in the order of the files.
#[must_use]
pub(crate) fn merged(gaps: Vec<Gap>, examined: Vec<Examination>) -> Linked {
    let mut linked = Linked {
        gaps,
        ..Linked::default()
    };
    for examination in examined {
        linked.dependencies.extend(examination.dependencies);
        linked.analyzed.extend(examination.analyzed);
    }
    linked
}

/// The names the modules declared in one file bear, from the notes the parse left of it.
#[must_use]
pub(crate) fn declared_in(note: &languages::Bytes) -> Option<Vec<Segment>> {
    crate::domain::fact_notes::service::decode(note, &|_| Naming::Unknown).map(|each| {
        each.declared
            .into_iter()
            .map(|each| each.name)
            .collect::<Vec<_>>()
    })
}

/// The facts of one file, from the notes the parse left of it: a path that starts with a name in `names` names a module.
#[must_use]
pub(crate) fn facts_in(note: &languages::Bytes, names: &BTreeSet<Segment>) -> Option<Facts> {
    crate::domain::fact_notes::service::decode(note, &|first| {
        if names.contains(first) {
            Naming::Known
        } else {
            Naming::Unknown
        }
    })
}

/// The dependencies of one file and whether it was analyzed. `files` are in the order of their places, and `facts` are the facts of
/// each source file, by its place.
#[must_use]
pub(crate) fn examine(
    packages: &Index,
    trees: &BTreeMap<RepoPath, Vec<Tree>>,
    kept: &BTreeSet<RepoPath>,
    files: &[FilePath],
    facts: &[Option<Facts>],
    place: Place,
    file: &FilePath,
) -> Examination {
    let nothing = Examination::default();
    let file = RepoPath(file.0.clone());
    if !file.ends_with(SOURCE) || !kept.contains(&file) {
        return nothing;
    }
    let Some(own) = package_of(packages, &file) else {
        return nothing;
    };
    let Some(known) = facts
        .get(place.0)
        .and_then(Option::as_ref)
        .filter(|known| known.clean == Cleanliness::Clean)
    else {
        return nothing;
    };
    let memberships: Vec<(&Tree, &ModulePath)> = trees
        .get(own)
        .into_iter()
        .flatten()
        .flat_map(|each| {
            each.files
                .get(&file)
                .into_iter()
                .flatten()
                .map(move |module| (each, module))
        })
        .collect();
    if memberships.is_empty() {
        return nothing;
    }
    let mut edges: BTreeSet<Edge<'_>> = BTreeSet::new();
    for (each, module) in memberships {
        for written in &known.written {
            let mut current = module.0.clone();
            current.extend(written.enclosing.iter().cloned());
            let Some(targets) = resolve(each, &current, &written.segments) else {
                continue;
            };
            for target in targets.iter().filter(|target| *target != &file) {
                edges.insert(Edge {
                    target,
                    start: written.start,
                    end: written.end,
                });
            }
        }
    }
    let dependencies = edges
        .into_iter()
        .filter_map(|Edge { target, start, end }| {
            place_of(files, target).map(|target| Link {
                source: place,
                target,
                start,
                end,
            })
        })
        .collect();
    Examination {
        dependencies,
        analyzed: vec![place],
    }
}

/// How the files of the project refer to one another, as the `project` contract worked it out when it resolved the manifests: nothing
/// is linked when the project has no manifest of the language or no relation of the links was asked for.
#[must_use]
pub(crate) fn linked_in(input: &languages::ManifestInput<'_, Linked>) -> Option<Linked> {
    Some(input.derived.clone())
}
