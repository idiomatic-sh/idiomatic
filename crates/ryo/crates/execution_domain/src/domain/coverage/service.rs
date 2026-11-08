//! The `coverage` service: an operator that answers from the absence of rows (an antijoin, a difference, a count) cannot trust it
//! where a relation it reads is read only in part. It drops the rows that the missing ones could change, and the plan answers
//! that it could not decide.

use ryo_relations::domain::relations::constants::{SYNTAX_ERRORS, UNPARSED};
use ryo_relations::domain::relations::service::{class_of, node_file};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, Class, FileId, Missing, Relation, Value};

use super::constants::REPOSITORY;
use super::constants::{UNKNOWN_BECAUSE, UNKNOWN_IN};
use super::types::{
    CoverageScope, DeclaredFiles, FileOrdinal, Keying, KeyColumn, Lacking, LackingKind, Message, Partial,
    PartialPlace, Recovery,
};

use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::stored_facts::types::{PartialCause, PartialScope, ReadInPart};

/// Physical file ordinals affected by a partial project publication. An
/// unlocated cause applies to the entire named file set; no named file means
/// the caller must retain the project refusal instead of losing it in a report.
pub fn affected_ordinals(
    affected: &[Option<FileId>],
    file_count: DeclaredFiles,
) -> Result<Vec<FileOrdinal>, crate::domain::operating::errors::OperatingError> {
    use crate::domain::operating::errors::OperatingError;
    let refuse = || OperatingError::ObservationIncomplete {
        reason: super::constants::UNPLACEABLE_COVERAGE.into(),
    };
    if affected.is_empty() || file_count.0 == 0 {
        return Err(refuse());
    }
    let unlocated = affected.iter().any(Option::is_none);
    let mut ordinals = affected
        .iter()
        .filter_map(|file| file.as_ref())
        .map(|file| {
            let ordinal = usize::try_from(file.0).map_err(|_| refuse())?;
            (ordinal < file_count.0)
                .then_some(ordinal)
                .ok_or_else(refuse)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if unlocated {
        return Ok((0..file_count.0).map(FileOrdinal).collect());
    }
    ordinals.sort_unstable();
    ordinals.dedup();
    Ok(ordinals.into_iter().map(FileOrdinal).collect())
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

/// Whether a column holds the files or the nodes of files: the one lineage rule of when a column states its row's file.
fn is_file_kind(kind: CellKind) -> Keying {
    if ryo_values::domain::lineage::service::anchor_of(kind)
        == ryo_values::domain::lineage::types::ColumnAnchor::InFile
    {
        Keying::ByFile
    } else {
        Keying::NotByFile
    }
}

/// The column of the answer that says which file a row is in, when the operator keys by one.
fn file_key(operation: &Operation, probed: &[CellKind], out: &Table) -> Option<KeyColumn> {
    match operation {
        // The answer of an antijoin is its left rows, and a key pair holds equal values, so the left key a file or node lies in
        // is read from the answer's own kinds; `probed` is the right operand's schema in one caller and the answer's in the other.
        Operation::Antijoin { left_on, .. } => left_on
            .iter()
            .find(|left| {
                out.kinds
                    .get(usize::try_from(left.0).unwrap_or(usize::MAX))
                    .is_some_and(|kind| is_file_kind(*kind) == Keying::ByFile)
            })
            .map(|left| KeyColumn(usize::try_from(left.0).unwrap_or(usize::MAX))),
        Operation::Difference { .. } => out
            .kinds
            .iter()
            .enumerate()
            .find(|(place, _)| {
                probed
                    .get(*place)
                    .is_some_and(|kind| is_file_kind(*kind) == Keying::ByFile)
            })
            .map(|(place, _)| KeyColumn(place)),
        // A grouped answer lists its keys first, so the key at `place` of the answer is the one the output
        // schema says is a file or a node. `probed` is the schema of the operand in one caller and of the
        // answer in the other; only the answer's own schema is positioned the same in both.
        Operation::GroupCount { keys, .. } | Operation::GroupDigest { keys, .. } => out
            .kinds
            .iter()
            .take(keys.len())
            .position(|kind| is_file_kind(*kind) == Keying::ByFile)
            .map(KeyColumn),
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
    let Some(KeyColumn(column)) = keyed.filter(|column| column.0 < width && !everywhere) else {
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

/// Drops from `out` the rows that missing rows of the inapplicable `files` (sorted, once each) could change. A file no language
/// reads, or whose syntax a rule skipped, is out of scope for a count not keyed by file, so such a count withholds nothing on its
/// account; an absence check (an antijoin, a difference) still treats the file as unavailable. An operator that answers from no
/// absence of rows (a join, a union, a selection, a hosted relation) only lacks the rows the file would have added, so the rows it
/// has stand and none is dropped.
pub(crate) fn drop_inapplicable(
    operation: &Operation,
    probed: &[CellKind],
    out: &mut Table,
    files: &[FileId],
) {
    if crate::domain::operator::contract::service::semantics(
        ryo_values::domain::plans::service::kind_of(operation),
    )
    .absence
    .0
        == 0
    {
        return;
    }
    let width = out.kinds.len();
    let counts = matches!(
        operation,
        Operation::GroupCount { .. } | Operation::GroupDigest { .. }
    );
    if counts && file_key(operation, probed, out).is_none_or(|column| column.0 >= width) {
        return;
    }
    let keyed: Vec<Option<FileId>> = files.iter().copied().map(Some).collect();
    drop_uncertain(operation, probed, out, &keyed);
}

/// The relations of a file that are read in part: those the reader said so of, and, when the whole file is only a recovery of what was
/// written (`everything` says how), every relation of code that was asked for.
#[must_use]
pub fn partial_of(
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
                PartialScope::OneFile(file) => PartialPlace::File(FileId(file.0)),
                PartialScope::Repository => PartialPlace::Repository,
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
                at: PartialPlace::Repository,
            });
        }
    }
    partial.sort();
    partial.dedup();
    partial
}

/// What an operator that trusts an absence of rows says it could not decide, for a relation read in part in a file.
#[must_use]
pub fn unknown_message(partial: &Partial, scope: CoverageScope<'_>) -> Message {
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
