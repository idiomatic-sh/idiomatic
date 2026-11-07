//! The `links` service: free functions named by verb over the domain's types, with no IO and no ports. It links each module a file
//! imports to the file of the repository it names, as TypeScript's resolution finds it: a relative specifier from the file's directory,
//! a bare one through the `paths` and `baseUrl` of the nearest project configuration and the configurations it extends; a specifier
//! is the file it names, then its JavaScript ending rewritten to TypeScript's, then itself with each ending, then its directory's
//! `index`. A bare specifier no configuration maps names a package, of the repository or outside it, which `package_dependency`
//! answers.

use ryo::languages::{self, FileId, FilePath};

use super::constants::{
    CONFIGURATION, CURRENT, EXTENDS_DEPTH, EXTENSIONS, INDEX, PACKAGE_IMPORT, PARENT, REWRITES,
    ROOT, SEPARATOR, UNRESOLVED_CAUSE, WILDCARD,
};
use super::types::{
    Candidate, Cause, FileIndex, ImportOutcome, TypeScriptImportLink, Linked, Matched, Options, Unresolved,
};
use crate::domain::fact_notes::types::{Note, Specifier};
use crate::domain::packages::service::directory;
use crate::domain::packages::types::{
    AliasPattern, Directory, ManifestPath, Package, Packages, RepositoryPath,
};

/// The number of the file at `place` among the files of the run.
#[must_use]
pub(crate) fn file_id(place: FileIndex) -> FileId {
    FileId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// Where the import of a link is written.
#[must_use]
pub(crate) fn link_span(link: &TypeScriptImportLink) -> languages::Span {
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
pub(crate) fn packages_in<'a>(input: &languages::ManifestInput<'a, Linked>) -> &'a Packages {
    &input.derived.packages
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

/// The file the path `path`, from the repository root, names: the file itself, then its JavaScript ending rewritten, then itself with
/// each ending, then its `index` with each.
#[must_use]
pub(crate) fn probed(files: &[FilePath], path: RepositoryPath<'_>) -> Option<FileId> {
    if let Some(file) = listed(files, path) {
        return Some(file);
    }
    for (ending, endings) in REWRITES {
        if let Some(stem) = path.0.strip_suffix(ending) {
            for each in endings {
                if let Some(file) = listed(files, RepositoryPath(&format!("{stem}{each}"))) {
                    return Some(file);
                }
            }
        }
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

/// The project configuration nearest above the file at `path`: the one in its directory, or the nearest directory above.
fn configuration_of<'a>(packages: &'a Packages, path: RepositoryPath<'_>) -> Option<&'a Package> {
    packages
        .entries
        .iter()
        .filter(|package| {
            let (place, name) = package
                .manifest
                .0
                .rsplit_once(SEPARATOR)
                .unwrap_or((ROOT, &package.manifest.0));
            name == CONFIGURATION
                && (place.is_empty()
                    || path
                        .0
                        .strip_prefix(place)
                        .is_some_and(|rest| rest.starts_with(SEPARATOR)))
        })
        .max_by_key(|package| package.manifest.0.len())
}

/// The configuration at `path`, if the manifests declared it.
fn configuration_at<'a>(packages: &'a Packages, path: &ManifestPath) -> Option<&'a Package> {
    packages
        .entries
        .iter()
        .find(|package| package.manifest == *path)
}

/// The `paths` and the `baseUrl` that hold for the files of `configuration`: its own, or else those of the last configuration it
/// extends that has them, as TypeScript lets a later one replace an earlier one.
fn options_of<'a>(packages: &'a Packages, configuration: &'a Package) -> Options<'a> {
    let mut aliases: Option<&[crate::domain::packages::types::PathAlias]> = None;
    let mut base: Option<&ManifestPath> = None;
    let mut pending = vec![configuration];
    let mut followed = 0;
    while let Some(each) = pending.pop() {
        if followed == EXTENDS_DEPTH || (aliases.is_some() && base.is_some()) {
            break;
        }
        followed += 1;
        if aliases.is_none() && !each.declared.aliases.is_empty() {
            aliases = Some(&each.declared.aliases);
        }
        base = base.or(each.declared.base_url.as_ref());
        pending.extend(
            each.declared
                .extends
                .iter()
                .filter_map(|path| configuration_at(packages, path)),
        );
    }
    Options {
        aliases: aliases.unwrap_or_default(),
        base,
    }
}

/// What `specifier` is matched by in `pattern`: the text the `*` stands for, or nothing for a pattern without one that is the
/// specifier; none when it does not match.
fn matched<'a>(pattern: &AliasPattern, specifier: &'a Specifier) -> Option<Matched<'a>> {
    match pattern.0.split_once(WILDCARD) {
        Some((prefix, suffix)) => specifier
            .0
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(suffix))
            .map(Matched),
        None => (*pattern.0 == *specifier.0).then_some(Matched(ROOT)),
    }
}

/// Where the bare `specifier`, written in the file at `path`, leads: through the alias of the nearest configuration whose pattern
/// matches it with the longest text before its `*`, then from its `baseUrl`; a package otherwise.
fn bare(
    files: &[FilePath],
    packages: &Packages,
    path: RepositoryPath<'_>,
    specifier: &Specifier,
) -> ImportOutcome {
    let Some(configuration) = configuration_of(packages, path) else {
        return ImportOutcome::Outside;
    };
    let options = options_of(packages, configuration);
    let best = options
        .aliases
        .iter()
        .filter_map(|alias| matched(&alias.pattern, specifier).map(|text| (alias, text)))
        .max_by_key(|(alias, _)| {
            alias
                .pattern
                .0
                .split_once(WILDCARD)
                .map_or(alias.pattern.0.len(), |(prefix, _)| prefix.len())
        });
    if let Some((alias, text)) = best {
        return alias
            .targets
            .iter()
            .filter_map(|target| {
                normalized(
                    Directory(ROOT),
                    RepositoryPath(&target.0.replacen(WILDCARD, text.0, 1)),
                )
            })
            .find_map(|target| probed(files, RepositoryPath(&target.0)))
            .map_or(ImportOutcome::Unresolved, ImportOutcome::File);
    }
    options
        .base
        .and_then(|base| normalized(Directory(&base.0), RepositoryPath(&specifier.0)))
        .and_then(|target| probed(files, RepositoryPath(&target.0)))
        .map_or(ImportOutcome::Outside, ImportOutcome::File)
}

/// Where `specifier`, written in the file at `path`, leads.
#[must_use]
pub(crate) fn outcome(
    files: &[FilePath],
    packages: &Packages,
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
    if written.starts_with(SEPARATOR) {
        return ImportOutcome::Outside;
    }
    bare(files, packages, path, specifier)
}

/// How the files of the run link to one another, from the notes the parse left of each, in the order of `files` (none for a file it
/// left none of), and the configurations the manifests declared.
#[must_use]
pub(crate) fn linked(files: &[FilePath], notes: &[Option<Note>], packages: Packages) -> Linked {
    let mut found = Linked::default();
    for (place, (file, note)) in files.iter().zip(notes).enumerate() {
        let Some(note) = note else {
            continue;
        };
        let source = file_id(FileIndex(place));
        found.analyzed.push(source);
        for import in &note.imports {
            match outcome(files, &packages, RepositoryPath(&file.0), &import.specifier) {
                ImportOutcome::File(target) if target != source => found.dependencies.push(TypeScriptImportLink {
                    source,
                    target,
                    start: import.start,
                    end: import.end,
                    written: import.text.clone(),
                }),
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
    found.packages = packages;
    found
}
