//! Actual shared-index acquisition, owner aliases, eviction and final consumer release.
use super::*;
use crate::domain::{
    operating::types::OperatorContext,
    operator::contract::views::BorrowedRows,
    table_model::types::{Rows, Table},
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use resource_fixture::ResourceFixture;
use ryo_language::domain::{
    relations::types::{CellKind, ColumnsView, FactColumn, Integer, Relation, Value},
    symbols::types::Symbols,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, JoinColumn, Side},
    plans::types::Operation,
};
const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];
fn fixture(columns: &[FactColumn]) -> ResourceFixture {
    let fixture =
        ResourceFixture::new(&[Relation::Function], &[], super::super::indexes::LIMIT).unwrap();
    let left = Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(1))],
        count: crate::domain::table_model::types::RowCount(1),
    };
    let operation = Operation::Join {
        left: Input::Relation(Relation::Function),
        right: Input::Relation(Relation::Function),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let (mut symbols, mut scratch, mut memo) =
        (Symbols::default(), OperatorScratch::default(), vec![]);
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        scratch: &mut scratch,
        memo: &mut memo,
    };
    let mut out = Table::default();
    fixture
        .collect(
            &operation,
            &[
                Rows::Computed(&left),
                Rows::Stored(ColumnsView {
                    kinds: KINDS,
                    columns,
                }),
            ],
            &mut context,
            &mut out,
        )
        .unwrap();
    assert_eq!(out.cells, [Value::Number(Integer(10))]);
    assert!(fixture.no_invocation_index_handles());
    assert_eq!(fixture.shared_index_backings(), 1);
    fixture
}
fn hold(fixture: &ResourceFixture, columns: &[FactColumn]) -> super::super::view::AcquiredView {
    let owner = fixture.owner();
    let rows = BorrowedRows::Stored(ColumnsView {
        kinds: KINDS,
        columns,
    });
    let proof = owner.admit_relation(Relation::Function, rows).unwrap();
    let (population, _) = proof.identity(owner).unwrap();
    let plan = owner
        .derived
        .state
        .borrow()
        .entries
        .values()
        .find(|entry| entry._plan.kind == DerivedKind::Equality)
        .unwrap()
        ._plan
        .clone();
    owner
        .derived
        .acquire_stored(
            &plan,
            &[population],
            ViewContext::default(),
            DerivedInputs::Rows(&[rows]),
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap()
}
fn reclaim(fixture: &ResourceFixture, all_payload: bool) -> bool {
    let owner = &fixture.owner().derived;
    let mut backend = owner.backend.borrow_mut();
    let cache = backend.as_mut().unwrap();
    let cost = if all_payload {
        let payload = crate::application::file_dag::derived::graph_cache::types::reclamation::value::index_payload(&cache.entries[0]).unwrap();
        let metadata = cache.bytes - payload - owner.state.borrow().retained_bytes;
        cache.limit - metadata
    } else {
        cache.limit - cache.bytes + 1
    };
    owner.prepare_graph_room(cache, cost, &[]).unwrap()
}
#[test]
fn wrapper_eviction_preserves_backend_charge_and_live_consumers_block_reclamation() {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![10, 20])];
    let fixture = fixture(&columns);
    let weak = fixture.first_index_weak().unwrap();
    let held = hold(&fixture, &columns);
    assert!(!fixture.no_invocation_index_handles());
    assert!(
        !reclaim(&fixture, true),
        "a live consumer is not an owner-only alias"
    );
    assert_eq!(fixture.shared_index_backings(), 1);
    drop(held);
    assert!(fixture.no_invocation_index_handles());
    assert!(reclaim(&fixture, false));
    assert_eq!(fixture.owner().derived.retained_bytes(), 0);
    assert!(
        weak.upgrade().is_some(),
        "backend retains the physical index"
    );
    assert_eq!(fixture.shared_index_backings(), 1);
    assert!(
        fixture
            .owner()
            .derived
            .ledger
            .borrow()
            .snapshot()
            .view_optional_bytes
            > 0
    );
    assert!(reclaim(&fixture, true));
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.shared_index_backings(), 0);
    assert_eq!(
        fixture
            .owner()
            .derived
            .ledger
            .borrow()
            .snapshot()
            .view_optional_bytes,
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
            .derived
            .ledger
            .borrow()
            .snapshot()
            .view_optional_bytes,
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
            .derived
            .ledger
            .borrow()
            .snapshot()
            .unique_bytes,
        0
    );
}
