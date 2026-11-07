//! The `ignores` service: free functions named by verb that read a file's in-source ignores and apply them to its
//! findings, with no IO and no ports. Where an ignore is written sets what it covers: at the top of the file, before any
//! code, every finding of its rule in the file; at the end of a line, that line; on its own line after code, the item on
//! the next line of code.

use std::collections::BTreeSet;

use super::constants::{LATE_NOTE, NO_RULE};
use super::types::{
    BadIgnore, Covered, Covering, Ignore, ReadIgnores, Sifted, Suppressed, Usage, Written,
};
use crate::domain::comments::constants::MARKER;
use crate::domain::comments::service::{code_before, holds_code, line_of, shown_line};
use crate::domain::comments::types::{
    ByteSpan, Comment, CommentBody, CommentKind, LineCode, LineIndex, SourceLines,
};
use crate::domain::findings::constants::{IGNORE, TASTEMAKER};
use crate::domain::findings::service::{check_id, error, spanning};
use crate::domain::findings::types::SourceText;
use crate::domain::findings::types::{
    Finding, MessageText, Place, Reason, Region, RuleId, Suppression,
};

/// What `comment`, a line comment's text after its marker, says; none when it is not an ignore.
#[must_use]
pub fn read(comment: &CommentBody) -> Option<Written> {
    let body = comment.0.trim_start().strip_prefix(MARKER)?.trim();
    // The separator is a `-` standing alone: rule ids hold `-` only inside words.
    let separator = body.char_indices().find(|(index, character)| {
        *character == '-'
            && body
                .get(..*index)
                .unwrap_or_default()
                .chars()
                .next_back()
                .is_none_or(char::is_whitespace)
            && body
                .get(index + 1..)
                .unwrap_or_default()
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
    });
    let (rule, reason) = match separator {
        Some((index, _)) => (
            body.get(..index).unwrap_or_default().trim(),
            body.get(index + 1..).unwrap_or_default().trim(),
        ),
        None => (body, body.get(body.len()..).unwrap_or_default()),
    };
    Some(if rule.is_empty() {
        Written::NoRule
    } else if reason.is_empty() {
        Written::NoReason {
            rule: RuleId(rule.into()),
        }
    } else {
        Written::Ignore {
            rule: RuleId(rule.into()),
            reason: Reason(reason.into()),
        }
    })
}

/// What an ignore on its own line `line` of `text` covers: the whole file when no code is above it, else the item on the
/// next line of code: that line, the lines after it indented deeper, and the line at its indentation that closes it.
#[must_use]
pub fn reach(lines: &SourceLines, text: SourceText<'_>, line: LineIndex) -> Covered {
    if (1..line.0).all(|above| holds_code(lines, LineIndex(above)) == LineCode::Blank) {
        return Covered::File;
    }
    let count = u32::try_from(lines.code.len()).unwrap_or(u32::MAX);
    let Some(first) =
        (line.0 + 1..=count).find(|below| holds_code(lines, LineIndex(*below)) == LineCode::Code)
    else {
        return Covered::Nothing;
    };
    let written = text.0.split('\n').collect::<Vec<_>>();
    let indent = |at: LineIndex| {
        let row = usize::try_from(at.0)
            .unwrap_or(usize::MAX)
            .checked_sub(1)
            .and_then(|index| written.get(index))
            .copied()
            .unwrap_or_default();
        (row.len() - row.trim_start().len(), row.trim_start())
    };
    let (depth, _) = indent(LineIndex(first));
    let mut last = first;
    for next in first + 1..=count {
        let (inner, line_text) = indent(LineIndex(next));
        if line_text.is_empty() {
            continue;
        }
        if inner > depth {
            last = next;
        } else {
            if inner == depth && line_text.starts_with(['}', ')', ']']) {
                last = next;
            }
            break;
        }
    }
    Covered::Lines {
        first: LineIndex(first),
        last: LineIndex(last),
    }
}

/// Whether a reach covers a finding whose first line of code is `line`, or a finding about the whole file.
fn covers(reach: Covered, line: Option<LineIndex>) -> Covering {
    match (reach, line) {
        (Covered::File, _) => Covering::Covers,
        (Covered::Lines { first, last }, Some(line)) if (first..=last).contains(&line) => {
            Covering::Covers
        }
        (Covered::Lines { .. } | Covered::Nothing, _) => Covering::Misses,
    }
}

/// The usable ignores one file's comments write, and what is wrong with the others.
#[must_use]
pub fn ignores(
    text: SourceText<'_>,
    lines: &SourceLines,
    comments: &[Comment],
    known: &BTreeSet<RuleId>,
) -> ReadIgnores {
    let mut found = Vec::new();
    let mut problems = Vec::new();
    for comment in comments {
        let CommentKind::Line(line) = &comment.kind else {
            continue;
        };
        let Some(written) = read(line) else { continue };
        let at = comment.span;
        match written {
            Written::NoRule => problems.push(BadIgnore {
                at,
                message: MessageText(NO_RULE.into()),
            }),
            Written::NoReason { rule } => problems.push(BadIgnore {
                at,
                message: MessageText(
                    format!(
                        "`ignore: {}` gives no reason: write `ignore: {} - <why the finding is accepted>`",
                        rule.0, rule.0
                    )
                    .into(),
                ),
            }),
            Written::Ignore { rule, .. } if !known.contains(&rule) => problems.push(BadIgnore {
                at,
                message: MessageText(
                    format!("`ignore: {}` names a rule this workspace does not load", rule.0).into(),
                ),
            }),
            Written::Ignore { rule, reason } => {
                let placed = line_of(lines, comment.span.start);
                let reach = if code_before(lines, text, comment.span.start) == LineCode::Code {
                    Covered::Lines { first: placed, last: placed }
                } else {
                    reach(lines, text, placed)
                };
                found.push(Ignore {
                    at,
                    rule,
                    reason,
                    reach,
                    used: Usage::Unused,
                });
            }
        }
    }
    ReadIgnores {
        ignores: found,
        problems,
    }
}

/// Applies a file's ignores to its findings: answers the findings no ignore covers, those one does with its reason, and
/// the ignores that suppress nothing or are malformed, as `(span, message)` problems.
#[must_use]
pub fn apply(
    findings: Vec<Finding>,
    text: SourceText<'_>,
    lines: &SourceLines,
    read: ReadIgnores,
) -> Sifted {
    let ReadIgnores {
        ignores: mut found,
        mut problems,
    } = read;
    // A line's ignore is preferred to the file's for the finding both cover.
    found.sort_by_key(|ignore| ignore.reach == Covered::File);
    let mut whole = BTreeSet::new();
    let mut kept = Vec::new();
    let mut suppressed = Vec::new();
    for finding in findings {
        let region = match finding.place {
            Place::At(region) => Some(region),
            Place::Line(line) => Some(Region {
                start_line: line,
                start_column: crate::domain::findings::types::Column(1),
                end_line: line,
                end_column: crate::domain::findings::types::Column(1),
            }),
            Place::Whole => None,
        };
        if region.is_none() {
            let _: bool = whole.insert(finding.rule.rule.0.clone());
        }
        let first = region.map(|region| shown_line(lines, text, region));
        match found.iter_mut().find(|ignore| {
            finding.rule.check == crate::domain::findings::types::CheckChoice::Rule
                && ignore.rule == finding.rule.rule
                && covers(ignore.reach, first) == Covering::Covers
        }) {
            Some(ignore) => {
                ignore.used = Usage::Used;
                suppressed.push(Suppression {
                    finding,
                    reason: ignore.reason.clone(),
                });
            }
            None => kept.push(finding),
        }
    }
    for ignore in found
        .into_iter()
        .filter(|ignore| ignore.used == Usage::Unused)
    {
        let mut message = format!("`ignore: {}` suppresses nothing: remove it", ignore.rule.0);
        if ignore.reach != Covered::File && whole.contains(&ignore.rule.0) {
            message.push_str(LATE_NOTE);
        }
        problems.push(BadIgnore {
            at: ignore.at,
            message: MessageText(message.into()),
        });
    }
    Sifted {
        kept,
        suppressed,
        problems,
    }
}

/// Where a problem's comment is, as a finding's place.
#[must_use]
pub(crate) fn place_of(text: SourceText<'_>, at: ByteSpan) -> Place {
    spanning(text, at.start, at.end).map_or(Place::Whole, Place::At)
}

/// Applies one file's ignores while its source lease remains pinned.
#[must_use]
pub fn suppress(
    findings: Vec<Finding>,
    path: &crate::domain::paths::types::RepoPath,
    text: SourceText<'_>,
    comments: &[Comment],
    lines: &SourceLines,
    known: &BTreeSet<RuleId>,
) -> Suppressed {
    let read = ignores(text, lines, comments, known);
    let Sifted {
        mut kept,
        suppressed,
        problems,
    } = apply(findings, text, lines, read);
    for problem in problems {
        kept.push(Finding {
            place: place_of(text, problem.at),
            ..error(
                check_id(TASTEMAKER, IGNORE),
                path.clone(),
                problem.message.0.to_string(),
            )
        });
    }
    Suppressed { kept, suppressed }
}

/// Extracts a fingerprint's first source line while the caller holds its file lease.
#[must_use]
pub fn first_line(
    finding: &Finding,
    text: Option<crate::domain::files::types::SourceView<'_>>,
    lines: Option<&SourceLines>,
) -> Option<crate::domain::findings::types::FirstLine> {
    let Place::At(region) = finding.place else {
        return None;
    };
    let text = text?;
    let line = lines.map_or(region.start_line.0, |source| {
        shown_line(source, SourceText(text.text.0), region).0
    });
    let start = text
        .starts
        .get(usize::try_from(line).unwrap_or(usize::MAX).checked_sub(1)?)?;
    text.text
        .get(usize::try_from(*start).unwrap_or(usize::MAX)..)?
        .lines()
        .next()
        .map(|first| crate::domain::findings::types::FirstLine(first.to_owned()))
}
