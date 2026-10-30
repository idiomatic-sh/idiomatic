//! The `links` service: free functions named by verb that link the files of a repository: the module tree of a package is drawn, and
//! the paths each file writes are resolved against the trees, with no IO and no ports. How the work spreads over the machine is the use
//! case's: each function here works on one package, one file or one note.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Drawn, Examination, Link, Linked, Packaged, Placed, Run, UnplacedLink};
use crate::domain::fact_notes::service::facts_of;
use crate::domain::fact_notes::types::{Naming, Note};
use crate::domain::module_tree::constants::SOURCE;
use crate::domain::module_tree::service::{resolve, tree};
use crate::domain::module_tree::types::{
    Cleanliness, Facts, ModulePath, ModuleTree, Segment, Unfollowed,
};
use crate::domain::package_index::constants::MANIFEST;
use crate::domain::package_index::service::{index, kept_files, package_of};
use crate::domain::package_index::types::{Index, ListIndex, Package};
use crate::domain::paths::service::lexical_join;
use crate::domain::paths::types::Relative;
use crate::domain::paths::types::RepoPath;
use ryo::languages::{self, FilePath};

/// The place of a file among the repository's, as the number the rows hold it by.
#[must_use]
pub(crate) fn file_id(place: ListIndex) -> languages::FileId {
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
    let packages = input.derived.packages.as_ref()?;
    let files = &input.request.files;
    let kept = kept_files(files, &packages.outputs);
    let index = index(&kept, &packages.roots);
    let placed: Vec<Placed> = files
        .iter()
        .enumerate()
        .filter(|(_, file)| kept.contains(&RepoPath(file.0.clone())))
        .filter_map(|(place, file)| {
            let directory = package_of(&index, &RepoPath(file.0.clone()))?;
            Some(Placed {
                place: ListIndex(place),
                package: lexical_join(directory, &Relative(MANIFEST.into())),
            })
        })
        .collect();
    Some(Packaged {
        files: placed,
        listed: crate::domain::packages::service::numbered(files, packages),
        refused: packages.refused.clone(),
    })
}

/// The place of `path` among `files`, which are in the order of their places and in the order of their paths.
#[must_use]
pub(crate) fn place_of(files: &[FilePath], path: &RepoPath) -> Option<ListIndex> {
    match files.binary_search_by(|each| (*each.0).cmp(&*path.0)) {
        Ok(place) => Some(ListIndex(place)),
        // The search answers where the path would go: the run does not list it.
        Err(_) => None,
    }
}

/// The module trees of the crate roots of one package, drawn from the facts of its files: the package's directory and its trees.
/// `facts` are the facts of each source file, by its place.
#[must_use]
pub(crate) fn draw(run: Run<'_>, package: &Package) -> Drawn {
    let facts_of = |path: &RepoPath| {
        place_of(run.files, path)
            .and_then(|place| run.facts.get(place.0))
            .and_then(Option::as_ref)
    };
    Drawn {
        directory: package.directory.clone(),
        trees: package
            .roots
            .iter()
            .map(|root| tree(run.kept, &package.directory, root, &facts_of))
            .collect::<Vec<ModuleTree>>(),
    }
}

/// The modules that could not be followed in any of the trees, each once.
#[must_use]
pub(crate) fn gaps_in(trees: &BTreeMap<RepoPath, Vec<ModuleTree>>) -> Vec<Unfollowed> {
    let mut gaps: Vec<Unfollowed> = trees
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
pub(crate) fn merged(run: Run<'_>, gaps: Vec<Unfollowed>, examined: Vec<Examination>) -> Linked {
    let gap_files = gaps.iter().map(|gap| gap_file(run.files, gap)).collect();
    let mut linked = Linked {
        gaps,
        gap_files,
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
pub(crate) fn declared_in(note: &Note<'_>) -> Vec<Segment> {
    note.declared
        .iter()
        .map(|each| Segment((*each.name.0).into()))
        .collect()
}

/// The facts of one file, from the notes the parse left of it: a path that starts with a name in `names` names a module.
#[must_use]
pub(crate) fn facts_in(note: Note<'_>, names: &BTreeSet<Segment>) -> Facts {
    facts_of(note, &|first| {
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
    run: Run<'_>,
    packages: &Index,
    trees: &BTreeMap<RepoPath, Vec<ModuleTree>>,
    place: ListIndex,
    file: &FilePath,
) -> Examination {
    let nothing = Examination::default();
    let file = RepoPath(file.0.clone());
    if !file.ends_with(SOURCE) || !run.kept.contains(&file) {
        return nothing;
    }
    let Some(own) = package_of(packages, &file) else {
        return nothing;
    };
    let memberships: Vec<(&ModuleTree, &ModulePath)> = trees
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
    // A file no crate root reaches is known not to be analyzed: it has no row, and no doubt.
    if memberships.is_empty() {
        return nothing;
    }
    let Some(known) = run
        .facts
        .get(place.0)
        .and_then(Option::as_ref)
        .filter(|known| known.clean == Cleanliness::Clean)
    else {
        return nothing;
    };
    let mut edges: BTreeSet<UnplacedLink<'_>> = BTreeSet::new();
    for (each, module) in memberships {
        for written in &known.written {
            let mut current = module.0.clone();
            current.extend(written.enclosing.iter().cloned());
            let Some(targets) = resolve(each, &current, &written.segments) else {
                continue;
            };
            for target in targets.iter().filter(|target| *target != &file) {
                edges.extend([UnplacedLink {
                    target,
                    start: written.start,
                    end: written.end,
                    written: &written.text,
                }]);
            }
        }
    }
    let dependencies = edges
        .into_iter()
        .filter_map(
            |UnplacedLink {
                 target,
                 start,
                 end,
                 written,
             }| {
                place_of(run.files, target).map(|target| Link {
                    source: place,
                    target,
                    start,
                    end,
                    written: written.clone(),
                })
            },
        )
        .collect();
    Examination {
        dependencies,
        analyzed: vec![place],
    }
}

/// The file a gap is about, among the `files` of the run, or none of them when the file that declares the module is not listed.
#[must_use]
pub(crate) fn gap_file(files: &[FilePath], gap: &Unfollowed) -> languages::FileId {
    let (Unfollowed::Files { file, .. } | Unfollowed::Loading { file, .. }) = gap;
    place_of(files, file).map_or(languages::FileId(u32::MAX), file_id)
}

/// How the files of the project refer to one another, as the `project` contract worked it out when it resolved the manifests: nothing
/// is linked when the project has no manifest of the language or no relation of the links was asked for.
#[must_use]
#[expect(
    clippy::unnecessary_wraps,
    reason = "a `model` statement of `reads!` takes the answer of a function that may have no model for the project; the links always have one, which is still answered as that Option"
)]
pub(crate) fn linked_in(input: &languages::ManifestInput<'_, Linked>) -> Option<Linked> {
    Some(input.derived.clone())
}
