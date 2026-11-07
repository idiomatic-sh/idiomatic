//! The `links` service: free functions named by verb over the domain's types, with no IO and no ports. It links each package a Go file
//! imports to the files of the repository that package is, as `go build` finds them in module mode: an import path that is the path of
//! the file's module, the one whose `go.mod` is the nearest above it, or starts with it, is the directory under the module's that the
//! rest of the path names, and the package is every Go file of that directory but its tests. Any other path names the standard library
//! or another module, which `package_dependency` answers.

use ryo::languages::{self, FileId, FilePath};

use super::constants::{GO, ROOT, SEPARATOR, TEST, UNREAD_CAUSE, UNRESOLVED_CAUSE};
use super::types::{
    Doubt, FileIndex, ImportOutcome, Layout, PackageImportLink, Linked, Owner, Rest, Root, UnlinkedFile,
};
use crate::domain::fact_notes::types::{ImportPath, Note};
use crate::domain::modules::constants::MODULE_FILE;
use crate::domain::modules::service::directory_of;
use crate::domain::modules::types::{Cause, Directory, Located, Manifest, Modules};

/// The number of the file at `place` among the files of the run.
#[must_use]
pub(crate) fn file_id(place: FileIndex) -> FileId {
    FileId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// Where the import of a link is written.
#[must_use]
pub(crate) fn link_span(link: &PackageImportLink) -> languages::Span {
    languages::Span {
        start: link.start,
        end: link.end,
    }
}

/// Why a file whose imports are not all linked makes the relation read in part.
#[must_use]
pub(crate) fn cause_of(unresolved: &UnlinkedFile) -> Cause {
    match &unresolved.doubt {
        Doubt::Path(path) => Cause(format!("`{}` {UNRESOLVED_CAUSE}", path.0).into()),
        Doubt::Module(manifest) => Cause(format!("{} {UNREAD_CAUSE}", manifest.0).into()),
    }
}

/// What linking was handed to the readers of a run: all of it.
#[must_use]
#[expect(
    clippy::unnecessary_wraps,
    reason = "a `model` statement of `reads!` takes the answer of a function that may have no model for the project; the links always have one, which is still answered as that Option"
)]
pub(crate) fn linked_in(input: &languages::ManifestInput<'_, Linked>) -> Option<Linked> {
    Some(input.derived.clone())
}

/// The modules the manifests declared, handed to the readers of a run.
#[must_use]
pub(crate) fn modules_in<'a>(input: &languages::ManifestInput<'a, Linked>) -> &'a Modules {
    &input.derived.modules
}

/// The directory the file at `path` is in.
fn directory(path: &FilePath) -> Directory<'_> {
    Directory(
        path.0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(directory, _)| directory),
    )
}

/// The directory above `directory`; none above the root.
fn above(directory: Directory<'_>) -> Option<Directory<'_>> {
    if directory.0.is_empty() {
        return None;
    }
    Some(
        directory
            .0
            .rsplit_once(SEPARATOR)
            .map_or(Directory(ROOT), |(parent, _)| Directory(parent)),
    )
}

/// The repository the files of the run and the manifests make: the Go files of each directory but its tests, and the directory of each
/// `go.mod`, read or refused.
#[must_use]
pub(crate) fn layout_of<'a>(files: &'a [FilePath], modules: &'a Modules) -> Layout<'a> {
    let mut layout = Layout::default();
    for (place, file) in files.iter().enumerate() {
        if file.0.ends_with(GO) && !file.0.ends_with(TEST) {
            layout
                .packages
                .entry(Located(directory(file).0.to_owned()))
                .or_insert_with(Vec::new)
                .push(file_id(FileIndex(place)));
        }
    }
    for manifest in &modules.manifests {
        if manifest.declared.module.is_some() {
            // A directory holds one `go.mod`, so no root of another manifest is replaced.
            let _: Option<Root<'_>> = layout.roots.insert(
                Located(directory_of(&manifest.manifest).0.to_owned()),
                Root::Module(manifest),
            );
        }
    }
    for refusal in &modules.refused {
        let file = refusal.manifest.0.rsplit(SEPARATOR).next();
        if file == Some(MODULE_FILE) {
            // A directory holds one `go.mod`, and a refused one declared no module above, so no root is replaced.
            let _: Option<Root<'_>> = layout.roots.insert(
                Located(directory_of(&refusal.manifest).0.to_owned()),
                Root::Refused(&refusal.manifest),
            );
        }
    }
    layout
}

/// The module whose directory is the nearest at or above `directory`.
#[must_use]
pub(crate) fn owner_of<'a>(layout: &Layout<'a>, directory: Directory<'_>) -> Owner<'a> {
    let mut at = Some(directory);
    while let Some(each) = at {
        match layout.roots.get(&Located(each.0.to_owned())) {
            Some(Root::Module(manifest)) => return Owner::Module(manifest),
            Some(Root::Refused(manifest)) => return Owner::Refused(manifest),
            None => at = above(each),
        }
    }
    Owner::Nothing
}

/// The directory the rest of an import path names under the module's directory `root`.
fn joined(root: Directory<'_>, rest: Rest<'_>) -> Located {
    match (root.0.is_empty(), rest.0.is_empty()) {
        (_, true) => Located(root.0.to_owned()),
        (true, false) => Located(rest.0.to_owned()),
        (false, false) => Located(format!("{}{SEPARATOR}{}", root.0, rest.0)),
    }
}

/// Where `path`, imported by a file of `module`, leads: when it is the module's path or starts with it, the files of the package of
/// the module the directory it names is; a directory of a module nested in it is another module's, outside.
#[must_use]
pub(crate) fn outcome<'a>(
    layout: &'a Layout<'_>,
    module: &Manifest,
    path: &ImportPath,
) -> ImportOutcome<'a> {
    let Some(name) = module.declared.module.as_ref() else {
        return ImportOutcome::Outside;
    };
    let rest = if *path.0 == *name.0 {
        Some(ROOT)
    } else {
        path.0
            .strip_prefix(&*name.0)
            .and_then(|rest| rest.strip_prefix(SEPARATOR))
    };
    let Some(rest) = rest else {
        return ImportOutcome::Outside;
    };
    let target = joined(directory_of(&module.manifest), Rest(rest));
    let own = matches!(
        owner_of(layout, Directory(&target.0)),
        Owner::Module(found) if found.manifest == module.manifest
    );
    if !own {
        return ImportOutcome::Outside;
    }
    layout
        .packages
        .get(&target)
        .map_or(ImportOutcome::Unresolved, |files| {
            ImportOutcome::Files(files)
        })
}

/// How the files of the run link to one another, from the notes the parse left of each, in the order of `files`, none for a file it
/// left no note of, and the modules the manifests declared. A file in a module whose `go.mod` was refused is not examined: which of its
/// imports are its module's is unknown.
#[must_use]
pub(crate) fn linked(files: &[FilePath], notes: &[Option<Note>], modules: Modules) -> Linked {
    let mut found = Linked::default();
    {
        let layout = layout_of(files, &modules);
        for (place, (file, note)) in files.iter().zip(notes).enumerate() {
            let Some(note) = note else {
                continue;
            };
            let source = file_id(FileIndex(place));
            let module = match owner_of(&layout, directory(file)) {
                Owner::Module(module) => Some(module),
                Owner::Refused(manifest) => {
                    found.unresolved.push(UnlinkedFile {
                        file: source,
                        doubt: Doubt::Module(manifest.clone()),
                    });
                    continue;
                }
                Owner::Nothing => None,
            };
            found.analyzed.push(source);
            let Some(module) = module else {
                continue;
            };
            for import in &note.imports {
                match outcome(&layout, module, &import.path) {
                    ImportOutcome::Files(targets) => {
                        found.dependencies.extend(
                            targets
                                .iter()
                                .filter(|target| **target != source)
                                .map(|target| PackageImportLink {
                                    source,
                                    target: *target,
                                    start: import.start,
                                    end: import.end,
                                    written: import.text.clone(),
                                }),
                        );
                    }
                    ImportOutcome::Unresolved => found.unresolved.push(UnlinkedFile {
                        file: source,
                        doubt: Doubt::Path(import.path.clone()),
                    }),
                    ImportOutcome::Outside => {}
                }
            }
        }
    }
    found
        .dependencies
        .sort_by_key(|link| (link.source, link.start, link.target));
    found.modules = modules;
    found
}
