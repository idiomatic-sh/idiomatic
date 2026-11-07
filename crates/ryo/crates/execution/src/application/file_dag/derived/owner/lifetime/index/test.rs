//! Actual shared-index acquisition, owner aliases, eviction and final consumer release.
use crate::application::file_dag::derived::owner::*;
use fixtures::resources::ResourceFixture;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use crate::domain::table_model::types::Rows;
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];
fn fixture(columns: &[FactColumn]) -> ResourceFixture {
    let fixture = ResourceFixture::new(
        &[Relation::Function],
        &[],
        crate::application::file_dag::derived::owner::index::service::LIMIT,
    )
    .unwrap();
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
        columns[1].0[index.rows[0].0],
        10,
        "the indexed first matching row retains its payload"
    );
    drop(held);
    assert!(fixture.no_invocation_index_handles());
    assert_eq!(fixture.shared_index_backings(), 1);
    fixture
}
fn hold(
    fixture: &ResourceFixture,
    columns: &[FactColumn],
) -> crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView {
    let owner = fixture.owner();
    let rows = BorrowedRows::Stored(ColumnsView {
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
            &owner.derived().admit_view(&plan, &owner, &[&proof]).unwrap(),
            ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap()
}
fn reclaim(fixture: &ResourceFixture) -> Room {
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
#[test]
fn one_owner_reclaims_index_only_after_the_final_live_consumer_releases() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![10, 20])];
    let fixture = fixture(&columns);
    let weak = fixture.first_index_weak().unwrap();
    let held = hold(&fixture, &columns);
    assert!(!fixture.no_invocation_index_handles());
    assert!(
        reclaim(&fixture) == Room::Full,
        "a live consumer is not an owner-only alias"
    );
    assert_eq!(fixture.shared_index_backings(), 1);
    drop(held);
    assert!(fixture.no_invocation_index_handles());
    // Equality payloads now have one owner entry, not a second backend cache.
    // Preserve the lifetime contract: reclamation was refused with a live
    // consumer above, and is allowed only after that final consumer released it.
    assert!(weak.upgrade().is_some());
    assert!(reclaim(&fixture) == Room::Fits);
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.shared_index_backings(), 0);
    assert_eq!(
        fixture
            .owner()
            .derived()
            .ledger
            .borrow()
            .snapshot()
            .view_optional_bytes
            .0,
        0
    );
}
#[test]
fn backend_clear_preserves_one_required_backing_until_last_consumer_drops() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![10, 20])];
    let fixture = fixture(&columns);
    let weak = fixture.first_index_weak().unwrap();
    let held = hold(&fixture, &columns);
    fixture.owner().release_population();
    assert!(weak.upgrade().is_some());
    assert_eq!(fixture.shared_index_backings(), 1);
    assert_eq!(
        fixture
            .owner()
            .derived()
            .ledger
            .borrow()
            .snapshot()
            .view_optional_bytes
            .0,
        0
    );
    let DerivedView::Equality(index) = held.view().unwrap() else {
        panic!("equality");
    };
    assert_eq!(
        index.rows.len(),
        2,
        "outstanding immutable resource remains readable"
    );
    drop(held);
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.shared_index_backings(), 0);
    assert_eq!(
        fixture
            .owner()
            .derived()
            .ledger
            .borrow()
            .snapshot()
            .unique_bytes
            .0,
        0
    );
}
