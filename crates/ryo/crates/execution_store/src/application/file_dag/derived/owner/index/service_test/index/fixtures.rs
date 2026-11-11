//! The support of the `index` cases.
use super::super::super::types::ByteLimit;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::derived::owner::fixtures::graph::GraphFixture;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(super) fn new(wanted: &[Relation], partials: &[Partial], limit: ByteLimit) -> GraphFixture {
    GraphFixture::new(wanted, partials, limit.0)
}

pub(super) fn acquire(
    owner: &GraphFixture,
    relation: Relation,
    rows: ColumnsView<'_>,
    on: &[ColumnIndex],
) -> Result<AcquiredView, OperatingError> {
    owner.index(relation, &Rows::Stored(rows), on)
}

pub(super) fn equality(view: &AcquiredView) -> &KeyIndex {
    let DerivedView::Equality(index) = view.view().expect("valid equality payload") else {
        panic!("equality payload")
    };
    index
}
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::service::matching_stored;
use ryo_relations::domain::relations::types::{CellKind, FactColumn, Value as CellValue};
use ryo_values::domain::rows::types::RowIndex;
use std::borrow::Cow;

/// The index the owner is asked for, and around what.
#[derive(Clone, Copy, Debug)]
pub(super) enum Asked {
    /// The rows `(2, 20), (1, 10), (2, 21), (2, 22)` on the first column, twice; then key 2.
    Duplicates,
    /// One population under four key orders and relations, then under a wrong schema and a
    /// shorter population, then the shorter one in a fresh owner.
    DistinctKeys,
    /// A population whose relation has a physical partial, as Function and as Identifier.
    Partial,
    /// 1024 rows under a limit that leaves no room past the owner's metadata.
    NoRoom,
    /// The rows `1, 1` in an owner that is dropped, then in a next owner.
    DroppedOwner,
    /// The flags `1, 2, 0` on a column that does not exist, then twice on the first; then the keys
    /// raised and the number 1.
    Flags,
    /// The rows `1, 1, 1` on keys of widths 1 to 64, all held while the owner drops.
    Widths(usize),
    /// One symbol column, reused after 128 derived symbols are interned.
    Symbols(u32),
    /// The rows `(0, 0), (1, c), (0, 0)`, where `(1, c)` hashes as `(0, 0)`, twice; then key
    /// `(0, 0)`.
    Collision,
}

/// What an index asked for twice answered: how the second was constructed and the rows a key
/// matches in it.
#[derive(Debug, PartialEq)]
pub(super) struct Reused {
    pub(super) construction: Construction,
    pub(super) matched: Cow<'static, [RowIndex]>,
}

/// What the asks observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    /// How the first was constructed, whether both share storage and the indexes owned.
    Duplicates {
        first: Construction,
        one_storage: bool,
        indexes: usize,
        reused: Reused,
    },
    /// How each of the four keys was constructed, the two refusals, the indexes owned, and how
    /// the shorter population was constructed in a fresh owner.
    DistinctKeys {
        keys: [Construction; 4],
        wrong_schema: Option<OperatingError>,
        shorter: Option<OperatingError>,
        indexes: usize,
        fresh: Construction,
    },
    /// Per relation, how it was constructed and its rows; and the indexes owned.
    Partial {
        local: [(Construction, usize); 2],
        indexes: usize,
    },
    /// How it was constructed, its rows, the indexes owned, whether bytes stayed within the
    /// limit, whether required bytes were charged, whether the backing outlived its handle, and
    /// the required bytes after the generation ends.
    NoRoom {
        construction: Construction,
        rows: usize,
        indexes: usize,
        within_limit: bool,
        required: bool,
        backing_outlived: bool,
        required_after_clear: usize,
    },
    /// Whether the dropped owner's index survived, and how the next was constructed and the
    /// indexes it owns.
    Dropped {
        survived: bool,
        next: Construction,
        indexes: usize,
    },
    /// The refusal of the missing column, the indexes after it, and the rows each key
    /// matches on reuse.
    Flags {
        refused: Option<OperatingError>,
        indexes: usize,
        raised: Cow<'static, [RowIndex]>,
        number: Cow<'static, [RowIndex]>,
    },
    /// In how many widths bytes stayed within the limit and required bytes were charged, and
    /// whether the backing survived the owner and then its last consumer.
    Widths {
        within_limit: usize,
        required: usize,
        survived_owner: bool,
        survived_consumers: bool,
    },
    Reused(Reused),
    /// Whether the two hashes collide, and the reuse.
    Collision {
        collide: bool,
        reused: Reused,
    },
}

pub(super) fn matched(
    view: ColumnsView<'_>,
    on: &[ColumnIndex],
    index: &KeyIndex,
    key: &[CellValue],
) -> Cow<'static, [RowIndex]> {
    Cow::Owned(
        matching_stored(
            view,
            on,
            index,
            key,
            #[cfg(feature = "diagnostic-row-progress")]
            &ryo_execution_domain::domain::operating::types::IgnoredProgress,
        )
        .copied()
        .collect(),
    )
}

pub(super) fn numbers<'a>(
    kinds: &'static [CellKind],
    columns: &'a [FactColumn],
) -> ColumnsView<'a> {
    ColumnsView { kinds, columns }
}

pub(super) const NUMBER: &[CellKind] = &[CellKind::Number];

pub(super) const NUMBERS: &[CellKind] = &[CellKind::Number, CellKind::Number];
