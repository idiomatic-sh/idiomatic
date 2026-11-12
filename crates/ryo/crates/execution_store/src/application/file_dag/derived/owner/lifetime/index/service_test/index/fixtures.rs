//! The support of the `index` cases.
use crate::application::file_dag::derived::owner::service::*;
use crate::application::file_dag::derived::owner::types::Room;
use crate::application::file_dag::derived::owner::*;
use fixtures::resources::ResourceFixture;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::derived_views::types::*;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;
use std::borrow::Cow;

pub(super) const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];

/// What building the shared equality index over the rows answered: the rows it indexed, the payload
/// of the first, whether the build left no invocation handle once its view was dropped, and the
/// shared backings.
#[derive(Debug, PartialEq)]
pub(super) struct Built {
    pub(super) rows: Cow<'static, [RowIndex]>,
    pub(super) first_payload: Option<u64>,
    pub(super) invocation_handles_released: bool,
    pub(super) backings: usize,
}

/// Builds the shared equality index on column 0 of `columns`, and what the build answered.
pub(super) fn fixture(columns: &[FactColumn]) -> (ResourceFixture, Built) {
    let fixture = ResourceFixture::new(&[Relation::Function], &[], index::service::LIMIT).unwrap();
    let input = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns,
    });
    let held = fixture
        .index(Relation::Function, &input, &[ColumnIndex(0)])
        .unwrap();
    let rows = if let DerivedView::Equality(index) = held.view().unwrap() {
        index.rows.clone()
    } else {
        Vec::new()
    };
    let first_payload = rows
        .first()
        .and_then(|row| columns.get(1)?.0.get(row.0).copied());
    drop(held);
    let built = Built {
        rows: Cow::Owned(rows),
        first_payload,
        invocation_handles_released: fixture.no_invocation_index_handles(),
        backings: fixture.shared_index_backings(),
    };
    (fixture, built)
}

pub(super) fn hold(
    fixture: &ResourceFixture,
    columns: &[FactColumn],
) -> acquisition::admission::service::AcquiredView {
    let owner = fixture.owner();
    let rows = RepresentationInput::Stored(ColumnsView {
        kinds: KINDS,
        columns,
    });
    let proof = owner.admit_relation(Relation::Function, rows).unwrap();
    let plan = owner
        .derived()
        .state
        .borrow()
        .entries
        .values()
        .find(|entry| entry._plan.kind == DerivedKind::Equality)
        .unwrap()
        ._plan
        .clone();
    owner
        .derived()
        .acquire_stored(
            &plan,
            &owner
                .derived()
                .admit_view(&plan, &owner, &[&proof])
                .unwrap(),
            ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap()
}

/// Asks the owner for all the room the cache's optional payload takes: the room answered, and the
/// optional payload charged before the ask.
pub(super) fn reclaim(fixture: &ResourceFixture) -> (Room, usize) {
    let owner = fixture.owner().derived();
    let mut backend = owner.backend.borrow_mut();
    let cache = backend.as_mut().unwrap();
    // The single owner charges each optional physical allocation once.
    // Request all that space; do not subtract the same equality payload
    // twice as if an independent backend cache still owned a second copy.
    let payload = owner.ledger.borrow().snapshot().view_optional_bytes.0;
    let metadata = cache
        .bytes
        .0
        .checked_sub(payload)
        .expect("charged optional payload belongs to this cache");
    let cost = cache.limit.0 - metadata;
    let room = owner
        .prepare_graph_room(
            cache,
            crate::application::file_dag::accounting::ledger::types::ByteCount(cost),
            &[],
        )
        .unwrap();
    (room, payload)
}
