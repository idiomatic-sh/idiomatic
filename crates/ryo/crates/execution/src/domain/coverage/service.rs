//! The `coverage` service: an operator that answers from the absence of rows (an antijoin, a difference, a count) cannot trust it
//! where a relation it reads is read only in part. It drops the rows that the missing ones could change, and the plan answers
//! that it could not decide.

use ryo_relations::domain::relations::constants::{SYNTAX_ERRORS, UNPARSED};
use ryo_relations::domain::relations::service::{class_of, node_file};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, Class, FileId, Missing, Relation, Value};

use super::constants::REPOSITORY;
use super::constants::{UNKNOWN_BECAUSE, UNKNOWN_IN};
use super::types::{CoverageScope, Keying, Lacking, LackingKind, Message, Partial, Recovery};

use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::stored_facts::types::{PartialCause, PartialScope, ReadInPart};

/// Physical file ordinals affected by a partial project publication. An
/// unlocated cause applies to the entire named file set; no named file means
/// the caller must retain the project refusal instead of losing it in a report.
pub(crate) fn affected_ordinals(
    affected: &[Option<FileId>],
    file_count: usize,
) -> Result<Vec<usize>, crate::domain::operating::errors::OperatingError> {
    use crate::domain::operating::errors::OperatingError;
    let refuse = || OperatingError::ObservationIncomplete {
        reason: "partial project coverage cannot be placed on its declared files".into(),
    };
    if affected.is_empty() || file_count == 0 {
        return Err(refuse());
    }
    let unlocated = affected.iter().any(Option::is_none);
    let mut ordinals = affected
        .iter()
        .filter_map(|file| file.as_ref())
        .map(|file| {
            let ordinal = usize::try_from(file.0).map_err(|_| refuse())?;
            (ordinal < file_count).then_some(ordinal).ok_or_else(refuse)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if unlocated {
        return Ok((0..file_count).collect());
    }
    ordinals.sort_unstable();
    ordinals.dedup();
    Ok(ordinals)
}

/// The file a value lies in, when it is a file or a node.
fn file_of(value: Value) -> Option<FileId> {
    match value {
        Value::File(file) => Some(file),
        Value::Node(node) => Some(node_file(node)),
        Value::Symbol(_)
        | Value::Name(_)
        | Value::Span(_)
        | Value::Number(Integer(_))
        | Value::Flag(_) => None,
    }
}

/// Whether a column holds the files or the nodes of files.
fn is_file_kind(kind: CellKind) -> Keying {
    if matches!(kind, CellKind::File | CellKind::Node) {
        Keying::ByFile
    } else {
        Keying::NotByFile
    }
}

/// The column of the answer that says which file a row is in, when the operator keys by one.
fn file_key(operation: &Operation, probed: &[CellKind], out: &Table) -> Option<usize> {
    match operation {
        Operation::Antijoin {
            left_on, right_on, ..
        } => left_on
            .iter()
            .zip(right_on)
            .find(|(_, right)| {
                probed
                    .get(usize::try_from(right.0).unwrap_or(usize::MAX))
                    .is_some_and(|kind| is_file_kind(*kind) == Keying::ByFile)
            })
            .map(|(left, _)| usize::try_from(left.0).unwrap_or(usize::MAX)),
        Operation::Difference { .. } => out
            .kinds
            .iter()
            .enumerate()
            .find(|(place, _)| {
                probed
                    .get(*place)
                    .is_some_and(|kind| is_file_kind(*kind) == Keying::ByFile)
            })
            .map(|(place, _)| place),
        // A grouped answer lists its keys first, so the key at `place` of the answer is the one the output
        // schema says is a file or a node. `probed` is the schema of the operand in one caller and of the
        // answer in the other; only the answer's own schema is positioned the same in both.
        Operation::GroupCount { keys, .. } | Operation::GroupDigest { keys, .. } => out
            .kinds
            .iter()
            .take(keys.len())
            .position(|kind| is_file_kind(*kind) == Keying::ByFile),
        Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    }
}

/// Drops from `out`, the rows an operator made from the absence of rows, that the missing ones of `files` could change. A row
/// keyed by a file or a node is uncertain only if its file is among them; a row with no such key, whatever file the missing rows are
/// in, is. `probed` is the columns of the input whose absence of rows the operator trusts.
pub(crate) fn drop_uncertain(
    operation: &Operation,
    probed: &[CellKind],
    out: &mut Table,
    files: &[Option<FileId>],
) {
    if files.is_empty() {
        return;
    }
    let width = out.kinds.len();
    // A relation read in part in the repository as a whole could have a missing row anywhere.
    let everywhere = files.contains(&None);
    let keyed = file_key(operation, probed, out);
    let Some(column) = keyed.filter(|column| *column < width && !everywhere) else {
        out.cells.clear();
        out.count = ryo_values::domain::rows::types::RowCount(0);
        return;
    };
    let mut kept: Vec<Value> = Vec::with_capacity(out.cells.len());
    let mut rows = 0_usize;
    for row in out.cells.chunks(width) {
        let uncertain = row
            .get(column)
            .copied()
            .and_then(file_of)
            .is_none_or(|file| files.binary_search(&Some(file)).is_ok());
        if !uncertain {
            kept.extend_from_slice(row);
            rows += 1;
        }
    }
    out.cells = kept;
    out.count = ryo_values::domain::rows::types::RowCount(rows);
}

/// Drops from `out` the rows keyed to the inapplicable `files` (sorted, once each). A file no language reads, or whose syntax a rule
/// skipped, is out of scope for an answer not keyed by file, so such an answer withholds nothing on its account.
pub(crate) fn drop_inapplicable(
    operation: &Operation,
    probed: &[CellKind],
    out: &mut Table,
    files: &[FileId],
) {
    let width = out.kinds.len();
    if file_key(operation, probed, out).is_none_or(|column| column >= width) {
        return;
    }
    let keyed: Vec<Option<FileId>> = files.iter().copied().map(Some).collect();
    drop_uncertain(operation, probed, out, &keyed);
}

/// The relations of a file that are read in part: those the reader said so of, and, when the whole file is only a recovery of what was
/// written (`everything` says how), every relation of code that was asked for.
#[must_use]
pub(crate) fn partial_of(
    said: &[ReadInPart],
    everything: Recovery,
    wanted: &[Relation],
) -> Vec<Partial> {
    let mut partial: Vec<Partial> = said
        .iter()
        .map(|each| Partial {
            relation: each.relation,
            cause: PartialCause(each.cause.0.clone()),
            at: match each.at {
                PartialScope::OneFile(file) => Some(FileId(file.0)),
                PartialScope::Repository => None,
            },
            lacking: match &each.lacking {
                Missing::Any => Lacking::Any,
                Missing::Kind(kind) => Lacking::Kind(LackingKind(kind.0.into())),
            },
        })
        .collect();
    let cause = match everything {
        Recovery::Whole => None,
        Recovery::SyntaxErrors | Recovery::LexicallyCompleteSyntaxErrors => Some(SYNTAX_ERRORS),
        Recovery::Unparsed => Some(UNPARSED),
    };
    if let Some(cause) = cause {
        for relation in wanted.iter().filter(|each| match everything {
            Recovery::SyntaxErrors | Recovery::Unparsed => matches!(
                class_of(**each),
                Class::Code | Class::Literal | Class::Document | Class::Lexical
            ),
            Recovery::LexicallyCompleteSyntaxErrors => matches!(
                class_of(**each),
                Class::Code | Class::Literal | Class::Document
            ),
            Recovery::Whole => false,
        }) {
            partial.push(Partial {
                relation: *relation,
                cause: PartialCause(cause.into()),
                lacking: Lacking::Any,
                at: None,
            });
        }
    }
    partial.sort();
    partial.dedup();
    partial
}

/// What an operator that trusts an absence of rows says it could not decide, for a relation read in part in a file.
#[must_use]
pub(crate) fn unknown_message(partial: &Partial, scope: CoverageScope<'_>) -> Message {
    let path: &str = match scope {
        CoverageScope::Repository => REPOSITORY,
        CoverageScope::File(file) => &file.0,
    };
    Message(
        [
            ryo_relations::domain::relations::service::name(partial.relation).0,
            UNKNOWN_IN,
            path,
            UNKNOWN_BECAUSE,
            &*partial.cause.0,
        ]
        .concat()
        .into(),
    )
}

use ryo_values::domain::rows::types::Table;
