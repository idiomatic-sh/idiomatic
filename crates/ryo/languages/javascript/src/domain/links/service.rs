//! The `links` service: free functions named by verb over the domain's types, with no IO and no ports. It links each module a file
//! imports to the file of the repository it names, as Node's resolution finds a relative specifier from the file's directory: the file
//! it names, then itself with each JavaScript ending, then its directory's `index` with each. A bare specifier names a package, of the
//! repository or outside it, which `package_dependency` answers.

use ryo::languages::{self, FileId, FilePath};

use super::constants::{
    CURRENT, EXTENSIONS, INDEX, PACKAGE_IMPORT, PARENT, ROOT, SEPARATOR, UNRESOLVED_CAUSE,
};
use super::types::{
    Candidate, Cause, FileIndex, ImportOutcome, JavaScriptImportLink, Linked, Unresolved,
};
use crate::domain::fact_notes::types::{Note, Specifier};
use crate::domain::packages::types::{Directory, RepositoryPath};

/// The number of the file at `place` among the files of the run.
#[must_use]
pub(crate) fn file_id(place: FileIndex) -> FileId {
    FileId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// Where the import of a link is written.
#[must_use]
pub(crate) fn link_span(link: &JavaScriptImportLink) -> languages::Span {
    languages::Span {
        start: link.start,
        end: link.end,
    }
}

/// Why an unresolved import makes the relation read in part.
#[must_use]
pub(crate) fn cause_of(unresolved: &Unresolved) -> Cause {
    Cause(format!("`{}` {UNRESOLVED_CAUSE}", unresolved.specifier.0).into())
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
pub(crate) fn packages_in<'a>(
    input: &languages::ManifestInput<'a, Linked>,
) -> &'a crate::domain::packages::types::Packages {
    &input.derived.packages
}

/// The directory the file at `path` is in.
fn directory(path: RepositoryPath<'_>) -> Directory<'_> {
    Directory(
        path.0
            .rsplit_once(SEPARATOR)
            .map_or(ROOT, |(directory, _)| directory),
    )
}

/// `path` from the directory `base`, its `.` and `..` parts gone: nothing when it leaves the repository.
#[must_use]
pub(crate) fn normalized(base: Directory<'_>, path: RepositoryPath<'_>) -> Option<Candidate> {
    let mut parts: Vec<&str> = base
        .0
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect();
    for part in path.0.split(SEPARATOR) {
        match part {
            // `..` goes up: the directory it leaves is dropped, and a path above the root names nothing.
            PARENT => {
                let _: &str = parts.pop()?;
            }
            _ if part.is_empty() || part == CURRENT => {}
            _ => parts.push(part),
        }
    }
    Some(Candidate(parts.join(&SEPARATOR.to_string()).into()))
}

/// The number of the file at `path`, if the run lists it.
fn listed(files: &[FilePath], path: RepositoryPath<'_>) -> Option<FileId> {
    match files.binary_search_by(|each| (*each.0).cmp(path.0)) {
        Ok(place) => Some(file_id(FileIndex(place))),
        // The search answers where the path would go: the run does not list it.
        Err(_) => None,
    }
}

/// The file the path `path`, from the repository root, names: the file itself, then itself with each ending, then its `index` with
/// each.
#[must_use]
pub(crate) fn probed(files: &[FilePath], path: RepositoryPath<'_>) -> Option<FileId> {
    if let Some(file) = listed(files, path) {
        return Some(file);
    }
    let index = if path.0.is_empty() {
        INDEX.to_owned()
    } else {
        format!("{}{SEPARATOR}{INDEX}", path.0)
    };
    EXTENSIONS
        .iter()
        .find_map(|each| listed(files, RepositoryPath(&format!("{}{each}", path.0))))
        .or_else(|| {
            EXTENSIONS
                .iter()
                .find_map(|each| listed(files, RepositoryPath(&format!("{index}{each}"))))
        })
}

/// Where `specifier`, written in the file at `path`, leads.
#[must_use]
pub(crate) fn outcome(
    files: &[FilePath],
    path: RepositoryPath<'_>,
    specifier: &Specifier,
) -> ImportOutcome {
    let written = &*specifier.0;
    let relative = written == CURRENT
        || written == PARENT
        || written.starts_with(&format!("{CURRENT}{SEPARATOR}"))
        || written.starts_with(&format!("{PARENT}{SEPARATOR}"));
    if relative {
        return normalized(directory(path), RepositoryPath(written))
            .and_then(|target| probed(files, RepositoryPath(&target.0)))
            .map_or(ImportOutcome::Unresolved, ImportOutcome::File);
    }
    if written.starts_with(PACKAGE_IMPORT) {
        return ImportOutcome::Unresolved;
    }
    ImportOutcome::Outside
}

/// How the files of the run link to one another, from the notes the parse left of each, in the order of `files`: none for a file it
/// left no notes of.
#[must_use]
pub(crate) fn linked(files: &[FilePath], notes: &[Option<Note>]) -> Linked {
    let mut found = Linked::default();
    for (place, (file, noted)) in files.iter().zip(notes).enumerate() {
        let Some(note) = noted else {
            continue;
        };
        let source = file_id(FileIndex(place));
        found.analyzed.push(source);
        for import in &note.imports {
            match outcome(files, RepositoryPath(&file.0), &import.specifier) {
                ImportOutcome::File(target) if target != source => {
                    found.dependencies.push(JavaScriptImportLink {
                        source,
                        target,
                        start: import.start,
                        end: import.end,
                        written: import.text.clone(),
                    })
                }
                ImportOutcome::Unresolved => found.unresolved.push(Unresolved {
                    file: source,
                    specifier: import.specifier.clone(),
                }),
                ImportOutcome::File(_) | ImportOutcome::Outside => {}
            }
        }
    }
    found
        .dependencies
        .sort_by_key(|link| (link.source, link.start, link.target));
    found
}
