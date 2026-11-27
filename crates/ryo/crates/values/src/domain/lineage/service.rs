//! The `lineage` service: free functions named by verb over where the rows of a step come from, with no IO and no ports.
//!
//! A relation's rows are partitioned by their file column and its nodes lie in that file, and a per-file step reads only the rows
//! of one file, so each of their file and node columns lies in the file its row comes from. A cross-file step keeps that only
//! through what it does: a row it passes on keeps its columns, and a join keeps it when a key pairs a column of each operand
//! that lies in its row's file, so the two rows it combines come from one file, or when every file or node column of its answer
//! comes from one operand, so the answer holds the files and nodes of that operand's row alone.
//!
//! The one rule of when a column states the file of its row: its value carries a file, so it is a file, or a node, whose number
//! holds its file. A span holds only a start and an end, so it states no file: equal spans of two files are equal, and a key of
//! spans alone never proves that the rows it matches come from one file. Two rows that meet on keys come from one file only when
//! the keys pair, at one position, a column of each operand that states its row's file.
use super::types::{ColumnAnchor, KeyPairing, Lineage};
use crate::domain::plan_parts::types::{ColumnIndex};
use ryo_relations::domain::relations::types::CellKind;

/// Whether a column of this kind, of rows that each come from one file, lies in its row's file: a file or a node does, since its
/// value carries its file; a span does not, since it holds only a start and an end; no other value names a file.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::lineage::service::anchor_of;
/// use ryo_values::domain::lineage::types::ColumnAnchor;
/// assert_eq!(anchor_of(CellKind::Node), ColumnAnchor::InFile);
/// assert_eq!(anchor_of(CellKind::Span), ColumnAnchor::Elsewhere);
/// ```
#[must_use]
pub fn anchor_of(kind: CellKind) -> ColumnAnchor {
    if matches!(kind, CellKind::File | CellKind::Node) {
        ColumnAnchor::InFile
    } else {
        ColumnAnchor::Elsewhere
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


/// Whether the column `column` lies in the file its row comes from: a column the lineage does not name proves nothing.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::lineage::service::{anchor_at, one_file};
/// use ryo_values::domain::lineage::types::{ColumnAnchor, Lineage};
/// let lineage = one_file(&[CellKind::File, CellKind::Span]);
/// assert_eq!(anchor_at(&lineage, ColumnIndex(0)), ColumnAnchor::InFile);
/// assert_eq!(anchor_at(&lineage, ColumnIndex(1)), ColumnAnchor::Elsewhere);
/// assert_eq!(anchor_at(&Lineage::Mixed, ColumnIndex(0)), ColumnAnchor::Elsewhere);
/// ```
#[must_use]
pub fn anchor_at(lineage: &Lineage, column: ColumnIndex) -> ColumnAnchor {
    match lineage {
        Lineage::OneFile(anchors)
            if usize::try_from(column.0)
                .is_ok_and(|at| anchors.get(at) == Some(&ColumnAnchor::InFile)) =>
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
/// let lineage = one_file(&[CellKind::Span, CellKind::Node]);
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
    crate::domain::plan_parts::service::view_source(view, column)
        .map_or(ColumnAnchor::Elsewhere, |own| anchor_at(lineage, own))
}

/// Whether the keys of two operands pair a column of each that lies in its row's file, the right keys read through `view`.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::lineage::service::{one_file, pairing};
/// use ryo_values::domain::lineage::types::KeyPairing;
/// let rows = one_file(&[CellKind::File, CellKind::Span]);
/// assert_eq!(pairing(&rows, &[ColumnIndex(0)], &rows, &[ColumnIndex(0)], &[]), KeyPairing::SameFile);
/// // A key of spans alone proves nothing about the files.
/// assert_eq!(pairing(&rows, &[ColumnIndex(1)], &rows, &[ColumnIndex(1)], &[]), KeyPairing::Unproven);
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






