//! The `violations` service: free functions over violations, with no IO and no ports.

use super::constants::{ERROR, HERE, NOTE, PARENT, SEPARATOR, WARNING};
use super::types::{
    Fingerprinted, Key, Keyed, Level, LevelWord, LineNumber, Place, RepoPath, Standing,
    Suppression, Violation, Within,
};

/// `raw` in normalized form: backslashes become `/`, `.` segments go and `..` is resolved lexically, never through
/// the file system; an absolute path stays absolute, and a `..` that cannot be resolved in a relative one is kept.
#[must_use]
pub fn normalize(raw: &RepoPath) -> RepoPath {
    let raw = &*raw.0;
    let absolute = raw.starts_with(['/', '\\']);
    let mut segments: Vec<&str> = Vec::new();
    for segment in raw.split(['/', '\\']) {
        if segment.is_empty() || segment == HERE {
            continue;
        }
        if segment == PARENT {
            if segments.last().is_some_and(|last| *last != PARENT) {
                let _: Option<&str> = segments.pop();
            } else if !absolute {
                segments.push(PARENT);
            } else {
                // `..` at an absolute path's root stays at the root.
            }
            continue;
        }
        segments.push(segment);
    }
    let joined = segments.join(&SEPARATOR.to_string());
    RepoPath(
        if absolute {
            format!("{SEPARATOR}{joined}")
        } else {
            joined
        }
        .into(),
    )
}

/// `path` relative to `root` when it lies beneath it, and as it is otherwise.
#[must_use]
pub fn relative(path: &RepoPath, root: &RepoPath) -> RepoPath {
    let inner = path
        .0
        .strip_prefix(&*root.0)
        .and_then(|rest| rest.strip_prefix(SEPARATOR));
    match inner {
        Some(rest) if !rest.is_empty() => RepoPath(rest.into()),
        Some(_) | None => path.clone(),
    }
}

/// The line a violation's fingerprint is read from: its own, or the first for a whole file, as GitHub anchors one.
#[must_use]
pub fn anchor(place: Place) -> LineNumber {
    match place {
        Place::File => LineNumber(1),
        Place::Line(line) | Place::At(line, _) => line,
    }
}

/// How a violation stands in its key: suppressed or reported, whatever the reason.
#[must_use]
pub const fn standing(suppression: &Suppression) -> Standing {
    match suppression {
        Suppression::Unsuppressed => Standing::Reported,
        Suppression::Suppressed(_) => Standing::Suppressed,
    }
}

/// The key of `violation`, when it has its fingerprint.
#[must_use]
pub fn key(violation: &Violation) -> Keyed {
    match violation.fingerprint {
        Fingerprinted::Known(fingerprint) => Keyed::Key(Key {
            tool: violation.tool.clone(),
            rule: violation.rule.clone(),
            path: violation.path.clone(),
            level: violation.level,
            standing: standing(&violation.suppression),
            fingerprint,
        }),
        Fingerprinted::Unknown => Keyed::Unkeyed,
    }
}

/// The keys of `violations` that have one: a violation whose fingerprint is unknown has none.
#[must_use]
pub fn keys(violations: &[Violation]) -> std::collections::BTreeSet<Key> {
    violations
        .iter()
        .filter_map(|violation| match key(violation) {
            Keyed::Key(key) => Some(key),
            Keyed::Unkeyed => None,
        })
        .collect()
}

/// Whether `path`, normalized, lies within the product: an absolute path, or one that keeps a `..`, does not.
#[must_use]
pub fn within(path: &RepoPath) -> Within {
    let outside =
        path.0.starts_with(SEPARATOR) || path.0.split(SEPARATOR).any(|segment| segment == PARENT);
    if outside {
        Within::Outside
    } else {
        Within::Inside
    }
}

/// The word `level` is written with: SARIF's, which the lock, the store and the JSON answer write too.
#[must_use]
pub const fn word(level: Level) -> LevelWord<'static> {
    LevelWord(match level {
        Level::Note => NOTE,
        Level::Warning => WARNING,
        Level::Error => ERROR,
    })
}

/// The level `written` names, when it is one of the words.
#[must_use]
pub fn level_of(written: LevelWord<'_>) -> Option<Level> {
    [Level::Note, Level::Warning, Level::Error]
        .into_iter()
        .find(|level| word(*level) == written)
}
