//! The `bases` service: free functions named by verb over the targets of a document, with no IO and no ports.

use ryo::languages;

use super::constants::{FRAGMENT, IN_DIRECTORY, QUERY, SEPARATOR};
use super::types::{Base, BasePath, Targeting, Whose};

/// What the targets of a document in the file at `file` are resolved against, given the `href` of its first `<base>` if it has one.
#[must_use]
pub(crate) fn based<'a>(
    file: languages::SourcePath<'a>,
    href: Option<languages::WrittenTarget<'_>>,
) -> Targeting<'a> {
    let base = match href {
        None => Base::Absent,
        Some(href) if languages::reach_of(href) == languages::TargetReach::Outside => Base::Outside,
        Some(href) => {
            // The URL without its query and fragment: they are not part of the directory the base names.
            let path = href
                .0
                .split(FRAGMENT)
                .next()
                .and_then(|before| before.split(QUERY).next())
                .unwrap_or_default();
            if path.is_empty() {
                // A base that names no path is the document's own address.
                Base::Absent
            } else {
                let named = languages::resolve_target(file, languages::WrittenTarget(path)).0;
                // A base that ends in a separator is a directory; any other is a file in its directory, which is the base.
                Base::Inside(BasePath(if path.ends_with(SEPARATOR) || named.is_empty() {
                    format!("{named}{SEPARATOR}{IN_DIRECTORY}")
                } else {
                    named
                }))
            }
        }
    };
    Targeting { file, base }
}

/// The path of the run the URL `written` in a document names: against the base the document names when it names one, else against
/// the file's own path. A fragment alone and the `<base>` element's own `href` are the file's; a relative URL under a base outside
/// the run leaves the run, as written.
#[must_use]
pub(crate) fn resolved(
    at: &Targeting<'_>,
    written: languages::WrittenTarget<'_>,
    whose: Whose,
) -> languages::ResolvedTarget {
    match (&at.base, whose) {
        (Base::Absent, _) | (_, Whose::Base) => languages::resolve_target(at.file, written),
        (_, Whose::Other) if written.0.starts_with(FRAGMENT) => {
            languages::resolve_target(at.file, written)
        }
        (Base::Outside, Whose::Other) => languages::ResolvedTarget(written.0.to_owned()),
        (Base::Inside(path), Whose::Other) => {
            languages::resolve_target(languages::SourcePath(&path.0), written)
        }
    }
}
