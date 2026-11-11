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

pub(super) const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];

pub(super) fn fixture(columns: &[FactColumn]) -> ResourceFixture {
    let fixture = ResourceFixture::new(&[Relation::Function], &[], index::service::LIMIT).unwrap();
    let input = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns,
    });
    let held = fixture
        .index(Relation::Function, &input, &[ColumnIndex(0)])
        .unwrap();
    let DerivedView::Equality(index) = held.view().unwrap() else {
        panic!("actual equality view")
    };
    assert_eq!(
        index.rows,
        [
            ryo_values::domain::rows::types::RowIndex(0),
            ryo_values::domain::rows::types::RowIndex(1)
        ]
    );
    assert_eq!(
        columns[1].0[index.rows[0].0], 10,
        "the indexed first matching row retains its payload"
    );
    drop(held);
    assert!(fixture.no_invocation_index_handles());
    assert_eq!(fixture.shared_index_backings(), 1);
    fixture
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

pub(super) fn reclaim(fixture: &ResourceFixture) -> Room {
    let owner = fixture.owner().derived();
    let mut backend = owner.backend.borrow_mut();
    let cache = backend.as_mut().unwrap();
    // The single owner charges each optional physical allocation once.
    // Request all that space; do not subtract the same equality payload
    // twice as if an independent backend cache still owned a second copy.
    let payload = owner.ledger.borrow().snapshot().view_optional_bytes.0;
    assert!(payload > 0);
    let metadata = cache
        .bytes
        .0
        .checked_sub(payload)
        .expect("charged optional payload belongs to this cache");
    let cost = cache.limit.0 - metadata;
    owner
        .prepare_graph_room(
            cache,
            crate::application::file_dag::derived::ledger::types::ByteCount(cost),
            &[],
        )
        .unwrap()
}
