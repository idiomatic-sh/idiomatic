//! The `links` service: free functions named by verb that link the files of a repository: the module tree of a package is drawn, and
//! the paths each file writes are resolved against the trees, with no IO and no ports. How the work spreads over the machine is the use
//! case's: each function here works on one package, one file or one note.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    Drawn, Examination, ExternCrate, Externs, Linked, Packaged, Placed, Run, SourceDependencyLink,
    TargetRole, UnplacedLink,
};
use crate::domain::fact_notes::service::facts_of;
use crate::domain::fact_notes::types::{Naming, Note};
use crate::domain::module_tree::constants::{CRATE, SOURCE};
use crate::domain::module_tree::service::{resolve, tree};
use crate::domain::module_tree::types::{
    Cleanliness, Facts, ModulePath, ModuleTree, Segment, Unfollowed, Written,
};
use crate::domain::package_index::constants::MANIFEST;
use crate::domain::package_index::service::{index, kept_files, package_of};
use crate::domain::package_index::types::{Index, ListIndex, Package};
use crate::domain::packages::types::Packages;
use crate::domain::paths::constants::SEPARATOR;
use crate::domain::paths::service::{directory_of, lexical_join};
use crate::domain::paths::types::Relative;
use crate::domain::paths::types::RepoPath;
use crate::domain::targets::constants::{BENCHES, BUILD, EXAMPLES, TESTS};
use crate::domain::targets::service::crate_name;
use crate::domain::targets::types::PackageName;
use ryo::languages::{self, FilePath};

/// The place of a file among the repository's, as the number the rows hold it by.
#[must_use]
pub(crate) fn file_id(place: ListIndex) -> languages::FileId {
    languages::FileId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// Where the path of a link is written.
#[must_use]
pub(crate) fn link_span(link: &SourceDependencyLink) -> languages::Span {
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

/// The libraries of the repository each package depends on, by the name its code writes for each: the binding when the manifest
/// renames the package, and the library's own name when it does not, each with the kind it is depended on as. A package's
/// dependency on itself names nothing new.
#[must_use]
pub(crate) fn externs_in(packages: &Packages) -> Externs {
    let mut externs = Externs::default();
    for dependency in &packages.dependencies {
        let Some(target) = dependency
            .target
            .as_ref()
            .filter(|target| **target != dependency.source)
        else {
            continue;
        };
        let Some(library) = packages.libraries.get(target) else {
            continue;
        };
        let name = if dependency.binding.0 == dependency.package.0 {
            library.name.0.clone()
        } else {
            crate_name(&PackageName(dependency.binding.0.clone())).0
        };
        externs
            .by_package
            .entry(directory_of(&dependency.source))
            .or_default()
            .entry(Segment(name))
            .or_default()
            .push(ExternCrate {
                directory: directory_of(target),
                root: library.root.clone(),
                kind: dependency.kind,
            });
    }
    externs
}

/// The role of the crate root `tree` is drawn from: its root file, relative to its package's directory, under `tests`, `benches` or
/// `examples`, or the package's `build.rs`.
fn role_of(tree: &ModuleTree) -> TargetRole {
    let Some(root) = tree
        .modules
        .get(&ModulePath::default())
        .and_then(|roots| roots.first())
    else {
        return TargetRole::Ordinary;
    };
    let relative = if tree.package.is_empty() {
        Some(&*root.0)
    } else {
        root.strip_prefix(&*tree.package.0)
            .and_then(|rest| rest.strip_prefix(SEPARATOR))
    };
    match relative {
        Some(relative) if relative == BUILD => TargetRole::Build,
        Some(relative)
            if [TESTS, BENCHES, EXAMPLES]
                .contains(&relative.split(SEPARATOR).next().unwrap_or_default()) =>
        {
            TargetRole::Test
        }
        _ => TargetRole::Ordinary,
    }
}

/// The files a path written in a crate root of `role` names in the library of another package of the repository its package
/// depends on: the path's first segment is the name the package's code writes for that library, depended on as a kind the root may
/// name, and the rest is resolved from the library's crate root.
fn resolve_extern<'a>(
    externs: Option<&BTreeMap<Segment, Vec<ExternCrate>>>,
    role: TargetRole,
    trees: &'a BTreeMap<RepoPath, Vec<ModuleTree>>,
    written: &[Segment],
) -> Option<&'a [RepoPath]> {
    let (first, rest) = written.split_first()?;
    // Cargo hands normal dependencies to every target but the build script, dev-dependencies to tests, benches and examples
    // besides, and build-dependencies to the build script alone.
    let library = externs?.get(first)?.iter().find(|each| {
        matches!(
            (role, each.kind),
            (
                TargetRole::Ordinary | TargetRole::Test,
                languages::DependencyKind::Normal
            ) | (TargetRole::Test, languages::DependencyKind::Dev)
                | (TargetRole::Build, languages::DependencyKind::Build)
        )
    })?;
    let tree = trees.get(&library.directory)?.iter().find(|tree| {
        tree.modules
            .get(&ModulePath::default())
            .is_some_and(|roots| roots.contains(&library.root))
    })?;
    let mut from_root = Vec::with_capacity(written.len());
    from_root.push(Segment(CRATE.into()));
    from_root.extend(rest.iter().cloned());
    resolve(tree, &[], &from_root)
}

/// The path `written` once its first name is replaced by what the `use` that binds it there names (`helper::f` after `use crate::util
/// as helper` is `crate::util::f`): a `use` binds where its scope holds the path with no inline module between them, and the one of the
/// innermost scope shadows the others. A path whose first name no other `use` binds there is as written.
fn through_uses<'w>(known: &'w Facts, written: &'w Written) -> std::borrow::Cow<'w, [Segment]> {
    let Some(first) = written.segments.first() else {
        return std::borrow::Cow::Borrowed(&written.segments);
    };
    let binding = known
        .written
        .iter()
        .filter(|each| {
            each.start != written.start
                && each.enclosing == written.enclosing
                && each
                    .scope
                    .is_none_or(|scope| scope.start <= written.start && written.end <= scope.end)
                && each.bound.as_ref() == Some(first)
        })
        .min_by_key(|each| {
            each.scope
                .map_or(u32::MAX, |scope| scope.end.0.saturating_sub(scope.start.0))
        });
    match binding {
        Some(binding) => std::borrow::Cow::Owned(
            binding
                .segments
                .iter()
                .chain(written.segments.iter().skip(1))
                .cloned()
                .collect(),
        ),
        None => std::borrow::Cow::Borrowed(&written.segments),
    }
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
/// each source file, by its place. A path resolves in the file's own crate first, then in the library of a package of the repository
/// its package depends on (`externs`).
#[must_use]
pub(crate) fn examine(
    run: Run<'_>,
    packages: &Index,
    trees: &BTreeMap<RepoPath, Vec<ModuleTree>>,
    externs: &Externs,
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
        let target_role = role_of(each);
        for written in &known.written {
            let mut current = module.0.clone();
            current.extend(written.enclosing.iter().cloned());
            // A module under `#[cfg(test)]`, or inside one, is built for the unit tests, with the dev-dependencies.
            let gated = (0..=current.len())
                .any(|length| each.tests.contains(&ModulePath(current[..length].to_vec())));
            let role = if target_role == TargetRole::Ordinary && gated {
                TargetRole::Test
            } else {
                target_role
            };
            let segments = through_uses(known, written);
            let Some(targets) = resolve(each, &current, &segments)
                .or_else(|| resolve_extern(externs.by_package.get(own), role, trees, &segments))
            else {
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
                place_of(run.files, target).map(|target| SourceDependencyLink {
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
