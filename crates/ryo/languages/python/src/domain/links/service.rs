//! The `links` service: free functions named by verb over the domain's types, with no IO and no ports. It links each import a Python
//! file writes to the file of the module it names, as Python's import system finds it: an absolute import under the directories of the
//! repository's packages and their `src` directories, a relative one from the file's own package; a module is its `__init__.py`, its
//! `.py`, or failing those their stubs, and a directory of Python files without either is a namespace package. `from m import n` names
//! the module `m.n` when there is one, and `m` otherwise.

use std::collections::BTreeSet;

use ryo::languages::{self, FileId, FilePath};

use super::constants::{
    DOT, MODULE, PACKAGE, ROOT, SEPARATOR, SOURCE_LAYOUT, STUB, UNRESOLVED_CAUSE,
};
use super::types::{
    Dotted, FileIndex, FilePlace, Found, ImportOutcome, Link, Linked, Listing, UnlinkedImport,
    WrittenImport,
};
use crate::domain::fact_notes::types::{Form, ImportNote, Note};
use crate::domain::language::service::reads;
use crate::domain::packages::service::directory_of;
use crate::domain::packages::types::{Cause, PackageLocation, Packages};
use crate::domain::patterns::types::Directory;

/// The number of the file at `place` among the files of the run.
#[must_use]
pub(crate) fn file_id(place: FileIndex) -> FileId {
    FileId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// Where the import of a link is written.
#[must_use]
pub(crate) fn link_span(link: &Link) -> languages::Span {
    languages::Span {
        start: link.start,
        end: link.end,
    }
}

/// Why an unresolved import makes the relation read in part.
#[must_use]
pub(crate) fn cause_of(unresolved: &UnlinkedImport) -> Cause {
    Cause(format!("`{}` {UNRESOLVED_CAUSE}", unresolved.written.0).into())
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

/// The packages the manifests declared, handed to the readers of a run.
#[must_use]
pub(crate) fn packages_in<'a>(input: &languages::ManifestInput<'a, Linked>) -> &'a Packages {
    &input.derived.packages
}

/// The path `part`, its parts separated by `/`, under `directory`: `part` alone under the root.
fn joined(directory: Directory<'_>, part: FilePlace<'_>) -> PackageLocation {
    if directory.0.is_empty() {
        PackageLocation(part.0.into())
    } else {
        PackageLocation(format!("{}{SEPARATOR}{}", directory.0, part.0).into())
    }
}

/// The directory above `directory`; nothing above the root.
fn above(directory: Directory<'_>) -> Option<Directory<'_>> {
    if directory.0.is_empty() {
        return None;
    }
    Some(Directory(
        directory
            .0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(above, _)| above),
    ))
}

/// The directory of the file at `path`; the root for a file at the root.
fn directory_of_file(path: FilePlace<'_>) -> Directory<'_> {
    Directory(
        path.0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(directory, _)| directory),
    )
}

/// Every directory that holds a Python file of `files`, at any depth, the root included.
#[must_use]
pub(crate) fn python_directories(files: &[FilePath]) -> BTreeSet<Directory<'_>> {
    let mut directories = BTreeSet::new();
    for file in files
        .iter()
        .filter(|file| reads(languages::SourcePath(&file.0)) == languages::Recognition::Reads)
    {
        let mut directory = Some(directory_of_file(FilePlace(&file.0)));
        while let Some(each) = directory {
            if !directories.insert(each) {
                break;
            }
            directory = above(each);
        }
    }
    directories
}

/// The number of the file at `path`, if the run lists it.
fn listed(listing: Listing<'_>, path: FilePlace<'_>) -> Option<FileId> {
    match listing.files.binary_search_by(|each| (*each.0).cmp(path.0)) {
        Ok(place) => Some(file_id(FileIndex(place))),
        // The search answers where the path would go: the run does not list it.
        Err(_) => None,
    }
}

/// What the dotted path `path` names under `directory`: the module's file, a namespace package, or nothing; the directory itself for
/// an empty path.
#[must_use]
pub(crate) fn module_at(listing: Listing<'_>, directory: Directory<'_>, path: Dotted<'_>) -> Found {
    let stem = if path.0.is_empty() {
        PackageLocation(directory.0.into())
    } else {
        joined(
            directory,
            FilePlace(&path.0.replace(DOT, &SEPARATOR.to_string())),
        )
    };
    let package = joined(Directory(&stem.0), FilePlace(PACKAGE));
    // An empty path is the directory itself: only its `__init__` can be it.
    let module = (!path.0.is_empty()).then_some(&stem);
    let candidates = [
        Some(format!("{}{MODULE}", package.0)),
        module.map(|each| format!("{}{MODULE}", each.0)),
        Some(format!("{}{STUB}", package.0)),
        module.map(|each| format!("{}{STUB}", each.0)),
    ];
    for candidate in candidates.iter().flatten() {
        if let Some(file) = listed(listing, FilePlace(candidate)) {
            return Found::File(file);
        }
    }
    if listing.directories.contains(&Directory(&stem.0)) {
        Found::Namespace
    } else {
        Found::Nothing
    }
}

/// What `import` names under `directory`: the module its path names, or, for `from m import n` with no module `m.n`, the module `m`.
fn import_at(listing: Listing<'_>, directory: Directory<'_>, import: &ImportNote) -> Found {
    let found = module_at(listing, directory, Dotted(&import.path.0));
    if found != Found::Nothing || import.form != Form::Name {
        return found;
    }
    let from = import
        .path
        .0
        .rsplit_once(DOT)
        .map_or(ROOT, |(from, _)| from);
    match module_at(listing, directory, Dotted(from)) {
        Found::File(file) => Found::File(file),
        Found::Namespace | Found::Nothing => Found::Nothing,
    }
}

/// Where `import`, written in the file at `path`, leads: a relative import from the file's package, an absolute one under the
/// directories `roots`, in order.
#[must_use]
pub(crate) fn outcome(
    listing: Listing<'_>,
    roots: &[PackageLocation],
    path: FilePlace<'_>,
    import: &ImportNote,
) -> ImportOutcome {
    if import.level.0 > 0 {
        let mut directory = Some(directory_of_file(path));
        for _ in 1..import.level.0 {
            directory = directory.and_then(above);
        }
        return match directory.map(|each| import_at(listing, each, import)) {
            Some(Found::File(file)) => ImportOutcome::File(file),
            Some(Found::Namespace) => ImportOutcome::Namespace,
            Some(Found::Nothing) | None => ImportOutcome::Unresolved,
        };
    }
    let mut namespace = false;
    for root in roots {
        match import_at(listing, Directory(&root.0), import) {
            Found::File(file) => return ImportOutcome::File(file),
            Found::Namespace => namespace = true,
            Found::Nothing => {}
        }
    }
    if namespace {
        return ImportOutcome::Namespace;
    }
    let first = Dotted(import.path.0.split(DOT).next().unwrap_or_default());
    if roots
        .iter()
        .any(|root| module_at(listing, Directory(&root.0), first) != Found::Nothing)
    {
        ImportOutcome::Unresolved
    } else {
        ImportOutcome::Outside
    }
}

/// The directories absolute imports are found under, for the file at `path`: the directory of each package's manifest and its `src`
/// directory when it holds Python files, those of the file's own package first.
#[must_use]
pub(crate) fn roots_for(
    listing: Listing<'_>,
    packages: &[PackageLocation],
    path: FilePlace<'_>,
) -> Vec<PackageLocation> {
    let own = packages
        .iter()
        .filter(|directory| {
            directory.0.is_empty() || path.0.starts_with(&format!("{}{SEPARATOR}", directory.0))
        })
        .max_by_key(|directory| directory.0.len());
    let mut roots = Vec::new();
    for directory in own
        .into_iter()
        .chain(packages.iter().filter(|each| Some(*each) != own))
    {
        roots.push(directory.clone());
        let source = joined(Directory(&directory.0), FilePlace(SOURCE_LAYOUT));
        if listing.directories.contains(&Directory(&source.0)) {
            roots.push(source);
        }
    }
    roots
}

/// The directories of the packages the manifests declared, each once, in order; the root alone when they declare none.
#[must_use]
pub(crate) fn package_directories(packages: &Packages) -> Vec<PackageLocation> {
    let mut directories: Vec<PackageLocation> = packages
        .packages
        .iter()
        .map(|package| PackageLocation(directory_of(&package.manifest).0.into()))
        .collect();
    directories.sort_by(|left, right| left.0.cmp(&right.0));
    directories.dedup();
    if directories.is_empty() {
        directories.push(PackageLocation(ROOT.into()));
    }
    directories
}

/// The import as it is written, its dots kept.
fn written_import(import: &ImportNote) -> WrittenImport {
    let dots = usize::try_from(import.level.0).unwrap_or(usize::MAX);
    WrittenImport(format!("{}{}", DOT.to_string().repeat(dots), import.path.0).into())
}

/// How the Python files of the run link to one another, from the notes the parse left of each, in the order of `files`, none for a file
/// it left none of, and the packages the manifests declared.
#[must_use]
pub(crate) fn linked(files: &[FilePath], notes: &[Option<Note>], packages: Packages) -> Linked {
    let directories = python_directories(files);
    let listing = Listing {
        files,
        directories: &directories,
    };
    let directories_of_packages = package_directories(&packages);
    let mut found = Linked::default();
    for (place, (file, note)) in files.iter().zip(notes).enumerate() {
        let Some(note) = note else {
            continue;
        };
        let source = file_id(FileIndex(place));
        found.analyzed.push(source);
        let roots = roots_for(listing, &directories_of_packages, FilePlace(&file.0));
        for import in &note.imports {
            match outcome(listing, &roots, FilePlace(&file.0), import) {
                ImportOutcome::File(target) if target != source => found.dependencies.push(Link {
                    source,
                    target,
                    start: import.start,
                    end: import.end,
                    written: import.text.clone(),
                }),
                ImportOutcome::Unresolved => found.unresolved.push(UnlinkedImport {
                    file: source,
                    written: written_import(import),
                }),
                ImportOutcome::File(_) | ImportOutcome::Namespace | ImportOutcome::Outside => {}
            }
        }
    }
    found
        .dependencies
        .sort_by_key(|link| (link.source, link.start, link.target));
    found.packages = packages;
    found
}
