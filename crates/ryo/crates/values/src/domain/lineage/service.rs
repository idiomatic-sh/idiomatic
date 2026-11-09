//! The `lineage` service: free functions named by verb over where the rows of a step come from, with no IO and no ports.
//!
//! A relation's rows are partitioned by their file column and its nodes lie in that file, and a per-file step reads only the rows
//! of one file, so each of their file, node and span columns lies in the file its row comes from. A cross-file step keeps that only
//! through what it does: a row it passes on keeps its columns, and a join keeps it when a key pairs a column of each operand
//! that lies in its row's file, so the two rows it combines come from one file, or when every file or node column of its answer
//! comes from one operand, so the answer holds the files and nodes of that operand's row alone.
//!
//! The one rule of when a column states the file of its row: its value carries a file, so it is a file, or a node, whose number
//! holds its file; or it is a span, a place in its row's file. Equal spans of two files are not the same place, so a join keyed on
//! spans pairs rows of one file only (the operators' join drops a pair of rows of two files), and a key of spans proves the rows it
//! matches come from one file. Two rows that meet on keys come from one file when the keys pair, at one position, a column of each
//! operand that states its row's file.
use super::types::{ColumnAnchor, FileNaming, KeyPairing, Lineage};
use ryo_relations::domain::relations::types::CellKind;
use crate::domain::plan_parts::types::{ColumnIndex, JoinColumn, RepeatedRows, Side};
use crate::domain::plans::types::Operation;

/// Whether a column of this kind, of rows that each come from one file, lies in its row's file: a file or a node does, since its
/// value carries its file; a span does, since it is a place in its row's file and a join on spans pairs rows of one file only; no
/// other value names a file.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::lineage::service::anchor_of;
/// use ryo_values::domain::lineage::types::ColumnAnchor;
/// assert_eq!(anchor_of(CellKind::Node), ColumnAnchor::InFile);
/// assert_eq!(anchor_of(CellKind::Span), ColumnAnchor::InFile);
/// assert_eq!(anchor_of(CellKind::Number), ColumnAnchor::Elsewhere);
/// ```
#[must_use]
pub fn anchor_of(kind: CellKind) -> ColumnAnchor {
    if matches!(kind, CellKind::File | CellKind::Node | CellKind::Span) {
        ColumnAnchor::InFile
    } else {
        ColumnAnchor::Elsewhere
    }
}

/// Whether a value of this kind names its file: a file or a node does; a span lies in its row's file (`anchor_of`) but holds only
/// a start and an end, so the file cannot be read from it.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::lineage::service::naming_of;
/// use ryo_values::domain::lineage::types::FileNaming;
/// assert_eq!(naming_of(CellKind::File), FileNaming::Named);
/// assert_eq!(naming_of(CellKind::Span), FileNaming::Unnamed);
/// ```
#[must_use]
pub fn naming_of(kind: CellKind) -> FileNaming {
    if matches!(kind, CellKind::File | CellKind::Node) {
        FileNaming::Named
    } else {
        FileNaming::Unnamed
    }
}

/// The lineage of rows that each come from one file, of these kinds: a relation's, or a per-file step's.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::lineage::service::one_file;
/// use ryo_values::domain::lineage::types::{ColumnAnchor, Lineage};
/// let lineage = one_file(&[CellKind::File, CellKind::Number]);
/// assert_eq!(lineage, Lineage::OneFile([ColumnAnchor::InFile, ColumnAnchor::Elsewhere].into()));
/// ```
#[must_use]
pub fn one_file(kinds: &[CellKind]) -> Lineage {
    Lineage::OneFile(kinds.iter().map(|kind| anchor_of(*kind)).collect())
}

/// The lineage of a hosted relation's rows, of these kinds. A hosted row is read from no file, so the file it is taken to come
/// from is the one its file column names: with exactly one file column, that column lies in its row's file and nothing else does
/// (a node may lie in any file). With no file column, or more than one, no column says which file a row comes from.
fn hosted(kinds: &[CellKind]) -> Lineage {
    if kinds.iter().filter(|kind| **kind == CellKind::File).count() != 1 {
        return Lineage::Mixed;
    }
    Lineage::OneFile(
        kinds
            .iter()
            .map(|kind| {
                if *kind == CellKind::File {
                    ColumnAnchor::InFile
                } else {
                    ColumnAnchor::Elsewhere
                }
            })
            .collect(),
    )
}

/// Whether the column `column` lies in the file its row comes from: a column the lineage does not name proves nothing.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::lineage::service::{anchor_at, one_file};
/// use ryo_values::domain::lineage::types::{ColumnAnchor, Lineage};
/// let lineage = one_file(&[CellKind::File, CellKind::Number]);
/// assert_eq!(anchor_at(&lineage, ColumnIndex(0)), ColumnAnchor::InFile);
/// assert_eq!(anchor_at(&lineage, ColumnIndex(1)), ColumnAnchor::Elsewhere);
/// assert_eq!(anchor_at(&Lineage::Mixed, ColumnIndex(0)), ColumnAnchor::Elsewhere);
/// ```
#[must_use]
pub fn anchor_at(lineage: &Lineage, column: ColumnIndex) -> ColumnAnchor {
    match lineage {
        Lineage::OneFile(anchors)
            if anchors.get(column.0 as usize) == Some(&ColumnAnchor::InFile) =>
        {
            ColumnAnchor::InFile
        }
        Lineage::OneFile(_) | Lineage::Mixed => ColumnAnchor::Elsewhere,
    }
}

/// Whether the right operand's column a join names at `column` through its view lies in its row's file: the view's column at
/// that place, the column itself without a view.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::lineage::service::{anchor_through, one_file};
/// use ryo_values::domain::lineage::types::ColumnAnchor;
/// let lineage = one_file(&[CellKind::Number, CellKind::Node]);
/// // Through a view, column 0 of the join is the operand's column 1.
/// assert_eq!(anchor_through(&lineage, &[ColumnIndex(1)], ColumnIndex(0)), ColumnAnchor::InFile);
/// assert_eq!(anchor_through(&lineage, &[], ColumnIndex(0)), ColumnAnchor::Elsewhere);
/// ```
#[must_use]
pub fn anchor_through(
    lineage: &Lineage,
    view: &[ColumnIndex],
    column: ColumnIndex,
) -> ColumnAnchor {
    if view.is_empty() {
        return anchor_at(lineage, column);
    }
    match view.get(column.0 as usize) {
        Some(own) => anchor_at(lineage, *own),
        None => ColumnAnchor::Elsewhere,
    }
}

/// Whether the keys of two operands pair a column of each that lies in its row's file, the right keys read through `view`. A key of
/// spans does: a join on spans pairs rows of one file only.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::lineage::service::{one_file, pairing};
/// use ryo_values::domain::lineage::types::KeyPairing;
/// let rows = one_file(&[CellKind::File, CellKind::Span, CellKind::Number]);
/// assert_eq!(pairing(&rows, &[ColumnIndex(0)], &rows, &[ColumnIndex(0)], &[]), KeyPairing::SameFile);
/// // A key of spans pairs rows of one file only.
/// assert_eq!(pairing(&rows, &[ColumnIndex(1)], &rows, &[ColumnIndex(1)], &[]), KeyPairing::SameFile);
/// // A key of numbers alone proves nothing about the files.
/// assert_eq!(pairing(&rows, &[ColumnIndex(2)], &rows, &[ColumnIndex(2)], &[]), KeyPairing::Unproven);
/// ```
#[must_use]
pub fn pairing(
    left: &Lineage,
    left_on: &[ColumnIndex],
    right: &Lineage,
    right_on: &[ColumnIndex],
    view: &[ColumnIndex],
) -> KeyPairing {
    if left_on.iter().zip(right_on).any(|(l, r)| {
        anchor_at(left, *l) == ColumnAnchor::InFile
            && anchor_through(right, view, *r) == ColumnAnchor::InFile
    }) {
        KeyPairing::SameFile
    } else {
        KeyPairing::Unproven
    }
}

/// The lineage of the columns `columns` of the rows of `lineage`, in that order.
fn picked(lineage: &Lineage, columns: &[ColumnIndex]) -> Lineage {
    match lineage {
        Lineage::OneFile(_) => Lineage::OneFile(
            columns
                .iter()
                .map(|column| anchor_at(lineage, *column))
                .collect(),
        ),
        Lineage::Mixed => Lineage::Mixed,
    }
}

/// The lineage of rows of the kinds `kinds` that keep the columns of `lineage` first and append columns of their own.
fn widened(lineage: Lineage, kinds: &[CellKind]) -> Lineage {
    match lineage {
        Lineage::OneFile(anchors) => {
            let mut anchors = anchors.into_vec();
            anchors.resize(kinds.len(), ColumnAnchor::Elsewhere);
            Lineage::OneFile(anchors.into_boxed_slice())
        }
        Lineage::Mixed => Lineage::Mixed,
    }
}

/// Whether a step that merges the rows of `lineage` that hold equal `keys` (a group, a closure's partitions) keeps rows of two files
/// apart: a key lies in its row's file and names it, so the key of the answer, at the same place among its first columns `kinds`,
/// differs between files. A key of spans alone does not: equal spans of two files would merge, as the merge compares values.
fn keyed_by_file(lineage: &Lineage, keys: &[ColumnIndex], kinds: &[CellKind]) -> ColumnAnchor {
    if keys.iter().zip(kinds).any(|(key, kind)| {
        anchor_at(lineage, *key) == ColumnAnchor::InFile && naming_of(*kind) == FileNaming::Named
    }) {
        ColumnAnchor::InFile
    } else {
        ColumnAnchor::Elsewhere
    }
}

/// The lineage of the rows of `lineage`, of the kinds `kinds`, once rows that repeat are kept once: a repeated row stays of one file
/// when a column of it lies in its row's file and names it. Otherwise equal rows of two files merge, and only spans can lie in their
/// row's file, which the merged row no longer has.
fn kept_apart(lineage: Lineage, kinds: &[CellKind]) -> Lineage {
    let Lineage::OneFile(anchors) = lineage else {
        return Lineage::Mixed;
    };
    if anchors.iter().zip(kinds).any(|(anchor, kind)| {
        *anchor == ColumnAnchor::InFile && naming_of(*kind) == FileNaming::Named
    }) {
        return Lineage::OneFile(anchors);
    }
    Lineage::OneFile(anchors.iter().map(|_| ColumnAnchor::Elsewhere).collect())
}

/// The one operand every file or node column of a join's answer comes from, when there is one: its rows' files are the answer's.
fn file_side(output: &[JoinColumn], kinds: &[CellKind]) -> Option<Side> {
    let mut sides = output
        .iter()
        .zip(kinds)
        .filter(|(_, kind)| naming_of(**kind) == FileNaming::Named)
        .map(|(column, _)| column.side);
    let first = sides.next()?;
    sides.all(|side| side == first).then_some(first)
}

/// The lineage of the answer of a join or an ordered match, whose `output` names columns of either operand; `kinds` are the
/// kinds of the answer's columns. Without a key pairing a column of each operand that lies in its row's file, the answer still
/// lies in one file when every file or node column of it comes from one operand: each row then holds the files and nodes of that
/// operand's row alone, and the other operand's columns hold no file or node.
fn joined(
    left: &Lineage,
    left_on: &[ColumnIndex],
    right: &Lineage,
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    kinds: &[CellKind],
) -> Lineage {
    let anchor = |column: &JoinColumn| match column.side {
        Side::Left => anchor_at(left, column.column),
        Side::Right => anchor_through(right, view, column.column),
    };
    if pairing(left, left_on, right, right_on, view) == KeyPairing::SameFile {
        return Lineage::OneFile(output.iter().map(anchor).collect());
    }
    let Some(side) = file_side(output, kinds) else {
        return Lineage::Mixed;
    };
    if output.len() != kinds.len()
        || matches!(
            if side == Side::Left { left } else { right },
            Lineage::Mixed
        )
    {
        return Lineage::Mixed;
    }
    Lineage::OneFile(
        output
            .iter()
            .map(|column| {
                if column.side == side {
                    anchor(column)
                } else {
                    ColumnAnchor::Elsewhere
                }
            })
            .collect(),
    )
}

/// The lineage of rows that are rows of either operand: a column lies in its row's file only when it does in both.
fn either(left: &Lineage, right: &Lineage) -> Lineage {
    match (left, right) {
        (Lineage::OneFile(first), Lineage::OneFile(second)) => Lineage::OneFile(
            first
                .iter()
                .zip(second.iter())
                .map(|(one, other)| {
                    if *one == ColumnAnchor::InFile && *other == ColumnAnchor::InFile {
                        ColumnAnchor::InFile
                    } else {
                        ColumnAnchor::Elsewhere
                    }
                })
                .collect(),
        ),
        _ => Lineage::Mixed,
    }
}

/// The lineage of the rows of a step, from the lineage of its operands in order; `kinds` are the kinds of its columns.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_relations::domain::relations::types::Relation;
/// use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
/// use ryo_values::domain::plans::types::Operation;
/// use ryo_values::domain::lineage::service::{of_step, one_file};
/// use ryo_values::domain::lineage::types::{ColumnAnchor, Lineage};
/// let operand = one_file(&[CellKind::Number, CellKind::File]);
/// let select = Operation::Select {
///     from: Input::Relation(Relation::Function),
///     comparisons: vec![],
///     kept: vec![ColumnIndex(1)],
/// };
/// let kept = of_step(&select, &[operand], &[CellKind::File]);
/// assert_eq!(kept, Lineage::OneFile([ColumnAnchor::InFile].into()));
/// ```
#[must_use]
pub fn of_step(operation: &Operation, operands: &[Lineage], kinds: &[CellKind]) -> Lineage {
    let first = operands.first().cloned().unwrap_or_default();
    let second = operands.get(1).cloned().unwrap_or_default();
    match operation {
        Operation::Select { kept, .. } => {
            if kept.is_empty() {
                first
            } else {
                picked(&first, kept)
            }
        }
        Operation::Project {
            columns,
            distinct: RepeatedRows::Dropped,
            ..
        } => kept_apart(picked(&first, columns), kinds),
        Operation::Project {
            columns,
            distinct: RepeatedRows::Kept,
            ..
        } => picked(&first, columns),
        Operation::Extend { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Replace { .. }
        | Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. } => widened(first, kinds),
        Operation::Antijoin { .. }
        | Operation::Difference { .. }
        | Operation::Intersection { .. } => first,
        Operation::Union { .. } => either(&first, &second),
        Operation::Join {
            left_on,
            right_on,
            output,
            view,
            reach: None,
            ..
        } => joined(&first, left_on, &second, right_on, output, view, kinds),
        Operation::OrderedMatch {
            left_on,
            right_on,
            output,
            ..
        } => joined(&first, left_on, &second, right_on, output, &[], kinds),
        Operation::GroupCount { keys, .. } | Operation::GroupDigest { keys, .. } => {
            if keyed_by_file(&first, keys, kinds) == ColumnAnchor::InFile {
                widened(picked(&first, keys), kinds)
            } else {
                Lineage::Mixed
            }
        }
        Operation::Closure {
            keys,
            source,
            destination,
            ..
        } => {
            if keyed_by_file(&first, keys, kinds) == ColumnAnchor::InFile {
                let mut columns = keys.clone();
                columns.extend([*source, *destination]);
                picked(&first, &columns)
            } else {
                Lineage::Mixed
            }
        }
        Operation::Join { reach: Some(_), .. }
        | Operation::Axis { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. } => Lineage::Mixed,
        Operation::Hosted { .. } => hosted(kinds),
    }
}
