//! The `targets` service: free functions named by verb that resolve a target a file writes, with no IO and no ports. A target with a
//! scheme (`https:`, `mailto:`, `data:`) or a host (`//cdn`) names something outside the run and stays as written; a fragment alone
//! names a place in the file itself; any other is a path, from the root of the run when it opens with `/`, else resolved against the
//! directory of the file that writes it, its query or fragment kept.

use super::constants::{FRAGMENT, HERE, HOST, QUERY, SCHEME_CHARACTERS, SCHEME_END, SEPARATOR, UP};
use super::types::{PathPart, ResolvedTarget, Scheme, TargetReach, WrittenTarget};
use crate::domain::files::types::SourcePath;

/// Whether a target opens with a scheme: a letter, then letters, digits, `+`, `.` or `-`, then `:`.
fn scheme_of(written: WrittenTarget<'_>) -> Scheme {
    let Some((scheme, _)) = written.0.split_once(SCHEME_END) else {
        return Scheme::Absent;
    };
    let mut characters = scheme.chars();
    if characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|each| each.is_ascii_alphanumeric() || SCHEME_CHARACTERS.contains(&each))
    {
        Scheme::Present
    } else {
        Scheme::Absent
    }
}

/// The directory of a path of the run: all of it but the last part, with no trailing separator; empty for the root.
fn directory_of(located: SourcePath<'_>) -> PathPart<'_> {
    PathPart(
        located
            .0
            .rsplit_once(SEPARATOR)
            .map(|(directory, _)| directory)
            .unwrap_or_default(),
    )
}

/// Appends the parts of `path` to `out` with `.` dropped and each `..` taken against the part before it; a `..` with nothing before it
/// stays, so that a path that leaves the run says so.
fn normalize(path: PathPart<'_>, out: &mut ResolvedTarget) {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.0.split(SEPARATOR) {
        if part.is_empty() || part == HERE {
            continue;
        }
        if part == UP && parts.last().is_some_and(|last| *last != UP) {
            let _: Option<&str> = parts.pop();
        } else {
            parts.push(part);
        }
    }
    for (place, part) in parts.iter().enumerate() {
        if place > 0 {
            out.0.push(SEPARATOR);
        }
        out.0.push_str(part);
    }
}

/// Whether the target `written` names something outside the run: it opens with a scheme or a host.
#[must_use]
pub fn reach_of(written: WrittenTarget<'_>) -> TargetReach {
    if scheme_of(written) == Scheme::Present || written.0.starts_with(HOST) {
        TargetReach::Outside
    } else {
        TargetReach::Inside
    }
}

/// What the target `written` names, written in the file at `located`, as a path of the run.
#[must_use]
pub fn resolve_target(located: SourcePath<'_>, written: WrittenTarget<'_>) -> ResolvedTarget {
    let text = written.0;
    let mut out = ResolvedTarget(String::with_capacity(located.0.len() + text.len()));
    if text.is_empty() || reach_of(written) == TargetReach::Outside {
        out.0.push_str(text);
        return out;
    }
    if text.starts_with(FRAGMENT) {
        out.0.push_str(located.0);
        out.0.push_str(text);
        return out;
    }
    let split = text.find([FRAGMENT, QUERY]).unwrap_or(text.len());
    let (path, rest) = text.split_at(split);
    if let Some(from_root) = path.strip_prefix(SEPARATOR) {
        normalize(PathPart(from_root), &mut out);
    } else {
        let directory = directory_of(located).0;
        let mut joined = String::with_capacity(directory.len() + 1 + path.len());
        joined.push_str(directory);
        joined.push(SEPARATOR);
        joined.push_str(path);
        normalize(PathPart(&joined), &mut out);
    }
    out.0.push_str(rest);
    out
}
