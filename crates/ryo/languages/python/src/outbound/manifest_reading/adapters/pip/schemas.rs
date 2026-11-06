//! The shape of a requirements file, which the language does not control, and its conversion into the port's own types: one
//! requirement of PEP 508 on each line, comments after `#`, lines continued with a trailing `\`, `-r` and `--requirement` including
//! another file, `-e` and `--editable` a path or a URL, `-c` constraints and every other option named by pip not a requirement.
//!
//! A path in a file is taken from the directory of that file, and what a file includes is read as part of it, its requirements of the
//! kind of the file that holds them.

use crate::domain::dependencies::service::pep440_exactness;
use ryo::languages::{DependencyKind, DependencyOrigin, ManifestFile, Optionality};

use super::constants::{PREFIX, SUFFIX};
use crate::domain::dependencies::service::{checkout_of, url_revision};
use crate::domain::packages::service::{
    directory_of, locating, origin_of, specifier_of, url_origin,
};
use crate::domain::packages::types::{
    Checkout, Declared, Interpreter, ManifestPath, Naming, Reference, Requirement, Written,
};
use crate::domain::patterns::types::{Directory, PatternPart};
use crate::outbound::manifest_reading::errors::ManifestReadingError;

/// What opens a comment, after the whitespace that sets it off.
const COMMENT: char = '#';

/// What ends a line that goes on in the next.
const CONTINUED: char = '\\';

/// What opens an option.
const OPTION: char = '-';

/// What opens the options a requirement carries after it, such as `--hash`.
const TRAILING_OPTIONS: &str = " --";

/// Where a name of a distribution ends and what follows it begins.
const AFTER_NAME: &str = " \t[<>=!~;(@,";

/// Marks a URL's scheme.
const SCHEME: &str = "://";

/// The URL schemes pip reads that have no `://`.
const SCHEMES: [&str; 2] = ["git+", "file:"];

/// What marks the name of a distribution in the URL of a repository.
const EGG: &str = "#egg=";

/// The endings of a file of a built distribution.
const ARCHIVES: [&str; 4] = [".whl", ".tar.gz", ".zip", ".tar.bz2"];

/// The names after `requirements-`, and the directories `requirements/`, of the files that are for development.
const DEVELOPMENT: [&str; 5] = ["dev", "test", "tests", "lint", "docs"];

/// The kind of requirement a file holds, from its name: `requirements-dev.txt` and `requirements/dev.txt` are for development.
fn kind_of(path: &ManifestPath) -> DependencyKind {
    let name = path.0.rsplit('/').next().unwrap_or_default();
    let stem = name
        .strip_prefix(PREFIX)
        .unwrap_or(name)
        .strip_suffix(SUFFIX)
        .unwrap_or(name);
    if DEVELOPMENT.contains(&stem) {
        DependencyKind::Dev
    } else {
        DependencyKind::Normal
    }
}

/// The lines of the text, each with its continuation joined and its comment cut, those left empty not among them.
fn logical(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut carried = String::new();
    for line in text.lines() {
        let joined = line.strip_suffix(CONTINUED);
        carried.push_str(joined.unwrap_or(line));
        if joined.is_some() {
            continue;
        }
        let whole = std::mem::take(&mut carried);
        let cut = whole
            .char_indices()
            .find(|(place, each)| {
                *each == COMMENT
                    && whole.get(..*place).is_some_and(|before| {
                        before.chars().next_back().is_none_or(char::is_whitespace)
                    })
            })
            .map_or(whole.len(), |(place, _)| place);
        let kept = whole.get(..cut).unwrap_or(&whole).trim();
        if !kept.is_empty() {
            lines.push(kept.to_owned());
        }
    }
    lines
}

/// The value of an option written `-r value`, `-rvalue`, `--requirement value` or `--requirement=value`.
fn option_value<'a>(line: &'a str, short: &str, long: &str) -> Option<&'a str> {
    if let Some(rest) = line.strip_prefix(long) {
        return match rest.chars().next() {
            Some('=') => rest.strip_prefix('=').map(str::trim),
            Some(each) if each.is_whitespace() => Some(rest.trim()),
            Some(_) | None => None,
        };
    }
    line.strip_prefix(short).map(str::trim)
}

/// What the text before a requirement's end is: a name of a distribution, with what may follow it.
fn names_a_distribution(text: &str) -> Option<()> {
    let end = text
        .find(|each: char| !(each.is_ascii_alphanumeric() || matches!(each, '-' | '_' | '.')))
        .unwrap_or(text.len());
    let starts = text
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric());
    let follows = text
        .get(end..)
        .and_then(|after| after.chars().next())
        .is_none_or(|each| AFTER_NAME.contains(each));
    (end > 0 && starts && follows).then_some(())
}

/// What a line that writes no name of a distribution writes for its name: nothing, and the marker it carries, if any.
fn unnamed(text: &str) -> String {
    text.split_once(';')
        .map_or_else(String::new, |(_, marker)| format!(" ; {}", marker.trim()))
}

/// The requirement a line of a file in `directory` is: what a path, a URL or a name says, and where it says the package comes from.
/// A URL or a path that writes no name has an empty name.
fn requirement(text: &str, directory: Directory<'_>, kind: DependencyKind) -> Requirement {
    let text = text.split(TRAILING_OPTIONS).next().unwrap_or(text).trim();
    let (written, reference, origin, checkout) = if let Some((url, egg)) = text.split_once(EGG) {
        let name = egg.split('&').next().unwrap_or(egg);
        (
            name.to_owned(),
            Reference::Named,
            url_origin(PatternPart(url)),
            Checkout {
                revision: url_revision(PatternPart(url)),
                ..Checkout::default()
            },
        )
    } else if text.contains('@') && names_a_distribution(text).is_some() {
        let written = Written(text.into());
        let origin = origin_of(&written);
        let checkout = checkout_of(&written);
        (text.to_owned(), Reference::Named, origin, checkout)
    } else if text.contains(SCHEME) || SCHEMES.iter().any(|scheme| text.starts_with(scheme)) {
        (
            unnamed(text),
            Reference::Unresolvable,
            url_origin(PatternPart(text)),
            Checkout {
                revision: url_revision(PatternPart(text)),
                ..Checkout::default()
            },
        )
    } else if text.starts_with('.') || text.contains('/') {
        let path = text.split(['[', '#', ';']).next().unwrap_or(text).trim();
        let reference = if ARCHIVES.iter().any(|ending| path.ends_with(ending))
            || path.starts_with('/')
            || path.starts_with('~')
        {
            Reference::Unresolvable
        } else {
            locating(directory, PatternPart(path))
        };
        (
            unnamed(text),
            reference,
            DependencyOrigin::Path,
            Checkout::default(),
        )
    } else if names_a_distribution(text).is_some() {
        (
            text.to_owned(),
            Reference::Named,
            DependencyOrigin::Registry,
            Checkout::default(),
        )
    } else {
        (
            unnamed(text),
            Reference::Unresolvable,
            DependencyOrigin::Registry,
            Checkout::default(),
        )
    };
    let written = Written(written.into());
    let specifier = specifier_of(&written);
    Requirement {
        exact: pep440_exactness(&specifier),
        specifier,
        written,
        kind,
        optional: Optionality::Required,
        reference,
        origin,
        checkout,
        interpreter: Interpreter::default(),
    }
}

/// The requirements of the file at `path` with `text`, and of the files it includes, added to `all`; `seen` holds the files being read.
fn collect(
    path: &ManifestPath,
    text: &str,
    available: &[ManifestFile],
    seen: &mut Vec<ManifestPath>,
    all: &mut Vec<Requirement>,
) -> Result<(), ManifestReadingError> {
    if seen.contains(path) {
        return Ok(());
    }
    seen.push(path.clone());
    let directory = directory_of(path);
    let kind = kind_of(path);
    for line in logical(text) {
        if let Some(included) = option_value(&line, "-r", "--requirement") {
            let found = match locating(directory, PatternPart(included)) {
                Reference::Located(location) => {
                    available.iter().find(|each| *each.path.0 == *location.0)
                }
                Reference::Named | Reference::Unresolvable => None,
            };
            let Some(file) = found else {
                return Err(ManifestReadingError::Unreadable {
                    reason: format!(
                        "{} includes {included}, which is not among the manifests of the repository",
                        path.0
                    ),
                });
            };
            collect(
                &ManifestPath(file.path.0.clone()),
                &file.text,
                available,
                seen,
                all,
            )?;
        } else if let Some(target) = option_value(&line, "-e", "--editable") {
            all.push(requirement(target, directory, kind));
        } else if !line.starts_with(OPTION) {
            all.push(requirement(&line, directory, kind));
        } else {
            // Any other option (an index, a hash, a pip flag) says nothing of what the project requires.
        }
    }
    Ok(())
}

/// A requirements file, with the manifests of the repository it may include.
#[derive(Debug)]
struct RequirementsSchema<'a> {
    /// The file's path from the repository root.
    path: ManifestPath,
    /// The file's text.
    text: &'a str,
    /// The manifests of the repository, which a `-r` or `-c` line may include.
    available: &'a [ManifestFile],
}

impl TryFrom<RequirementsSchema<'_>> for Declared {
    type Error = ManifestReadingError;

    fn try_from(schema: RequirementsSchema<'_>) -> Result<Self, Self::Error> {
        let mut all = Vec::new();
        collect(
            &schema.path,
            schema.text,
            schema.available,
            &mut Vec::new(),
            &mut all,
        )?;
        Ok(Self {
            name: Naming::Unnamed,
            members: Vec::new(),
            excludes: Vec::new(),
            requirements: all,
            sources: Vec::new(),
            overrides: Vec::new(),
        })
    }
}

/// What the requirements file at `manifest`, with `text`, declares, the files it includes read from `available`.
///
/// # Errors
///
/// `Unreadable` when the file includes one that is not among `available`.
pub(super) fn declared(
    manifest: &ManifestPath,
    text: &str,
    available: &[ManifestFile],
) -> Result<Declared, ManifestReadingError> {
    Declared::try_from(RequirementsSchema {
        path: manifest.clone(),
        text,
        available,
    })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
