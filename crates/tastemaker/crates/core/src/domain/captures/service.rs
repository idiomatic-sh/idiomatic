//! The `captures` service: free functions named by verb over what a pattern's captures took, with no IO and no ports.

use std::collections::BTreeSet;

use super::constants::{
    CAPTURE_CLOSE, CAPTURE_OPEN, CAPTURE_SEPARATOR, PATTERN_ESCAPE, PATTERN_SPECIALS, QUESTION,
    SPAN_MARK, STAR,
};
use super::types::{
    CaptureBinding, CaptureName, Captures, CapturesText, NamedPath, PatternChar, PatternName,
    PatternPiece, PatternWritten, PlainText, Substituted, Template, Wildcards,
};

/// `pattern` with each `{name}` and `{*name}` replaced by its value, the first value of a name winning.
#[must_use]
pub fn substitute<'v>(
    pattern: Template<'_>,
    values: impl IntoIterator<Item = CaptureBinding<'v>>,
) -> Substituted {
    let mut result = pattern.0.to_owned();
    for CaptureBinding { name, value } in values {
        result = result
            .replace(&format!("{{{}}}", name.0), &value.0)
            .replace(&format!("{{*{}}}", name.0), &value.0);
    }
    Substituted(result)
}

/// The captures in `captures` named in `names`.
#[must_use]
pub fn project(captures: &Captures, names: &BTreeSet<CaptureName>) -> Captures {
    Captures(
        captures
            .0
            .iter()
            .filter(|(name, _)| names.contains(*name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    )
}

/// `captures` beside those of the instance that places them: a name of their own wins.
#[must_use]
pub fn inheriting(captures: &Captures, enclosing: &Captures) -> Captures {
    let mut all = enclosing.0.clone();
    all.extend(captures.0.clone());
    Captures(all)
}

/// `captures` as a message writes them: `name = value, name = value`.
#[must_use]
pub fn captures_text(captures: &Captures) -> CapturesText {
    CapturesText(
        captures
            .0
            .iter()
            .map(|(name, value)| format!("{} = {}", name.0, value.0))
            .collect::<Vec<_>>()
            .join(CAPTURE_SEPARATOR),
    )
}

/// The pieces of a path pattern's text: `\\c` is `c` standing for itself, `*` and `?` are wildcards, `{name}` and `{*name}`
/// captures, and every other character itself. A `{` with no `}` after it, and a trailing `\\`, stand for themselves.
#[must_use]
pub fn pieces(text: Template<'_>) -> Vec<PatternPiece<'_>> {
    let mut found = Vec::new();
    let mut rest = text.0;
    while let Some(character) = rest.chars().next() {
        let after = rest.get(character.len_utf8()..).unwrap_or_default();
        if character == PATTERN_ESCAPE {
            if let Some(escaped) = after.chars().next() {
                found.push(PatternPiece::Literal(PatternChar(escaped)));
                rest = after.get(escaped.len_utf8()..).unwrap_or_default();
            } else {
                found.push(PatternPiece::Literal(PatternChar(PATTERN_ESCAPE)));
                rest = after;
            }
            continue;
        }
        if character == CAPTURE_OPEN
            && let Some((name, closed)) = after.split_once(CAPTURE_CLOSE)
        {
            found.push(match name.strip_prefix(SPAN_MARK) {
                Some(span) => PatternPiece::Span(PatternName(span)),
                None => PatternPiece::Capture(PatternName(name)),
            });
            rest = closed;
            continue;
        }
        found.push(match character {
            STAR => PatternPiece::Star,
            QUESTION => PatternPiece::Question,
            other => PatternPiece::Literal(PatternChar(other)),
        });
        rest = after;
    }
    found
}

/// `text` written into a path pattern as itself: each character the pattern gives a meaning escaped, so a file named
/// `a*b` is that file, never a wildcard.
#[must_use]
pub fn escaped(text: PlainText<'_>) -> PatternWritten {
    let mut written = String::with_capacity(text.0.len());
    for character in text.0.chars() {
        if PATTERN_SPECIALS.contains(&character) {
            written.push(PATTERN_ESCAPE);
        }
        written.push(character);
    }
    PatternWritten(written)
}

/// The path a pattern with no wildcard and no capture left names: its escapes taken away. A wildcard or a capture left
/// in it is written as the pattern writes it.
#[must_use]
pub fn unescaped(text: Template<'_>) -> NamedPath {
    NamedPath(
        pieces(text)
            .into_iter()
            .map(|piece| match piece {
                PatternPiece::Literal(character) => character.0.to_string(),
                other @ (PatternPiece::Star
                | PatternPiece::Question
                | PatternPiece::Capture(_)
                | PatternPiece::Span(_)) => written_piece(other).0,
            })
            .collect(),
    )
}

/// The names a path pattern captures, in order: those of its `{name}`s and `{*name}`s, never an escaped brace's.
#[must_use]
pub fn captured_names(text: Template<'_>) -> Vec<CaptureName> {
    pieces(text)
        .into_iter()
        .filter_map(|piece| match piece {
            PatternPiece::Capture(name) | PatternPiece::Span(name) => {
                Some(CaptureName(name.0.into()))
            }
            PatternPiece::Literal(_) | PatternPiece::Star | PatternPiece::Question => None,
        })
        .collect()
}

/// Whether a path pattern holds a wildcard (`*` or `?`) that no escape makes text.
#[must_use]
pub fn wildcards(text: Template<'_>) -> Wildcards {
    if pieces(text)
        .iter()
        .any(|piece| matches!(piece, PatternPiece::Star | PatternPiece::Question))
    {
        Wildcards::Held
    } else {
        Wildcards::None
    }
}

/// `pattern` with each capture bound in `values` put in place, the value written as itself (escaped), and every other
/// piece kept as the pattern writes it: still a pattern, whose `unescaped` text is the path when nothing is left open.
#[must_use]
pub fn substitute_pattern<'v>(
    pattern: Template<'_>,
    values: impl IntoIterator<Item = CaptureBinding<'v>>,
) -> Substituted {
    // The first value of a name wins, as `substitute` puts it in place.
    let mut bound = std::collections::BTreeMap::new();
    for binding in values {
        let _: &mut &str = bound.entry(&*binding.name.0).or_insert(&*binding.value.0);
    }
    let mut written = String::with_capacity(pattern.0.len());
    for piece in pieces(pattern) {
        match piece {
            // A bound capture is its value; one the bindings do not name stays as the pattern writes it.
            PatternPiece::Capture(name) | PatternPiece::Span(name) => match bound.get(name.0) {
                Some(value) => written.push_str(&escaped(PlainText(value)).0),
                None => written.push_str(&written_piece(piece).0),
            },
            other @ (PatternPiece::Literal(_) | PatternPiece::Star | PatternPiece::Question) => {
                written.push_str(&written_piece(other).0);
            }
        }
    }
    Substituted(written)
}

/// A piece of a path pattern as the pattern writes it: a literal character escaped when the pattern gives it a meaning.
#[must_use]
pub fn written_piece(piece: PatternPiece<'_>) -> PatternWritten {
    match piece {
        PatternPiece::Literal(character) => escaped(PlainText(&character.0.to_string())),
        PatternPiece::Star => PatternWritten(STAR.to_string()),
        PatternPiece::Question => PatternWritten(QUESTION.to_string()),
        PatternPiece::Capture(name) => {
            PatternWritten(format!("{CAPTURE_OPEN}{}{CAPTURE_CLOSE}", name.0))
        }
        PatternPiece::Span(name) => PatternWritten(format!(
            "{CAPTURE_OPEN}{SPAN_MARK}{}{CAPTURE_CLOSE}",
            name.0
        )),
    }
}
