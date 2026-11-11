//! Actual shared-index acquisition, owner aliases, eviction and final consumer release.
use super::fixtures::{fixture, hold, reclaim};
use crate::application::file_dag::derived::owner::*;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_relations::domain::relations::types::FactColumn;
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
