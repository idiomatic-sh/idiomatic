//! The `modules` service: free functions named by verb that find the modules a repository's manifests declare, say which require which and
//! which a workspace holds, and place each file in one, with no IO and no ports.

use ryo::languages::{self, Tree};

use super::constants::{
    ABOVE, CAUSE_SEPARATOR, EMPTY, HERE, JOINER, MODULE_FILE, ROOT, SEPARATOR, WORKSPACE_FILE,
};
use super::constants::{QUOTES, REQUIRE_VERB};
use super::types::{
    Cause, Diagnosed, Directory, FilePlacement, Located, Manifest, ManifestPath, Manifestness,
    Modules, Part, Relative, Replacement, Replacing, Wired, Wiring, WorkspaceMembership,
};
use super::types::{ModuleMapping, RequiredLine, Verb};
use std::borrow::Cow;

use crate::domain::extras::types::Extra;
use crate::domain::literals::service::unquoted;
use crate::domain::literals::types::{Unquoted, Written as LiteralText};
use crate::domain::syntax::service::{kind, parent, written};
use crate::domain::syntax::types::Kind;

/// Which manifest a path is: by the last part of it.
#[must_use]
pub(crate) fn manifestness_of(path: &languages::FilePath) -> Manifestness {
    match path.rsplit(SEPARATOR).next() {
        Some(MODULE_FILE) => Manifestness::Module,
        Some(WORKSPACE_FILE) => Manifestness::Workspace,
        Some(_) | None => Manifestness::Other,
    }
}

/// The directory a manifest is in.
#[must_use]
pub(crate) fn directory_of(manifest: &ManifestPath) -> Directory<'_> {
    Directory(
        manifest
            .0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(directory, _)| directory),
    )
}

/// The directory above `directory`; the root's is the root.
fn above(directory: Directory<'_>) -> Directory<'_> {
    Directory(
        directory
            .0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(parent, _)| parent),
    )
}

/// The directory `relative` names from `from`, as a path from the root of the repository: `./app` and `../util` are read as a shell
/// does; none for a path that leaves the repository.
#[must_use]
pub(crate) fn resolve(from: Directory<'_>, relative: Relative<'_>) -> Option<Located> {
    let mut parts: Vec<Part<'_>> = from
        .0
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .map(Part)
        .collect();
    for part in relative.0.split(SEPARATOR) {
        match part {
            EMPTY | HERE => {}
            ABOVE => {
                // A `..` drops the part above it; with none left, the path leaves the repository.
                let _: Part<'_> = parts.pop()?;
            }
            other => parts.push(Part(other)),
        }
    }
    Some(Located(
        parts
            .iter()
            .map(|part| part.0)
            .collect::<Vec<&str>>()
            .join(JOINER),
    ))
}

/// The place of the file `manifest` among the files of the run.
#[must_use]
pub(crate) fn file_of(
    files: &[languages::FilePath],
    manifest: &ManifestPath,
) -> Option<languages::FileId> {
    files
        .iter()
        .position(|file| *file.0 == *manifest.0)
        .map(|place| languages::FileId(u32::try_from(place).unwrap_or(u32::MAX)))
}

/// The manifests in the order of their paths.
#[must_use]
pub(crate) fn sorted(mut manifests: Vec<Manifest>) -> Vec<Manifest> {
    manifests.sort_by(|left, right| left.manifest.cmp(&right.manifest));
    manifests
}

/// Why the relation is read in part for each manifest the tool refused.
fn causes_of(modules: &Modules) -> Vec<Cause> {
    modules
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

/// The manifests the tool refused that are among the files of the run, each with what it said, in the order of their paths.
#[must_use]
pub(crate) fn diagnoses_in(files: &[languages::FilePath], modules: &Modules) -> Vec<Diagnosed> {
    modules
        .refused
        .iter()
        .filter_map(|refusal| {
            file_of(files, &refusal.manifest).map(|file| Diagnosed {
                file,
                manifest: manifestness_of(&languages::FilePath(refusal.manifest.0.clone())),
                reason: refusal.reason.clone(),
            })
        })
        .collect()
}

/// The module whose manifest is in `directory`.
fn module_in<'a>(modules: &'a Modules, directory: Directory<'_>) -> Option<&'a Manifest> {
    modules
        .manifests
        .iter()
        .find(|each| each.declared.module.is_some() && directory_of(&each.manifest) == directory)
}

/// The requirements of each module: on another module of the repository, found by its path or by the directory a `replace` sends it
/// to, or on a module outside it, whose row has no target. A requirement a `replace` sends to a directory comes from that path; any
/// other, the module path a proxy or its repository serves, comes from a registry.
#[must_use]
pub(crate) fn wirings_in(files: &[languages::FilePath], modules: &Modules) -> Wired<Wiring> {
    let mut rows = Vec::new();
    for manifest in modules
        .manifests
        .iter()
        .filter(|each| each.declared.module.is_some())
    {
        let Some(source) = file_of(files, &manifest.manifest) else {
            continue;
        };
        for requirement in &manifest.declared.requirements {
            // A `replace` that names a version applies to that version alone, and one that names none to every version: the one of
            // the version wins over the one of every version, wherever each is written, as the go command applies them.
            let of_module = |each: &&Replacement| each.from == requirement.path;
            let replacement = manifest
                .declared
                .replacements
                .iter()
                .filter(of_module)
                .find(|each| !each.version.0.is_empty() && each.version == requirement.version)
                .or_else(|| {
                    manifest
                        .declared
                        .replacements
                        .iter()
                        .filter(of_module)
                        .find(|each| each.version.0.is_empty())
                });
            let local = replacement.filter(|each| each.to.0.starts_with('.'));
            let replaced = local
                .and_then(|each| resolve(directory_of(&manifest.manifest), Relative(&each.to.0)))
                .and_then(|directory| module_in(modules, Directory(&directory.0)));
            let named = modules.manifests.iter().find(|each| {
                each.declared.module.as_ref() == Some(&requirement.path)
                    && each.manifest != manifest.manifest
            });
            let found = if local.is_some() { replaced } else { named };
            let target = match found {
                Some(target) => match file_of(files, &target.manifest) {
                    Some(target) => target,
                    None => continue,
                },
                None => languages::NO_FILE,
            };
            rows.push(Wiring {
                source,
                target,
                origin: if local.is_some() {
                    languages::DependencyOrigin::Path
                } else {
                    languages::DependencyOrigin::Registry
                },
                path: requirement.path.clone(),
                version: requirement.version.clone(),
                replacement: replacement.cloned(),
            });
        }
    }
    Wired {
        rows,
        refused: causes_of(modules),
    }
}

/// What a `replace` puts in the place of the module, as `go.mod` writes it after its arrow: a module path and its version
/// (`example.com/fork v1.2.0`), or a directory (`../util`); empty when no `replace` applies.
#[must_use]
pub(crate) fn replacement_text(replacement: Option<&Replacement>) -> Replacing {
    Replacing(replacement.map_or_else(String::new, |each| {
        if each.to_version.0.is_empty() {
            each.to.0.to_string()
        } else {
            format!("{} {}", each.to.0, each.to_version.0)
        }
    }))
}

/// The modules each workspace uses.
#[must_use]
pub(crate) fn holdings_in(
    files: &[languages::FilePath],
    modules: &Modules,
) -> Wired<WorkspaceMembership> {
    let mut rows = Vec::new();
    for workspace in modules
        .manifests
        .iter()
        .filter(|each| each.declared.module.is_none())
    {
        for used in &workspace.declared.uses {
            if let Some(member) = resolve(directory_of(&workspace.manifest), Relative(&used.0))
                .and_then(|directory| module_in(modules, Directory(&directory.0)))
                && let (Some(workspace), Some(module)) = (
                    file_of(files, &workspace.manifest),
                    file_of(files, &member.manifest),
                )
            {
                rows.push(WorkspaceMembership { workspace, module });
            }
        }
    }
    Wired {
        rows,
        refused: causes_of(modules),
    }
}

/// The module each file of the run belongs to: the one whose directory is the nearest above it.
#[must_use]
pub(crate) fn placed_in(files: &[languages::FilePath], modules: &Modules) -> Wired<FilePlacement> {
    let mut rows = Vec::new();
    for (place, file) in files.iter().enumerate() {
        let path = ManifestPath(file.0.clone());
        let mut directory = directory_of(&path);
        let owner = loop {
            if let Some(module) = module_in(modules, directory) {
                break Some(module);
            }
            if directory.0.is_empty() {
                break None;
            }
            directory = above(directory);
        };
        if let Some(module) = owner
            && let Some(manifest) = file_of(files, &module.manifest)
        {
            rows.push(FilePlacement {
                path: languages::FileId(u32::try_from(place).unwrap_or(u32::MAX)),
                module: manifest,
            });
        }
    }
    Wired {
        rows,
        refused: causes_of(modules),
    }
}

/// The words of the `go.mod` element at `at`, in order: its children that are words.
fn words<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    languages::children(tree.elements, at)
        .filter(move |each| kind(tree, *each) == Kind::Extra(Extra::ModWord))
}

/// The text of the `go.mod` word at `at`, as Go reads it: a quoted word without its quotes and with its escapes decoded
/// (`"example.com/\u007a"` is `example.com/z`), as `modfile` reads one with `strconv.Unquote`; a word that is no string, or a
/// string that holds no text, as it is written. Its spelling is `written`.
#[must_use]
pub(crate) fn word_text<'a>(tree: &Tree<'a>, at: languages::Position) -> Unquoted<'a> {
    let spelled = written(tree, at).0;
    if spelled.starts_with(QUOTES) {
        unquoted(LiteralText(spelled)).unwrap_or(Unquoted(Cow::Borrowed(spelled)))
    } else {
        Unquoted(Cow::Borrowed(spelled))
    }
}

/// Whether the `go.mod` element at `at` starts with the verb that requires a module.
fn verb_of(tree: &Tree<'_>, at: languages::Position) -> Verb {
    if words(tree, at)
        .next()
        .is_some_and(|verb| word_text(tree, verb).0 == REQUIRE_VERB)
    {
        Verb::Require
    } else {
        Verb::Other
    }
}

/// The requirement the `go.mod` line at `at` writes, when it writes one: a line of a `require` block, or a `require` line of its own.
#[must_use]
pub(crate) fn required_at(tree: &Tree<'_>, at: languages::Position) -> Option<RequiredLine> {
    if kind(tree, at) != Kind::Extra(Extra::ModLine) {
        return None;
    }
    let owner = parent(tree, at);
    let in_block = kind(tree, owner) == Kind::Extra(Extra::ModLineBlock);
    if in_block && verb_of(tree, owner) != Verb::Require {
        return None;
    }
    if !in_block && verb_of(tree, at) != Verb::Require {
        return None;
    }
    // A line of a block writes the module first; a line of its own writes the verb first.
    let skipped = usize::from(!in_block);
    let mut written = words(tree, at).skip(skipped);
    let path = written.next()?;
    let version = written.next();
    let earlier = languages::children(tree.elements, owner)
        .take_while(|each| *each != at)
        .filter(|each| {
            kind(tree, *each) == Kind::Extra(Extra::ModLine)
                && (in_block || verb_of(tree, *each) == Verb::Require)
        })
        .count();
    Some(RequiredLine {
        path,
        version,
        owner,
        ordinal: languages::Integer(i64::try_from(earlier).unwrap_or(i64::MAX)),
    })
}

/// The mapping the `go.mod` element at `at` is, when it is one: the file, which holds the `require` lines of its own, and each
/// `require` block, which holds its lines. The file is implied by its lines and holds itself; a block is opened by its `require (`
/// and is held by the file.
#[must_use]
pub(crate) fn mapping_of(tree: &Tree<'_>, at: languages::Position) -> Option<ModuleMapping> {
    let found = kind(tree, at);
    if found == Kind::Extra(Extra::ModFile) {
        Some(ModuleMapping {
            kind: languages::MappingKind::Implicit,
            owner: at,
        })
    } else if found == Kind::Extra(Extra::ModLineBlock) && verb_of(tree, at) == Verb::Require {
        Some(ModuleMapping {
            kind: languages::MappingKind::Standard,
            owner: parent(tree, at),
        })
    } else {
        None
    }
}
