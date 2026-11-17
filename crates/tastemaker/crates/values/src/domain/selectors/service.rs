//! The `selectors` service: free functions named by verb that resolve a selector list at one place and ask what it
//! matches and covers, with no IO and no ports; patterns are compiled by the function the caller hands in.

use std::collections::{BTreeMap, BTreeSet};

use super::collections::selectors::interface::SelectorsItem;
use super::errors::SelectorsError;
use super::types::{
    Coverage, FirstMatch, Match, MatcherShape, Matching, PathMatcher, PathPattern, PathRest,
    PatternCompiler, Repetition, RepoPathText, Resolution, Resolved, ResolvedSelectors,
    SelectionContext, Selector, SelectorIndex, Selectors,
};
use crate::domain::captures::service::{captured_names, substitute_pattern, unescaped, wildcards};
use crate::domain::captures::types::{
    CaptureBinding, CaptureName, CaptureValue, Captures, Interpolations, Template, Wildcards,
};
use crate::domain::paths::service::{ancestors, containment, join, relative};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::{Containment, PathText};

/// `selectors` resolved at the place `context` describes, in the order written.
///
/// # Errors
///
/// `Pattern` when a pattern does not compile; `Unresolved` for a variable left in the list.
pub fn resolve(
    selectors: &Selectors,
    context: &SelectionContext<'_>,
) -> Result<ResolvedSelectors, SelectorsError> {
    let mut resolved = Vec::with_capacity(selectors.0.len());
    for selector in &selectors.0 {
        if let Selector::Variable(name) = selector {
            return Err(SelectorsError::Unresolved {
                name: name.0.to_string(),
            });
        }
        for item in items() {
            if let Resolution::Resolved(found) = item.resolve(selector, context)? {
                resolved.push(found);
                break;
            }
        }
    }
    Ok(ResolvedSelectors(resolved))
}

/// Every selector kind, in the order a selector is offered to them.
fn items() -> [&'static dyn SelectorsItem; 4] {
    [
        &super::collections::selectors::glob::item::GlobItem,
        &super::collections::selectors::regex::item::RegexItem,
        &super::collections::selectors::part::item::PartItem,
        &super::collections::selectors::placed_by::item::PlacedByItem,
    ]
}

/// `pattern` placed at `base` with `values` interpolated: the glob kind's own placement, for a template entry's `file`.
///
/// # Errors
///
/// `Pattern` when the pattern does not compile.
pub fn place(
    base: &RepoPath,
    pattern: &PathPattern,
    values: &Interpolations,
    compile: &dyn PatternCompiler,
) -> Result<PathMatcher, SelectorsError> {
    super::collections::selectors::glob::item::place(base, pattern, values, compile)
}

/// The part of `path` beneath the matcher's base, which its test reads: none for the base itself and a path outside it.
fn beneath<'p>(matcher: &PathMatcher, path: &'p RepoPath) -> Option<PathRest<'p>> {
    relative(path, &matcher.base)
        .map(|relative| relative.0)
        .filter(|rest| !rest.is_empty())
        .map(PathRest)
}

/// What the matcher captured when `path` itself matches.
#[must_use]
pub fn matches(matcher: &PathMatcher, path: &RepoPath) -> Match {
    let compiled = match &matcher.shape {
        MatcherShape::Itself => {
            return if *path == matcher.base {
                Match::Matched(Captures::default())
            } else {
                Match::Missed
            };
        }
        MatcherShape::Compiled(compiled) => compiled,
    };
    let Some(rest) = beneath(matcher, path) else {
        return Match::Missed;
    };
    let Some(values) = compiled.test.bind(rest) else {
        return Match::Missed;
    };
    let mut captures: BTreeMap<CaptureName, CaptureValue> = BTreeMap::new();
    for (name, value) in compiled.groups.iter().zip(values) {
        match captures.get(name) {
            Some(bound) if *bound != value => return Match::Missed,
            Some(_) => {}
            None => {
                drop(captures.insert(name.clone(), value));
            }
        }
    }
    Match::Matched(Captures(captures))
}

/// Whether `path` or one of its ancestors matches: an entry's `include` and `exclude` cover the matched paths and
/// everything beneath them.
#[must_use]
pub fn covers(matcher: &PathMatcher, path: &RepoPath) -> Coverage {
    if containment(path, &matcher.prefix) != Containment::Inside {
        return Coverage::Uncovered;
    }
    if let MatcherShape::Compiled(compiled) = &matcher.shape
        && compiled.repeats == Repetition::Once
    {
        let Some(rest) = beneath(matcher, path) else {
            return Coverage::Uncovered;
        };
        let passes = |candidate: PathRest<'_>| {
            !candidate.0.is_empty() && compiled.test.passes(candidate) == Coverage::Covered
        };
        let covered = passes(PathRest(rest.0))
            || rest
                .0
                .match_indices('/')
                .any(|(at, _)| passes(PathRest(rest.0.get(..at).unwrap_or_default())));
        return Coverage::from(covered);
    }
    let covered = matches!(matches(matcher, path), Match::Matched(_))
        || ancestors(path)
            .iter()
            .any(|ancestor| matches!(matches(matcher, ancestor), Match::Matched(_)));
    Coverage::from(covered)
}

/// Whether any of `resolved` covers `path`.
#[must_use]
pub fn covers_any(resolved: &ResolvedSelectors, path: &RepoPath) -> Coverage {
    Coverage::from(resolved.0.iter().any(|item| {
        match item {
            Resolved::Pattern(matcher) => covers(matcher, path) == Coverage::Covered,
            Resolved::Paths(paths) => paths
                .iter()
                .any(|named| containment(path, named) == Containment::Inside),
        }
    }))
}

/// Whether `matcher` matches the path `text` itself: the cheap test of a pattern that captures nothing, and the binding
/// of one that does.
fn matches_text(matcher: &PathMatcher, path: RepoPathText<'_>) -> Matching {
    let text = path.0;
    match &matcher.shape {
        MatcherShape::Itself => Matching::from(text == &*matcher.base.0),
        MatcherShape::Compiled(compiled) => {
            let rest = if matcher.base.0.is_empty() {
                text
            } else {
                match text
                    .strip_prefix(&*matcher.base.0)
                    .and_then(|rest| rest.strip_prefix('/'))
                {
                    Some(rest) => rest,
                    None => return Matching::Misses,
                }
            };
            if rest.is_empty() {
                return Matching::Misses;
            }
            if compiled.groups.is_empty() || compiled.repeats == Repetition::Once {
                return Matching::from(compiled.test.passes(PathRest(rest)) == Coverage::Covered);
            }
            Matching::from(matches!(
                matches(matcher, &RepoPath(text.into())),
                Match::Matched(_)
            ))
        }
    }
}

/// Whether `resolved` covers each of `files`, in order: what a directory answers is asked once and kept, so files that
/// share their directories cost a match each, not one for every ancestor.
#[must_use]
pub fn covering(resolved: &ResolvedSelectors, files: &[&RepoPath]) -> Vec<Coverage> {
    if resolved.0.is_empty() {
        return vec![Coverage::Uncovered; files.len()];
    }
    let mut directories: std::collections::HashMap<RepoPathText<'_>, Matching> =
        std::collections::HashMap::new();
    // A directory above every pattern's base matches none of them: past the shortest base, no ancestor is asked. A list
    // of paths can name any directory, so with one every ancestor is.
    let floor = resolved
        .0
        .iter()
        .map(|item| match item {
            Resolved::Pattern(matcher) => Some(&*matcher.base.0),
            Resolved::Paths(_) => None,
        })
        .collect::<Option<Vec<_>>>()
        .and_then(|bases| bases.into_iter().min_by_key(|base| base.len()));
    let above = |directory: RepoPathText<'_>| {
        floor.is_some_and(|floor| {
            !(floor.is_empty()
                || directory.0 == floor
                || directory
                    .0
                    .strip_prefix(floor)
                    .is_some_and(|rest| rest.starts_with('/')))
        })
    };
    let matched = |text: RepoPathText<'_>| {
        Matching::from(resolved.0.iter().any(|item| match item {
            Resolved::Pattern(matcher) => matches_text(matcher, text) == Matching::Matches,
            Resolved::Paths(paths) => paths.contains(&RepoPath(text.0.into())),
        }))
    };
    files
        .iter()
        .map(|file| {
            if matched(RepoPathText(&file.0)) == Matching::Matches {
                return Coverage::Covered;
            }
            let mut rest = &*file.0;
            while let Some((directory, _)) = rest.rsplit_once('/') {
                rest = directory;
                if above(RepoPathText(directory)) {
                    break;
                }
                let answer = if let Some(known) = directories.get(&RepoPathText(directory)) {
                    *known
                } else {
                    let answer = matched(RepoPathText(directory));
                    let _: Option<Matching> = directories.insert(RepoPathText(directory), answer);
                    answer
                };
                if answer == Matching::Matches {
                    return Coverage::Covered;
                }
            }
            Coverage::Uncovered
        })
        .collect()
}

/// What the first of `resolved` that matches `path` itself captured, with its position.
#[must_use]
pub fn first_match(resolved: &ResolvedSelectors, path: &RepoPath) -> Option<FirstMatch> {
    resolved
        .0
        .iter()
        .enumerate()
        .find_map(|(index, item)| match item {
            Resolved::Pattern(matcher) => match matches(matcher, path) {
                Match::Matched(captures) => Some(FirstMatch {
                    position: SelectorIndex(index),
                    captures,
                }),
                Match::Missed => None,
            },
            Resolved::Paths(paths) => paths.contains(path).then(|| FirstMatch {
                position: SelectorIndex(index),
                captures: Captures::default(),
            }),
        })
}

/// The names the matcher captures, in the order its expression binds them.
#[must_use]
pub fn groups(matcher: &PathMatcher) -> &[CaptureName] {
    match &matcher.shape {
        MatcherShape::Itself => &[],
        MatcherShape::Compiled(compiled) => &compiled.groups,
    }
}

/// Every name the matcher captures.
#[must_use]
pub fn names(matcher: &PathMatcher) -> BTreeSet<CaptureName> {
    groups(matcher).iter().cloned().collect()
}

/// Every name the resolved list captures.
#[must_use]
pub fn names_of(resolved: &ResolvedSelectors) -> BTreeSet<CaptureName> {
    resolved
        .0
        .iter()
        .flat_map(|item| match item {
            Resolved::Pattern(matcher) => names(matcher),
            Resolved::Paths(_) => BTreeSet::new(),
        })
        .collect()
}

/// The names captured before the matcher's last capture: an entry's count holds for each value these take; empty when
/// the pattern captures one name.
#[must_use]
pub fn leading(matcher: &PathMatcher) -> BTreeSet<CaptureName> {
    let Some((last, before)) = groups(matcher).split_last() else {
        return BTreeSet::new();
    };
    before
        .iter()
        .filter(|name| *name != last)
        .cloned()
        .collect()
}

/// The directory holding the matcher's last capture for the leading values in `captures`: where one group of an entry's
/// matches lies. None when a wildcard or an unbound name precedes the last capture.
#[must_use]
pub fn group(matcher: &PathMatcher, captures: &Captures) -> Option<RepoPath> {
    let last = groups(matcher).last()?;
    let mut directory = matcher.base.clone();
    for segment in matcher.text.0.split('/') {
        if captured_names(Template(segment)).contains(last) {
            return Some(directory);
        }
        let text = substitute_pattern(
            Template(segment),
            captures
                .0
                .iter()
                .chain(matcher.values.0.iter())
                .map(|(name, value)| CaptureBinding { name, value }),
        );
        if wildcards(Template(&text)) == Wildcards::Held
            || !captured_names(Template(&text)).is_empty()
        {
            return None;
        }
        directory = join(&directory, PathText(&unescaped(Template(&text)).0))?;
    }
    None
}
