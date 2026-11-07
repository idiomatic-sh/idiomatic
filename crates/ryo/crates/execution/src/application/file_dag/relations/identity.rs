//! Shallow immutable-population witnesses; no row scan, hashing or retained provider borrow.
use crate::application::file_dag::derived::ledger::types::BackingAddress;
use crate::application::file_dag::relations::publication::types::{
    SourceDescriptor, SpanLength, StorageSpan,
};
use crate::application::file_dag::relations::types::Sameness;

use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_model::service::kind;
use ryo_relations::domain::relations::types::{FactColumn, Relation};

macro_rules! sameness {
    ($equal:expr) => {
        if $equal {
            Sameness::Same
        } else {
            Sameness::Different
        }
    };
}
fn span<T>(pointer: *const T, length: SpanLength) -> StorageSpan {
    StorageSpan {
        address: BackingAddress(pointer as usize),
        length,
    }
}
fn column_span(column: &FactColumn) -> StorageSpan {
    span(column.0.as_ptr(), SpanLength(column.0.len()))
}
fn tree_spans(tree: &crate::domain::table_cells::types::TreeRows<'_>) -> Vec<StorageSpan> {
    let e = tree.elements;
    vec![
        span(e, SpanLength(tree.file.0 as usize)),
        span(e.kind.as_ptr(), SpanLength(e.kind.len())),
        span(e.start.as_ptr(), SpanLength(e.start.len())),
        span(e.end.as_ptr(), SpanLength(e.end.len())),
        span(e.parent.as_ptr(), SpanLength(e.parent.len())),
        span(e.ordinal.as_ptr(), SpanLength(e.ordinal.len())),
        span(e.text.as_ptr(), SpanLength(e.text.len())),
        span(e.size.as_ptr(), SpanLength(e.size.len())),
    ]
}
fn compact_spans(
    table: &ryo_values::domain::table_words::types::CompactTable,
) -> Vec<StorageSpan> {
    let storage = ryo_values::domain::compact_table::service::compact_storage(table);
    vec![
        StorageSpan {
            address: BackingAddress(storage.words.address.0),
            length: SpanLength(storage.words.length.0),
        },
        StorageSpan {
            address: BackingAddress(storage.offsets.address.0),
            length: SpanLength(storage.offsets.length.0),
        },
    ]
}
fn computed_spans(table: &ryo_values::domain::rows::types::Table) -> Vec<StorageSpan> {
    vec![
        span(table, SpanLength(table.count.0)),
        span(table.cells.as_ptr(), SpanLength(table.cells.len())),
    ]
}
fn same_columns(columns: &[FactColumn], stored: &[StorageSpan]) -> Sameness {
    if columns.len() == stored.len()
        && columns
            .iter()
            .zip(stored)
            .all(|(column, span)| column_span(column) == *span)
    {
        Sameness::Same
    } else {
        Sameness::Different
    }
}
pub(super) fn describe(relation: Relation, input: RepresentationInput<'_>) -> SourceDescriptor {
    let (storage, projection, source_kinds) = match input {
        RepresentationInput::Stored(view) => (
            view.columns.iter().map(column_span).collect(),
            vec![],
            view.kinds.to_vec(),
        ),
        RepresentationInput::Selected {
            source,
            positions,
            columns,
            ..
        } => {
            let mut addresses = vec![span(positions.0.as_ptr(), SpanLength(positions.0.len()))];
            addresses.extend(source.columns.iter().map(column_span));
            (addresses, columns.to_vec(), source.kinds.to_vec())
        }
        RepresentationInput::Compact(table) => (compact_spans(table), vec![], vec![]),
        RepresentationInput::Computed(table) => (computed_spans(table), vec![], vec![]),
        RepresentationInput::Positioned(view) => {
            let tree = describe(relation, RepresentationInput::Tree(view.tree));
            let mut storage = tree.storage;
            storage.push(span(
                view.positions.slots.0.as_ptr(),
                SpanLength(view.positions.slots.0.len()),
            ));
            storage.push(span(view.shape, SpanLength(view.positions.width.0)));
            (storage, vec![], vec![])
        }
        RepresentationInput::Tree(tree) => (tree_spans(&tree), vec![], vec![]),
    };
    SourceDescriptor {
        relation,
        kind: kind(input),
        kinds: input.kinds().to_vec(),
        rows: input.count(),
        storage,
        projection,
        source_kinds,
    }
}
pub(super) fn matches(source: &SourceDescriptor, input: RepresentationInput<'_>) -> Sameness {
    if source.kind != kind(input)
        || source.kinds != input.kinds()
        || source.rows != input.count()
    {
        return Sameness::Different;
    }
    match input {
        RepresentationInput::Stored(view) => {
            if source.source_kinds == view.kinds {
                same_columns(view.columns, &source.storage)
            } else {
                Sameness::Different
            }
        }
        RepresentationInput::Selected {
            source: view,
            positions,
            columns,
            ..
        } => sameness!({
            source.source_kinds == view.kinds
                && source.projection == columns
                && source.storage.first()
                    == Some(&span(positions.0.as_ptr(), SpanLength(positions.0.len())))
                && same_columns(view.columns, &source.storage[1..]) == Sameness::Same
        }),
        RepresentationInput::Compact(table) => sameness!(source.storage == compact_spans(table)),
        RepresentationInput::Computed(table) => sameness!(source.storage == computed_spans(table)),
        RepresentationInput::Positioned(view) => {
            let actual = describe(source.relation, RepresentationInput::Positioned(view));
            sameness!(source.storage == actual.storage)
        }
        RepresentationInput::Tree(tree) => sameness!(source.storage == tree_spans(&tree)),
    }
}

/// Compare an admitted borrow without replacing its identity with caller-provided keys.
pub(super) fn same(left: RepresentationInput<'_>, right: RepresentationInput<'_>) -> Sameness {
    fn columns(
        left: ryo_relations::domain::relations::types::ColumnsView<'_>,
        right: ryo_relations::domain::relations::types::ColumnsView<'_>,
    ) -> Sameness {
        sameness!({
            left.kinds == right.kinds
                && left.columns.len() == right.columns.len()
                && left.columns.iter().zip(right.columns).all(|(left, right)| {
                    left.0.as_ptr() == right.0.as_ptr() && left.0.len() == right.0.len()
                })
        })
    }
    match (left, right) {
        (RepresentationInput::Stored(left), RepresentationInput::Stored(right)) => columns(left, right),
        (RepresentationInput::Computed(left), RepresentationInput::Computed(right)) => {
            sameness!(std::ptr::eq(left, right))
        }
        (RepresentationInput::Compact(left), RepresentationInput::Compact(right)) => {
            sameness!(std::ptr::eq(left, right))
        }
        (RepresentationInput::Tree(left), RepresentationInput::Tree(right)) => sameness!({
            left.file == right.file
                && left.form == right.form
                && std::ptr::eq(left.elements, right.elements)
        }),
        (
            RepresentationInput::Selected {
                source: left,
                positions: lp,
                columns: lc,
                kinds: lk,
            },
            RepresentationInput::Selected {
                source: right,
                positions: rp,
                columns: rc,
                kinds: rk,
            },
        ) => sameness!({
            columns(left, right) == Sameness::Same
                && std::ptr::eq(lp.0, rp.0)
                && lc == rc
                && lk == rk
        }),
        (RepresentationInput::Positioned(left), RepresentationInput::Positioned(right)) => sameness!({
            std::ptr::eq(left.positions, right.positions)
                && std::ptr::eq(left.shape, right.shape)
                && left.kinds == right.kinds
                && same(
                    RepresentationInput::Tree(left.tree),
                    RepresentationInput::Tree(right.tree),
                ) == Sameness::Same
        }),
        _ => Sameness::Different,
    }
}
