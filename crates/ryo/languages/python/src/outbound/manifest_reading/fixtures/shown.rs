//! What an adapter's cases compare: a declaration, as one line of text.

use crate::domain::packages::types::{Checkout, Declared, ManifestPath, Naming, Reference};
use crate::domain::patterns::types::Pattern;
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use ryo::languages::{DependencyOrigin, FilePath, ManifestFile, ManifestText};

/// The manifests a case gives an adapter: each with its path and its text. The first is the one read; all are available to it.
pub(crate) type Files = &'static [(&'static str, &'static str)];

/// A declaration as one line: the name, the member patterns, the exclusion patterns, the redirects (with where each says the package
/// comes from when it is not a path), then each requirement with its kind, whether it is optional, where it says the package is,
/// where it comes from when it is not the registry, and the version it asks for in `<..>`, with `!` when that admits one version.
#[must_use]
pub(crate) fn shown(declared: &Declared) -> String {
    let name = match &declared.name {
        Naming::Named(name) => name.0.to_string(),
        Naming::Unnamed => String::new(),
    };
    let join = |each: &[Pattern]| {
        each.iter()
            .map(|pattern| pattern.0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    // A revision is shown by what it is, where one is written.
    let checkout = |each: &Checkout| -> String {
        let mut shown = String::new();
        for (key, revision) in [
            ("branch", &each.branch),
            ("tag", &each.tag),
            ("rev", &each.revision),
        ] {
            if !revision.0.is_empty() {
                shown.push(' ');
                shown.push_str(key);
                shown.push('=');
                shown.push_str(&revision.0);
            }
        }
        shown
    };
    let sources: Vec<String> = declared
        .sources
        .iter()
        .map(|redirect| {
            let place = redirect
                .location
                .as_ref()
                .map_or("?", |location| &*location.0);
            let place = if redirect.origin == DependencyOrigin::Path {
                place.to_owned()
            } else {
                format!("{place}:{:?}", redirect.origin)
            };
            let place = format!("{place}{}", checkout(&redirect.checkout));
            if redirect.marker.0.is_empty() {
                format!("{}={place}", redirect.name.0)
            } else {
                format!("{}={place} ; {}", redirect.name.0, redirect.marker.0)
            }
        })
        .collect();
    let requirements: Vec<String> = declared
        .requirements
        .iter()
        .map(|each| {
            let place = match &each.reference {
                Reference::Named => String::new(),
                Reference::Located(location) => format!(" @{}", location.0),
                Reference::Unresolvable => " @?".to_owned(),
            };
            let origin = if each.origin == DependencyOrigin::Registry {
                String::new()
            } else {
                format!(" {:?}", each.origin)
            };
            let version = if each.specifier.0.is_empty() {
                String::new()
            } else if each.exact == ryo::languages::Flag::Raised {
                format!(" <{}>!", each.specifier.0)
            } else {
                format!(" <{}>", each.specifier.0)
            };
            let python = if each.interpreter.0.is_empty() {
                String::new()
            } else {
                format!(" python={}", each.interpreter.0)
            };
            format!(
                "{} {:?} {:?}{place}{origin}{version}{}{python}",
                each.written.0,
                each.kind,
                each.optional,
                checkout(&each.checkout)
            )
        })
        .collect();
    // The overrides are shown only where the manifest writes some.
    let mut overrides = String::new();
    for each in &declared.overrides {
        overrides.push_str(" | override ");
        overrides.push_str(&each.0);
    }
    format!(
        "{name} | {} | {} | {} | {}{overrides}",
        join(&declared.members),
        join(&declared.excludes),
        sources.join(","),
        requirements.join(", ")
    )
}

/// What the adapter declares for the first of `files`, shown; the others are the manifests of the repository.
///
/// # Errors
///
/// What the adapter refuses with, and a refusal when `files` holds no manifest to read.
pub(crate) fn declared_by<P>(
    build: impl Fn(Vec<ManifestFile>) -> P,
    files: Files,
) -> Result<String, ManifestReadingError>
where
    P: ManifestReadingPort,
{
    let all: Vec<ManifestFile> = files
        .iter()
        .map(|(path, text)| ManifestFile {
            path: FilePath((*path).into()),
            text: ManifestText((*text).into()),
        })
        .collect();
    let Some(own) = all.first() else {
        return Err(ManifestReadingError::Unreadable {
            reason: String::from("the row gives no manifest to read"),
        });
    };
    build(all.clone())
        .declared(own)
        .map(|declared| shown(&declared))
}

/// What `read` declares of the first of `files`, given all of them, as one line: the schemas of an adapter read without the adapter.
///
/// # Errors
///
/// When the row gives no manifest, or `read` refuses the first.
pub(crate) fn declared_with(
    read: impl Fn(&ManifestPath, &str, &[ManifestFile]) -> Result<Declared, ManifestReadingError>,
    files: Files,
) -> Result<String, ManifestReadingError> {
    let all: Vec<ManifestFile> = files
        .iter()
        .map(|(path, text)| ManifestFile {
            path: FilePath((*path).into()),
            text: ManifestText((*text).into()),
        })
        .collect();
    let Some(own) = all.first() else {
        return Err(ManifestReadingError::Unreadable {
            reason: String::from("the row gives no manifest to read"),
        });
    };
    read(&ManifestPath(own.path.0.clone()), &own.text, &all).map(|declared| shown(&declared))
}
