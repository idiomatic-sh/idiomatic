//! The `globbing` service: free functions named by verb that match a glob against all of a text, as today's ryo reads a glob
//! (globset's `Glob`): `*` is any run of characters, `/` included, `**` the same, `**/` any directories or none, `?` one character,
//! `[abc]`, `[a-z]` and `[!a-z]` one character of a class, a backslash makes the byte after it itself, and `{a,b}` is one of the
//! alternatives, groups following one another or nesting.

use super::constants::DIRECTORY_STARS;
use super::types::{
    Alternated, Byte, ClassBounds, ClassClose, ClassMembership, GlobIndex, GlobMatch, GlobPiece,
    GlobText, Letter, Negation, OwnedGlobText, Ruling, Subject, TextGlobProgram,
};

/// How long the three pieces of a pattern are together.
fn joined_len(pattern: Alternated<'_>) -> GlobIndex {
    GlobIndex(pattern.before.0.len() + pattern.alternative.0.len() + pattern.after.0.len())
}

/// The byte of the pattern at `at`, read across its three pieces; a NUL past its end, which no glob syntax names.
fn byte_at(pattern: Alternated<'_>, at: GlobIndex) -> Byte {
    let at = at.0;
    let (before, alternative, after) = (pattern.before.0, pattern.alternative.0, pattern.after.0);
    let found = if at < before.len() {
        before.as_bytes().get(at)
    } else if at < before.len() + alternative.len() {
        alternative.as_bytes().get(at - before.len())
    } else {
        after.as_bytes().get(at - before.len() - alternative.len())
    };
    Byte(found.copied().unwrap_or(0))
}

/// Whether the first or the last byte of the pattern, when it is a plain byte and not a wildcard, already rules the text out:
/// most alternatives of a long list are refused by their first byte alone.
fn refused_at_the_ends(pattern: Alternated<'_>, text: Subject<'_>) -> Ruling {
    let (Some(first), Some(last)) = (
        [pattern.before, pattern.alternative, pattern.after]
            .iter()
            .find_map(|part| part.0.as_bytes().first().copied()),
        [pattern.after, pattern.alternative, pattern.before]
            .iter()
            .find_map(|part| part.0.as_bytes().last().copied()),
    ) else {
        return if text.0.is_empty() {
            Ruling::Leaves
        } else {
            Ruling::RulesOut
        };
    };
    let bytes = text.0.as_bytes();
    let plain = |byte: Byte| !matches!(byte.0, b'*' | b'?' | b'{' | b'[' | b']' | b'\\');
    if (plain(Byte(first)) && bytes.first() != Some(&first))
        || (plain(Byte(last)) && bytes.last() != Some(&last))
    {
        Ruling::RulesOut
    } else {
        Ruling::Leaves
    }
}

/// Where the class that opens at `open` closes, and whether it is negated (`[!a-z]`); none when it never closes, and then
/// the `[` is itself. A `]` right after the opening is a member of the class.
fn class_close(pattern: Alternated<'_>, open: GlobIndex) -> Option<ClassClose> {
    let open = open.0;
    let negated = matches!(byte_at(pattern, GlobIndex(open + 1)), Byte(b'!' | b'^'))
        && open + 1 < joined_len(pattern).0;
    let first = open + 1 + usize::from(negated);
    (first..joined_len(pattern).0)
        .find(|index| *index > first && byte_at(pattern, GlobIndex(*index)) == Byte(b']'))
        .map(|close| ClassClose {
            close: GlobIndex(close),
            negation: if negated {
                Negation::Negated
            } else {
                Negation::Plain
            },
        })
}

/// Whether `found` is a member of the class between `open` and `close`: its single characters and its ranges `a-z`.
fn in_class(pattern: Alternated<'_>, bounds: ClassBounds, found: Letter) -> ClassMembership {
    let ClassBounds {
        open: GlobIndex(open),
        close: GlobIndex(close),
        negation,
    } = bounds;
    let negated = negation == Negation::Negated;
    let found = found.0;
    let mut index = open + 1 + usize::from(negated);
    let mut member = false;
    while index < close {
        let low = char::from(byte_at(pattern, GlobIndex(index)).0);
        if index + 2 < close && byte_at(pattern, GlobIndex(index + 1)) == Byte(b'-') {
            let high = char::from(byte_at(pattern, GlobIndex(index + 2)).0);
            member |= (low..=high).contains(&found);
            index += 3;
        } else {
            member |= low == found;
            index += 1;
        }
    }
    if member == negated {
        ClassMembership::Out
    } else {
        ClassMembership::In
    }
}

/// Whether the brace-free pattern matches all of `text`: `*` and `**` are any run, `**/` any directories or none, `?` one character.
/// A star that failed is retried one character further, so the work is the pattern times the text, never more.
fn matches_plain(pattern: Alternated<'_>, text: Subject<'_>) -> GlobMatch {
    if refused_at_the_ends(pattern, text) == Ruling::RulesOut {
        return GlobMatch::Differs;
    }
    let text = text.0;
    let bytes = text.as_bytes();
    let (mut at, mut here) = (0_usize, 0_usize);
    // The last star: where in the pattern it ended, where in the text it now reaches, and whether it is a `**/`.
    let mut star: Option<(usize, usize, bool)> = None;
    while here < bytes.len() || at < joined_len(pattern).0 {
        if at < joined_len(pattern).0 && byte_at(pattern, GlobIndex(at)) == Byte(b'*') {
            let mut end = at + 1;
            while end < joined_len(pattern).0 && byte_at(pattern, GlobIndex(end)) == Byte(b'*') {
                end += 1;
            }
            let directories = end - at >= 2
                && end < joined_len(pattern).0
                && byte_at(pattern, GlobIndex(end)) == Byte(b'/');
            if directories {
                end += 1;
            }
            star = Some((end, here, directories));
            at = end;
            continue;
        }
        // A backslash makes the byte after it itself.
        if at + 1 < joined_len(pattern).0
            && byte_at(pattern, GlobIndex(at)) == Byte(b'\\')
            && bytes
                .get(here)
                .is_some_and(|byte| byte_at(pattern, GlobIndex(at + 1)) == Byte(*byte))
        {
            at += 2;
            here += 1;
            continue;
        }
        // A class takes one character, and when it holds none the text goes on only by a star before it.
        let class = (at < joined_len(pattern).0
            && here < bytes.len()
            && byte_at(pattern, GlobIndex(at)) == Byte(b'['))
        .then(|| class_close(pattern, GlobIndex(at)))
        .flatten();
        // The character of the text at `here`, none when `here` is past its end or inside a character.
        let character = text.get(here..).and_then(|rest| rest.chars().next());
        match class {
            Some(ClassClose {
                close: GlobIndex(close),
                negation,
            }) => {
                let found = character.unwrap_or_default();
                let bounds = ClassBounds {
                    open: GlobIndex(at),
                    close: GlobIndex(close),
                    negation,
                };
                if in_class(pattern, bounds, Letter(found)) == ClassMembership::In {
                    at = close + 1;
                    here += found.len_utf8();
                    continue;
                }
            }
            None if at < joined_len(pattern).0
                && byte_at(pattern, GlobIndex(at)) != Byte(b'\\')
                && bytes.get(here).is_some_and(|byte| {
                    byte_at(pattern, GlobIndex(at)) == Byte(*byte)
                        || byte_at(pattern, GlobIndex(at)) == Byte(b'?')
                }) =>
            {
                let step = if byte_at(pattern, GlobIndex(at)) == Byte(b'?') {
                    character.map_or(1, char::len_utf8)
                } else {
                    1
                };
                at += 1;
                here += step;
                continue;
            }
            None => {}
        }
        let Some((after, reach, directories)) = star else {
            return GlobMatch::Differs;
        };
        // Retry the star over one more character; a directory star may end only after a `/`.
        let mut next = reach;
        loop {
            if next >= bytes.len() {
                return GlobMatch::Differs;
            }
            next += text
                .get(next..)
                .and_then(|rest| rest.chars().next())
                .map_or(1, char::len_utf8);
            if !directories || next.checked_sub(1).and_then(|last| bytes.get(last)) == Some(&b'/') {
                break;
            }
        }
        star = Some((after, next, directories));
        at = after;
        here = next;
    }
    GlobMatch::Matches
}

/// Where a class that opens at `open` ends in `pattern`, when it closes: a `]` right after the opening (and its `!` or `^`) is a
/// member.
fn class_end(pattern: GlobText<'_>, open: GlobIndex) -> Option<GlobIndex> {
    let bytes = pattern.0.as_bytes();
    let first = open.0 + 1 + usize::from(matches!(bytes.get(open.0 + 1), Some(b'!' | b'^')));
    let from = first + 1;
    bytes
        .get(from..)
        .and_then(|rest| rest.iter().position(|byte| *byte == b']'))
        .map(|offset| GlobIndex(from + offset))
}

/// The first byte at or after `from` that is `wanted` and not inside a class.
fn byte_outside_classes(pattern: GlobText<'_>, from: GlobIndex, wanted: Byte) -> Option<GlobIndex> {
    let bytes = pattern.0.as_bytes();
    let mut index = from.0;
    while let Some(&byte) = bytes.get(index) {
        if byte == wanted.0 {
            return Some(GlobIndex(index));
        }
        index = match byte {
            b'[' => class_end(pattern, GlobIndex(index)).map_or(index + 1, |end| end.0 + 1),
            b'\\' => index + 2,
            _ => index + 1,
        };
    }
    None
}

/// Where the first alternatives group of the glob opens.
fn brace_open(pattern: GlobText<'_>) -> Option<GlobIndex> {
    byte_outside_classes(pattern, GlobIndex(0), Byte(b'{'))
}

/// Where the group that opens at `open` closes: the `}` that balances it, nested groups and classes passed over.
fn brace_close(pattern: GlobText<'_>, open: GlobIndex) -> Option<GlobIndex> {
    let bytes = pattern.0.as_bytes();
    let mut depth = 0_usize;
    let mut index = open.0 + 1;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'{' => depth += 1,
            b'}' if depth == 0 => return Some(GlobIndex(index)),
            b'}' => depth -= 1,
            b'\\' => index += 1,
            b'[' => {
                if let Some(end) = class_end(pattern, GlobIndex(index)) {
                    index = end.0;
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Whether any alternative of the group `inside`, split at the commas that are not in a nested group or a class, matches by
/// `holds`.
fn any_alternative(
    inside: GlobText<'_>,
    mut holds: impl FnMut(GlobPiece<'_>) -> GlobMatch,
) -> GlobMatch {
    let text = inside.0;
    let bytes = text.as_bytes();
    let (mut depth, mut start, mut index) = (0_usize, 0_usize, 0_usize);
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'\\' => index += 1,
            b',' if depth == 0 => {
                let piece = GlobPiece(text.get(start..index).unwrap_or_default());
                if holds(piece) == GlobMatch::Matches {
                    return GlobMatch::Matches;
                }
                start = index + 1;
            }
            b'[' => {
                if let Some(end) = class_end(inside, GlobIndex(index)) {
                    index = end.0;
                }
            }
            _ => {}
        }
        index += 1;
    }
    holds(GlobPiece(text.get(start..).unwrap_or_default()))
}

/// Whether the glob `pattern` matches all of `text`: `{a,b}` is one of its alternatives, and groups may follow one another or nest.
#[must_use]
pub(crate) fn matches(pattern: GlobText<'_>, text: Subject<'_>) -> GlobMatch {
    let Some((open, close)) =
        brace_open(pattern).and_then(|open| brace_close(pattern, open).map(|close| (open, close)))
    else {
        return matches_plain(
            Alternated {
                before: GlobPiece(pattern.0),
                alternative: GlobPiece::default(),
                after: GlobPiece::default(),
            },
            text,
        );
    };
    let glob = pattern.0;
    // The braces are ASCII, so the places beside them are the boundaries of characters.
    let before = glob.get(..open.0).unwrap_or_default();
    let after = glob.get(close.0 + 1..).unwrap_or_default();
    let inside = glob.get(open.0 + 1..close.0).unwrap_or_default();
    any_alternative(GlobText(inside), |alternative| {
        // The common case, one group of plain alternatives, is matched in place; any other is expanded one group at a time.
        if brace_open(GlobText(alternative.0)).is_none() && brace_open(GlobText(after)).is_none() {
            matches_plain(
                Alternated {
                    before: GlobPiece(before),
                    alternative,
                    after: GlobPiece(after),
                },
                text,
            )
        } else {
            matches(GlobText(&format!("{before}{}{after}", alternative.0)), text)
        }
    })
}

/// Makes the library-compatible subset ready once; other syntax keeps the interpreter.
/// ASCII literals and ordinary stars are equivalent even for Unicode subjects. Question
/// marks, classes, escapes and directory stars have additional character or path semantics.
#[must_use]
pub(crate) fn compile(patterns: &[GlobText<'_>]) -> TextGlobProgram {
    let mut expressions = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        if !pattern.0.is_ascii()
            || pattern.0.contains(['?', '[', '\\'])
            || pattern.0.contains(DIRECTORY_STARS.0)
        {
            return interpreted(patterns);
        }
        let Ok(glob) = globset::GlobBuilder::new(pattern.0)
            .literal_separator(false)
            .empty_alternates(true)
            .build()
        else {
            return interpreted(patterns);
        };
        expressions.push(glob.regex().to_owned());
    }
    let syntax = regex_automata::util::syntax::Config::new()
        .utf8(false)
        .dot_matches_new_line(true);
    let config = regex_automata::meta::Regex::config()
        .match_kind(regex_automata::MatchKind::All)
        .utf8_empty(false)
        .nfa_size_limit(Some(10 * (1 << 20_i32)))
        .hybrid_cache_capacity(10 * (1 << 20));
    regex_automata::meta::Regex::builder()
        .syntax(syntax)
        .configure(config)
        .build_many(&expressions)
        .map_or_else(|_| interpreted(patterns), TextGlobProgram::Ready)
}

/// Keep authored patterns only when the exact compiler optimization is unavailable.
fn interpreted(patterns: &[GlobText<'_>]) -> TextGlobProgram {
    TextGlobProgram::Interpreted(
        patterns
            .iter()
            .map(|pattern| OwnedGlobText(Box::from(pattern.0)))
            .collect(),
    )
}

/// Match raw text using the strategy selected during semantic compilation.
#[must_use]
pub(crate) fn matches_compiled(compiled: &TextGlobProgram, text: Subject<'_>) -> GlobMatch {
    let matched = match compiled {
        TextGlobProgram::Ready(matcher) => matcher.is_match(text.0.as_bytes()),
        TextGlobProgram::Interpreted(patterns) => patterns
            .iter()
            .any(|pattern| matches(GlobText(&pattern.0), text) == GlobMatch::Matches),
    };
    if matched {
        GlobMatch::Matches
    } else {
        GlobMatch::Differs
    }
}
