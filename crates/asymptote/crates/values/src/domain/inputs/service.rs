//! The `inputs` service: the digest of what each pathspec names, and the files a pathspec names in a repository's
//! listing, with no IO and no ports. The digest function is the caller's.

use std::collections::BTreeMap;

use super::constants::{ANY, CURRENT, ONE, PARENT, ROOT, SEPARATOR, TOP};
use super::types::{Named, Segment};
use crate::domain::manifest::types::Pathspec;
use crate::domain::reports::types::{Digest, HashParts};
use crate::domain::violations::types::RepoPath;

/// The digest of what a pathspec names, `listed` its files: `hash` over each file's path and stamp, in path order, so
/// an edit of one file moves the digest of every pathspec that names it, and of no other.
#[must_use]
pub fn named(
    listed: &[RepoPath],
    files: &BTreeMap<RepoPath, Digest>,
    hash: &dyn Fn(HashParts<'_>) -> Digest,
) -> Digest {
    let mut sorted: Vec<&RepoPath> = listed.iter().collect();
    sorted.sort();
    sorted.dedup();
    let mut parts: Vec<&[u8]> = Vec::with_capacity(sorted.len().saturating_mul(2));
    for file in sorted {
        parts.push(file.0.as_bytes());
        parts.push(
            files
                .get(file)
                .map_or(TOP.as_bytes(), |stamp| stamp.0.as_bytes()),
        );
    }
    hash(HashParts(parts))
}

/// `path`'s segments, `.` and empty ones left out and `..` resolved where it can be.
fn segments(path: Segment<'_>) -> Vec<Segment<'_>> {
    let mut kept: Vec<Segment<'_>> = Vec::new();
    for segment in path.0.split(SEPARATOR) {
        if segment.is_empty() || segment == ROOT {
            continue;
        }
        if segment == PARENT && kept.last().is_some_and(|last| last.0 != PARENT) {
            let _: Option<Segment<'_>> = kept.pop();
            continue;
        }
        kept.push(Segment(segment));
    }
    kept
}

/// Whether `text` matches `pattern`, as git matches a pathspec with wildcards: `*` any run of characters, `/`
/// included, and `?` any one.
fn wildcard(pattern: Segment<'_>, text: Segment<'_>) -> Named {
    let mut characters = pattern.0.chars();
    let Some(first) = characters.next() else {
        return if text.0.is_empty() {
            Named::Named
        } else {
            Named::Unnamed
        };
    };
    let rest = Segment(characters.as_str());
    if first == ANY {
        let found = text
            .0
            .char_indices()
            .map(|(at, _)| at)
            .chain([text.0.len()])
            .any(|at| {
                wildcard(rest, Segment(text.0.get(at..).unwrap_or_default())) == Named::Named
            });
        return if found { Named::Named } else { Named::Unnamed };
    }
    let mut remaining = text.0.chars();
    match remaining.next() {
        Some(character) if first == ONE || first == character => {
            wildcard(rest, Segment(remaining.as_str()))
        }
        Some(_) | None => Named::Unnamed,
    }
}

/// The files of `listing`, each a path from the repository's top, sorted, that `spec` names when it is written from
/// the product at `base`, the product's path from the top: the file it names, every file beneath the directory it
/// names, or every file its wildcards match. Each is answered as a path from the product, `..` where it lies outside.
#[must_use]
pub fn beneath(listing: &[RepoPath], base: &RepoPath, spec: &Pathspec) -> Vec<RepoPath> {
    let written = spec.0.strip_prefix(CURRENT).unwrap_or(&spec.0);
    let joined = format!("{}{SEPARATOR}{written}", base.0);
    let target: Vec<Segment<'_>> = segments(Segment(&joined));
    let target_text = target
        .iter()
        .map(|segment| segment.0)
        .collect::<Vec<_>>()
        .join(&SEPARATOR.to_string());
    let from = segments(Segment(&base.0));
    let relative = |file: &RepoPath| -> RepoPath {
        let parts = segments(Segment(&file.0));
        let shared = from
            .iter()
            .zip(&parts)
            .take_while(|(left, right)| left == right)
            .count();
        let up = from.len().saturating_sub(shared);
        let mut out: Vec<&str> = vec![PARENT; up];
        out.extend(parts.iter().skip(shared).map(|segment| segment.0));
        RepoPath(out.join(&SEPARATOR.to_string()).into())
    };
    let globbed = target_text.contains([ANY, ONE]);
    if globbed {
        return listing
            .iter()
            .filter(|file| wildcard(Segment(&target_text), Segment(&file.0)) == Named::Named)
            .map(relative)
            .collect();
    }
    if target_text.is_empty() {
        return listing.iter().map(relative).collect();
    }
    let prefix = format!("{target_text}{SEPARATOR}");
    let start = listing.partition_point(|file| *file.0 < *target_text);
    listing
        .get(start..)
        .unwrap_or_default()
        .iter()
        .take_while(|file| {
            *file.0 == *target_text || file.0.starts_with(&prefix) || *file.0 < *prefix
        })
        .filter(|file| *file.0 == *target_text || file.0.starts_with(&prefix))
        .map(relative)
        .collect()
}
