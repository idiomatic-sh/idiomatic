//! The `comments` service: free functions named by verb that read a file's comments, as the shared `comment` relation
//! gives them, and its lines as its comments leave them, with no IO and no ports.

use super::constants::MARKER;
use super::types::{
    ByteSpan, Comment, CommentBody, CommentKind, CommentText, Commentary, Documentation, LineCode,
    LineIndex, Passing, SourceLines,
};
use crate::domain::findings::types::{ByteOffset, SourceText};
use crate::domain::findings::types::{Column, Line, Region};
use crate::domain::paths::types::RepoPath;
use std::collections::BTreeMap;

/// A comment as the shared `comment` relation gives it, its content without its delimiters at `span`: a comment of one
/// line is one an ignore may be written in, whatever language reads it; one of several lines is not. A finding's shown line
/// passes over documentation and an ignore, and keeps every other comment.
#[must_use]
pub fn comment(span: ByteSpan, documentation: Documentation, text: CommentText<'_>) -> Comment {
    let kind = if text.0.contains('\n') {
        CommentKind::Other
    } else {
        CommentKind::Line(CommentBody(text.0.into()))
    };
    let ignore = matches!(&kind, CommentKind::Line(body) if body.trim_start().starts_with(MARKER));
    let passing = if documentation == Documentation::Documents || ignore {
        Passing::Passed
    } else {
        Passing::Kept
    };
    Comment {
        span,
        kind,
        passing,
    }
}

/// `text`'s lines, a line holding code when some non-blank character of it lies outside every one of `comments`.
#[must_use]
pub fn lines(text: SourceText<'_>, comments: &[Comment]) -> SourceLines {
    // A text's offsets fit in 64 bits: the conversion saturates rather than fail.
    let starts = std::iter::once(0)
        .chain(
            text.0
                .bytes()
                .enumerate()
                .filter(|(_, byte)| *byte == b'\n')
                .map(|(offset, _)| u64::try_from(offset + 1).unwrap_or(u64::MAX)),
        )
        .collect::<Vec<_>>();
    indexed_lines(
        crate::domain::files::types::SourceView {
            text: crate::domain::files::types::BorrowedText(text.0),
            starts: crate::domain::files::types::LineStarts(&starts),
        },
        comments,
    )
}

/// Adds comment coverage to the inventory's line boundaries without rediscovering them.
#[must_use]
pub fn indexed_lines(
    source: crate::domain::files::types::SourceView<'_>,
    comments: &[Comment],
) -> SourceLines {
    let mut spans = comments
        .iter()
        .map(|comment| comment.span)
        .collect::<Vec<_>>();
    spans.sort_unstable();
    // A comment written after code on its line ends that line; a node that starts with it starts on the next.
    let trails = |comment: &Comment| {
        let start = comment.span.start.0;
        let line = source
            .text
            .get(..start)
            .unwrap_or_default()
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        source
            .text
            .get(line..start)
            .unwrap_or_default()
            .chars()
            .any(|character| !character.is_whitespace())
    };
    let mut passed = comments
        .iter()
        .filter(|comment| comment.passing == Passing::Passed || trails(comment))
        .map(|comment| comment.span)
        .collect::<Vec<_>>();
    passed.sort_unstable();
    let mut lines = SourceLines {
        starts: source
            .starts
            .iter()
            .copied()
            // A start past what the platform addresses saturates, and lies past the text.
            .map(|start| usize::try_from(start).unwrap_or(usize::MAX))
            .filter(|start| *start < source.text.len())
            .map(ByteOffset)
            .collect(),
        code: Vec::new(),
        comments: spans,
        passed,
    };
    for (index, start) in lines.starts.iter().enumerate() {
        let end = lines
            .starts
            .get(index + 1)
            .map_or(source.text.len(), |next| next.0);
        let holds = source
            .text
            .get(start.0..end)
            .unwrap_or_default()
            .char_indices()
            .any(|(offset, character)| {
                !character.is_whitespace()
                    && commented(&lines, ByteOffset(start.0 + offset)) == Commentary::Uncommented
            });
        lines.code.push(if holds {
            LineCode::Code
        } else {
            LineCode::Blank
        });
    }
    lines
}

/// Whether byte `at` lies within one of the comments.
#[must_use]
pub(crate) fn commented(lines: &SourceLines, at: ByteOffset) -> Commentary {
    within(&lines.comments, at)
}

/// Whether byte `at` lies within one of `spans`, sorted.
fn within(spans: &[ByteSpan], at: ByteOffset) -> Commentary {
    let after = spans.partition_point(|span| span.start <= at);
    let covering = after.checked_sub(1).and_then(|before| spans.get(before));
    if covering.is_some_and(|span| at < span.end) {
        Commentary::Commented
    } else {
        Commentary::Uncommented
    }
}

/// The line holding byte `offset`.
#[must_use]
pub fn line_of(lines: &SourceLines, offset: ByteOffset) -> LineIndex {
    let line = lines.starts.partition_point(|start| *start <= offset);
    LineIndex(u32::try_from(line.max(1)).unwrap_or(u32::MAX))
}

/// Whether line `line` holds code.
#[must_use]
pub fn holds_code(lines: &SourceLines, line: LineIndex) -> LineCode {
    usize::try_from(line.0)
        .unwrap_or(usize::MAX)
        .checked_sub(1)
        .and_then(|index| lines.code.get(index))
        .copied()
        .unwrap_or(LineCode::Blank)
}

/// Whether the line holding byte `offset` of `text` has code before it.
#[must_use]
pub fn code_before(lines: &SourceLines, text: SourceText<'_>, offset: ByteOffset) -> LineCode {
    let line = usize::try_from(line_of(lines, offset).0).unwrap_or(1);
    let start = lines.starts.get(line - 1).map_or(0, |start| start.0);
    let found = text
        .0
        .get(start..offset.0)
        .unwrap_or_default()
        .char_indices()
        .any(|(index, character)| {
            !character.is_whitespace()
                && commented(lines, ByteOffset(start + index)) == Commentary::Uncommented
        });
    if found {
        LineCode::Code
    } else {
        LineCode::Blank
    }
}

/// The line a finding is shown on in `text`: the blank space, the documentation, the ignores and the comments that end an earlier
/// line of code a syntax node starts with are passed over; any other comment is kept, as written.
fn first_code(lines: &SourceLines, text: SourceText<'_>, region: Region) -> LineIndex {
    let text = text.0;
    let start_line = region.start_line.0;
    let Some(line_start) = usize::try_from(start_line)
        .unwrap_or(usize::MAX)
        .checked_sub(1)
        .and_then(|index| lines.starts.get(index))
    else {
        return LineIndex(start_line);
    };
    let column = text
        .get(line_start.0..)
        .unwrap_or_default()
        .char_indices()
        .nth(
            usize::try_from(region.start_column.0)
                .unwrap_or(1)
                .saturating_sub(1),
        )
        .map_or(0, |(index, _)| index);
    let from = line_start.0 + column;
    text.get(from..)
        .unwrap_or_default()
        .char_indices()
        .find(|(index, character)| {
            !character.is_whitespace()
                && within(&lines.passed, ByteOffset(from + index)) == Commentary::Uncommented
        })
        .map_or(LineIndex(start_line), |(index, _)| {
            line_of(lines, ByteOffset(from + index))
        })
}

/// `region` with its start moved past the documentation, the ignores and the blank space a syntax node starts with: where a
/// finding on the node is shown. A region whose shown line is its first line, or past its end, keeps its start.
#[must_use]
pub fn code_region(lines: &SourceLines, text: SourceText<'_>, region: Region) -> Region {
    let line = first_code(lines, text, region);
    if line.0 == region.start_line.0 || line.0 > region.end_line.0 {
        return region;
    }
    let indent = usize::try_from(line.0)
        .unwrap_or(usize::MAX)
        .checked_sub(1)
        .and_then(|index| lines.starts.get(index))
        .map_or(0, |start| {
            text.0
                .get(start.0..)
                .unwrap_or_default()
                .chars()
                .take_while(|character| *character != '\n' && character.is_whitespace())
                .count()
        });
    Region {
        start_line: Line(line.0),
        start_column: Column(u32::try_from(indent + 1).unwrap_or(1)),
        ..region
    }
}

/// The line a finding at `region` is shown on: the one place a finding has a line, so its showing, its fingerprint and the
/// reach of an ignore all read the same line.
#[must_use]
pub fn shown_line(lines: &SourceLines, text: SourceText<'_>, region: Region) -> LineIndex {
    LineIndex(code_region(lines, text, region).start_line.0)
}

/// The files among `comments` read for their ignores: those one of whose line comments writes the marker. A file whose
/// comments hold none has no ignore to apply, so its text is not read for them.
pub fn ignoring(comments: &BTreeMap<RepoPath, Vec<Comment>>) -> impl Iterator<Item = &RepoPath> {
    comments.iter().filter_map(|(path, found)| {
        found
            .iter()
            .any(
                |comment| matches!(&comment.kind, CommentKind::Line(body) if body.contains(MARKER)),
            )
            .then_some(path)
    })
}
