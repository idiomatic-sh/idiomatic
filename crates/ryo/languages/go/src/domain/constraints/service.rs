//! The `constraints` service: free functions named by verb over the domain's types, with no IO and no ports. It says which comments are
//! build constraint lines as `go/build` reads them: `constraint.IsGoBuild` and `constraint.IsPlusBuild` say what a line is, and the
//! header `go/build`'s `parseFileHeader` reads says where one counts: a `//go:build` line before the first text that is no comment, a
//! `// +build` line only above a blank line that comes before it.

use ryo::languages::{self, Position, Tree};

use super::constants::{
    GO_BUILD, GO_BUILD_KIND, NEWLINE, PLUS_BUILD, PLUS_BUILD_KIND, SLASH_SLASH, SLASH_STAR,
    STAR_SLASH,
};
use super::types::{At, BlockComment, Constraint, Content, Ending, SourceBytes};

/// `text` without the white space at its ends, as Go's `bytes.TrimSpace` and `strings.TrimSpace` read it: Unicode white space of
/// valid UTF-8, the trimming stopping at a byte that is no UTF-8.
fn trimmed(text: SourceBytes<'_>) -> SourceBytes<'_> {
    let text = text.0;
    let mut start = 0;
    for chunk in text.utf8_chunks() {
        let valid = chunk.valid();
        let kept = valid.trim_start_matches(char::is_whitespace);
        start += valid.len() - kept.len();
        if !kept.is_empty() || !chunk.invalid().is_empty() {
            break;
        }
    }
    let rest = text.get(start..).unwrap_or_default();
    let end = rest.utf8_chunks().last().map_or(0, |chunk| {
        if chunk.invalid().is_empty() {
            let valid = chunk.valid();
            valid.len() - valid.trim_end_matches(char::is_whitespace).len()
        } else {
            0
        }
    });
    SourceBytes(rest.get(..rest.len() - end).unwrap_or_default())
}

/// Whether `rest`, what follows the word of a constraint, ends the word: nothing follows it, or white space does.
fn ending(rest: SourceBytes<'_>) -> Ending {
    if rest.0.is_empty() || trimmed(rest).0.len() < rest.0.len() {
        Ending::Ends
    } else {
        Ending::Continues
    }
}

/// Which constraint the line `line` writes, wherever it is: `constraint.IsGoBuild` and `constraint.IsPlusBuild`.
#[must_use]
pub(crate) fn written(line: SourceBytes<'_>) -> Constraint {
    let line = trimmed(line).0;
    if let Some(rest) = line.strip_prefix(GO_BUILD)
        && ending(SourceBytes(rest)) == Ending::Ends
    {
        return Constraint::GoBuild;
    }
    if let Some(comment) = line.strip_prefix(SLASH_SLASH)
        && let Some(rest) = trimmed(SourceBytes(comment)).0.strip_prefix(PLUS_BUILD)
        && ending(SourceBytes(rest)) == Ending::Ends
    {
        return Constraint::PlusBuild;
    }
    Constraint::Other
}

/// What `line` holds once its comments are left out: a line that holds code ends the header. `block` says whether a block comment is
/// open, before the line and after it.
fn content(line: SourceBytes<'_>, block: &mut BlockComment) -> Content {
    let mut line = line.0;
    while !line.is_empty() {
        if *block == BlockComment::Open {
            let Some(close) = line
                .windows(STAR_SLASH.len())
                .position(|each| each == STAR_SLASH)
            else {
                return Content::Comments;
            };
            *block = BlockComment::Closed;
            line = trimmed(SourceBytes(
                line.get(close + STAR_SLASH.len()..).unwrap_or_default(),
            ))
            .0;
        } else if line.starts_with(SLASH_SLASH) {
            return Content::Comments;
        } else if let Some(rest) = line.strip_prefix(SLASH_STAR) {
            *block = BlockComment::Open;
            line = trimmed(SourceBytes(rest)).0;
        } else {
            return Content::Code;
        }
    }
    Content::Comments
}

/// Which constraint the comment that starts at `start` of `source` is, read as `go/build` reads the file's header.
#[must_use]
pub(crate) fn constraint(source: SourceBytes<'_>, start: At) -> Constraint {
    let bytes = source.0;
    let (mut offset, mut ended, mut block) = (0, false, BlockComment::Closed);
    let mut found = Constraint::Other;
    while offset < bytes.len() {
        let rest = bytes.get(offset..).unwrap_or_default();
        let length = rest
            .iter()
            .position(|each| *each == NEWLINE)
            .unwrap_or(rest.len());
        let raw = rest.get(..length).unwrap_or_default();
        let line_start = offset;
        offset += length + 1;
        let line = trimmed(SourceBytes(raw)).0;
        if line.is_empty() && !ended {
            if found == Constraint::PlusBuild {
                return found;
            }
            continue;
        }
        if !line.starts_with(SLASH_SLASH) {
            ended = true;
        }
        if (line_start..offset).contains(&start.0) {
            let before = raw.get(..start.0 - line_start).unwrap_or_default();
            if block == BlockComment::Open || !trimmed(SourceBytes(before)).0.is_empty() {
                return Constraint::Other;
            }
            match written(SourceBytes(line)) {
                Constraint::GoBuild => return Constraint::GoBuild,
                Constraint::PlusBuild => found = Constraint::PlusBuild,
                Constraint::Other => return Constraint::Other,
            }
        }
        if content(SourceBytes(line), &mut block) == Content::Code {
            return Constraint::Other;
        }
    }
    Constraint::Other
}

/// The name `constraint` is written with, as the `directive` relation gives it: `go:build` or `+build`; none for a comment that is no
/// constraint.
#[must_use]
pub(crate) const fn kind_of(constraint: Constraint) -> SourceBytes<'static> {
    match constraint {
        Constraint::GoBuild => SourceBytes(GO_BUILD_KIND),
        Constraint::PlusBuild => SourceBytes(PLUS_BUILD_KIND),
        Constraint::Other => SourceBytes(&[]),
    }
}

/// What follows the word of the constraint line `line`, without the white space around it: the expression `go/build` reads. `line` is
/// a line `written` says is a constraint.
#[must_use]
pub(crate) fn argument(line: SourceBytes<'_>) -> SourceBytes<'_> {
    let line = trimmed(line).0;
    let rest = line.strip_prefix(GO_BUILD).unwrap_or_else(|| {
        line.strip_prefix(SLASH_SLASH)
            .map_or(line, |comment| trimmed(SourceBytes(comment)).0)
            .strip_prefix(PLUS_BUILD)
            .unwrap_or_default()
    });
    trimmed(SourceBytes(rest))
}

/// Which constraint the comment token at `at` of `tree` is; a comment that is no line comment is none.
#[must_use]
pub(crate) fn constraint_at(tree: &Tree<'_>, at: Position) -> Constraint {
    let (start, end) = (
        usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX),
        usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX),
    );
    let text = SourceBytes(tree.source.0.get(start..end).unwrap_or_default());
    if written(text) == Constraint::Other {
        return Constraint::Other;
    }
    constraint(SourceBytes(tree.source.0), At(start))
}
