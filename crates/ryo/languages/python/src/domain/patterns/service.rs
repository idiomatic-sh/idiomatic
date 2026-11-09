//! The `patterns` service: free functions named by verb over the packages' patterns and directories, with no IO and no ports. A pattern
//! is a glob of path parts: `*` and `?` inside one part, `**` for any number of parts.

use crate::domain::patterns::constants::{ANY_ONE, ANY_PARTS, ANY_RUN, HERE, SEPARATOR};
use crate::domain::patterns::types::{Directory, Matching, Pattern, PatternPart};

/// Whether the directory `directory` is the one the pattern `pattern` names, both from the same place.
#[must_use]
pub(crate) fn matches_directory(pattern: &Pattern, directory: Directory<'_>) -> Matching {
    let wanted: Vec<PatternPart<'_>> = pattern
        .0
        .split(SEPARATOR)
        .filter(|part| !part.is_empty() && *part != HERE)
        .map(PatternPart)
        .collect();
    let found: Vec<PatternPart<'_>> = directory
        .0
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .map(PatternPart)
        .collect();
    parts_match(&wanted, &found)
}

/// Whether the parts of a pattern name the parts of a directory one by one.
fn parts_match(wanted: &[PatternPart<'_>], found: &[PatternPart<'_>]) -> Matching {
    let Some((first, rest)) = wanted.split_first() else {
        return if found.is_empty() {
            Matching::Matches
        } else {
            Matching::Differs
        };
    };
    if first.0 == ANY_PARTS {
        return if (0..=found.len())
            .filter_map(|skip| found.get(skip..))
            .any(|tail| parts_match(rest, tail) == Matching::Matches)
        {
            Matching::Matches
        } else {
            Matching::Differs
        };
    }
    match found.split_first() {
        Some((head, others))
            if text_matches(*first, *head) == Matching::Matches
                && parts_match(rest, others) == Matching::Matches =>
        {
            Matching::Matches
        }
        Some(_) | None => Matching::Differs,
    }
}

/// Whether `text` is what the pattern `pattern` of one part names.
fn text_matches(pattern: PatternPart<'_>, text: PatternPart<'_>) -> Matching {
    let mut rest = pattern.0.chars();
    let verdict = match rest.next() {
        None => text.0.is_empty(),
        Some(ANY_RUN) => {
            let after = rest.as_str();
            (0..=text.0.len())
                .filter_map(|place| text.0.get(place..))
                .any(|tail| {
                    text_matches(PatternPart(after), PatternPart(tail)) == Matching::Matches
                })
        }
        Some(ANY_ONE) => {
            let mut others = text.0.chars();
            others.next().is_some()
                && text_matches(PatternPart(rest.as_str()), PatternPart(others.as_str()))
                    == Matching::Matches
        }
        Some(literal) => text.0.strip_prefix(literal).is_some_and(|others| {
            text_matches(PatternPart(rest.as_str()), PatternPart(others)) == Matching::Matches
        }),
    };
    if verdict {
        Matching::Matches
    } else {
        Matching::Differs
    }
}
