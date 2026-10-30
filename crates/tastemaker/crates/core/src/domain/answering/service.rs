//! The `answering` service: free functions named by verb that read the lint call's answer back into findings, with no
//! IO and no ports: each rule's rows rendered with its diagnostic, each condition answered, the syntax the contracts
//! compare and the imports kept.

use std::collections::BTreeMap;

use super::constants::{DOC, RAISED, SPAN, TESTS, TEXT};
use super::types::{Heading, Owned};
use crate::domain::applications::constants::{PATH_SEPARATOR, SPAN_SEPARATOR};
use crate::domain::applications::types::{
    AnswerCell, Answered, CellText, CellValue, FoundAt, Judged, LintAnswer, LintUnknown, Plan,
    Purpose, Rendering, SpannedFinding,
};
use crate::domain::comments::service::comment;
use crate::domain::comments::types::{ByteSpan, Comment, CommentText, Documentation};
use crate::domain::contracts::constants::RENDERED;
use crate::domain::contracts::types::{
    ColumnText, DataRow, RowKey, RowNode, RowPlace, Syntax, SyntaxKind, WrittenValue,
};
use crate::domain::entries::types::ConditionName;
use crate::domain::findings::constants::UNKNOWN;
use crate::domain::findings::service::{check_id, error, finding, render, said};
use crate::domain::findings::types::{
    Finding, MessageText, Place, Placeholder, PlaceholderValue, RuleId,
};
use crate::domain::imports::types::{ImportColumn, ImportPlace};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::Identity;

/// A finding of the lint call rendered with the diagnostic of the rule it came from: its message and help with its
/// cells put in, and its place when its primary column is a span.
#[must_use]
pub fn answered(rendering: &Rendering, path: RepoPath, cells: &[AnswerCell]) -> Answered {
    let mut values: BTreeMap<Placeholder<'_>, PlaceholderValue> = BTreeMap::new();
    for cell in cells {
        let _: &mut PlaceholderValue = values
            .entry(Placeholder(&cell.name.0))
            .or_insert_with(|| written(&cell.value));
    }
    let located = cells
        .iter()
        .find(|cell| cell.name.0 == rendering.diagnostic.primary.0)
        .and_then(|cell| match &cell.value {
            CellValue::Span(span) => Some(FoundAt::Span(*span)),
            CellValue::File(_) | CellValue::Text(_) => None,
        })
        .unwrap_or(FoundAt::Whole);
    let message = render(&rendering.diagnostic.message, &values).0;
    let help = render(&rendering.diagnostic.help, &values).0;
    Answered {
        finding: finding(
            crate::domain::findings::service::of_rule(rendering.rule.clone()),
            path,
            Place::Whole,
            message,
            said(help),
        ),
        located,
    }
}

/// A cell's value as a message writes it: a file by its path, a span as the engine writes it, `start..end`, any other
/// value as its text.
fn written(value: &CellValue) -> PlaceholderValue {
    PlaceholderValue(match value {
        CellValue::File(path) => path.0.to_string(),
        CellValue::Span(span) => format!("{}{SPAN_SEPARATOR}{}", span.start.0, span.end.0),
        CellValue::Text(text) => text.0.to_string(),
    })
}

/// The value of the column headed `name` in a row, when the row has it.
fn column<'a>(found: &'a LintAnswer, name: Heading<'_>) -> Option<&'a CellValue> {
    found
        .cells
        .iter()
        .find(|cell| &*cell.name.0 == name.0)
        .map(|cell| &cell.value)
}

/// The text of the column headed `name` in a row, owned: empty when the row does not have it.
fn owned(found: &LintAnswer, name: Heading<'_>) -> Owned {
    Owned(column(found, name).map_or_else(Box::default, |value| written(value).0.into()))
}

/// The span of the column headed `name` in a row, when it holds one.
fn spanned(found: &LintAnswer, name: Heading<'_>) -> Option<ByteSpan> {
    match column(found, name)? {
        CellValue::Span(span) => Some(*span),
        CellValue::File(_) | CellValue::Text(_) => None,
    }
}

/// One row of a built-in comparison rule, kept with the rows of its file; a row without its node is counted as dropped,
/// so the contracts of its file are not compared blind.
fn syntax_row(judged: &mut Judged, syntax: Syntax, found: &LintAnswer) {
    let names = crate::domain::contracts::service::columns(syntax);
    let text = |name: ColumnText| {
        if name.0.is_empty() {
            Box::default()
        } else {
            owned(found, Heading(name.0)).0
        }
    };
    let Some(id) = column(found, Heading(names.id.0)) else {
        let _: bool = judged.dropped.insert(found.path.clone());
        return;
    };
    let row = DataRow {
        syntax,
        id: RowNode(written(id).0.into()),
        owner: RowNode(text(names.owner)),
        key: RowKey(text(names.key)),
        kind: SyntaxKind(text(names.kind)),
        text: WrittenValue(text(names.text)),
        span: spanned(found, Heading(SPAN)).map_or(RowPlace::Unplaced, RowPlace::At),
    };
    judged.data.entry(found.path.clone()).or_default().push(row);
}

/// The finding of an import the lint call could not read: none for a test, which no `imports` reads, for a file under
/// a `rules-exclude`, which only placement reads, for a template's rendered content, or for a file the workspace does
/// not govern.
fn import_unknown(plan: &Plan, missing: LintUnknown) -> Option<Finding> {
    // An import that could not be read decides nothing of a test, which no `imports` reads, nor of a file
    // under a `rules-exclude`, which only placement reads.
    let tests = missing
        .path
        .0
        .rsplit(PATH_SEPARATOR)
        .skip(1)
        .any(|part| part == TESTS);
    // A template's rendered content is given beside the disk to be compared, never built: that no crate root
    // reaches it decides nothing.
    let rendered = missing.message.0.contains(&format!("{RENDERED}/"))
        || missing.path.0.starts_with(RENDERED)
        || missing.path.0.contains(&format!("/{RENDERED}/"));
    // A relation read in part in the run as a whole is said of the lint call's root, a directory: its finding is
    // anchored at the workspace's own file, a path of the repository wherever it is checked out. One read in part in a
    // file the workspace does not govern, a package it depends on outside it, misses rows about that file only, which
    // no `imports` of the workspace reads.
    let whole = missing.path == plan.base;
    let governed = plan.files.contains(&missing.path);
    if !whole && !governed {
        return None;
    }
    if !tests && !rendered && !plan.excluded.contains(&missing.path) {
        let anchor = if whole {
            plan.anchors.first().cloned().unwrap_or_default()
        } else {
            missing.path
        };
        return Some(error(
            check_id(crate::domain::imports::constants::WORKSPACE_FILE, UNKNOWN),
            anchor,
            format!("imports could not be read: {}", missing.message.0),
        ));
    }
    None
}

/// What the lint call's `answers` and `unknowns` come to under `plan`: each rule's findings rendered and the comments of
/// the files that may hold an ignore read. What the conditions answered is the count call's to judge
/// (`crate::domain::judging`).
#[must_use]
pub fn judge(plan: &Plan, answers: Vec<LintAnswer>, unknowns: Vec<LintUnknown>) -> Judged {
    let mut judged = Judged::default();
    for found in answers {
        match plan.purposes.get(&found.rule) {
            Some(Purpose::Rule(rendering)) => {
                let answer = answered(rendering, found.path, &found.cells);
                match answer.located {
                    FoundAt::Whole => judged.findings.push(answer.finding),
                    FoundAt::Span(span) => judged.spans.push(SpannedFinding {
                        finding: answer.finding,
                        span,
                    }),
                }
            }
            Some(Purpose::Declarations(syntax)) => syntax_row(&mut judged, *syntax, &found),
            Some(Purpose::Imports(relation)) => {
                let cell = |name: ImportColumn<'_>| {
                    column(&found, Heading(name.0)).map(|value| CellText(written(value).0.into()))
                };
                let place =
                    spanned(&found, Heading(SPAN)).map_or(ImportPlace::Unplaced, ImportPlace::At);
                crate::domain::import_rows::service::keep(
                    &mut judged.imports,
                    *relation,
                    found.path.clone(),
                    &plan.base,
                    &cell,
                    place,
                );
            }
            Some(Purpose::Comments) => {
                if let Some(read) = comment_row(&found) {
                    judged.comments.entry(found.path).or_default().push(read);
                }
            }
            // A condition's answers are judged by the count query, never here.
            Some(Purpose::Condition { .. }) | None => {}
        }
    }
    for missing in unknowns {
        match plan.purposes.get(&missing.rule) {
            Some(Purpose::Rule(rendering)) => {
                judged
                    .findings
                    .push(unknown(&rendering.rule, missing.path, &missing.message));
            }
            Some(Purpose::Condition { name, identity, .. }) => {
                judged
                    .findings
                    .push(unanswered(identity, name, missing.path, &missing.message));
            }
            Some(Purpose::Imports(_)) => judged.findings.extend(import_unknown(plan, missing)),
            Some(Purpose::Comments | Purpose::Declarations(_)) | None => {}
        }
    }
    judged
}

/// The comment a row of the built-in comments rule answers: its span, whether it is documentation, and its text.
fn comment_row(found: &LintAnswer) -> Option<Comment> {
    let documentation = if owned(found, Heading(DOC)).0.as_ref() == RAISED {
        Documentation::Documents
    } else {
        Documentation::Plain
    };
    Some(comment(
        spanned(found, Heading(SPAN))?,
        documentation,
        CommentText(&owned(found, Heading(TEXT)).0),
    ))
}

/// A rule's `#unknown`: what it could not decide.
#[must_use]
pub fn unknown(rule: &RuleId, path: RepoPath, message: &MessageText) -> Finding {
    error(
        check_id(&rule.0, UNKNOWN),
        path,
        format!("`{}` could not decide: {}", rule.0, message.0),
    )
}

/// A condition's `#unknown`: what it could not answer.
#[must_use]
pub(crate) fn unanswered(
    identity: &Identity,
    name: &ConditionName,
    path: RepoPath,
    message: &MessageText,
) -> Finding {
    error(
        check_id(&identity.0, UNKNOWN),
        path,
        format!(
            "condition `{}` could not be answered by `{}`: {}",
            name.0, identity.0, message.0
        ),
    )
}
