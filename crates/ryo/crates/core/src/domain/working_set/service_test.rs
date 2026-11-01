//! The `working_set` service's cases, on values alone.

use super::service::{clear_rule, retain_pool};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::types::Room;
use crate::domain::operator::types::Pool;
use crate::domain::positions::types::Slots;
use crate::domain::table_model::types::{RowCount, Table};
use ryo_language::domain::relations::types::{CellKind, Integer, Value as Cell};

/// Aggregate reusable-payload allowance exercised by one case.
type Input = usize;
/// One when retirement preserves every live owner and enforces the allowance.
type Value = usize;
/// This retirement policy has no fallible dependency.
type Failure = ();

/// Retire dead slots while checking live frontier/shared-class ownership and actual capacities.
fn subject(limit: Input) -> Result<Value, Failure> {
    let row = Cell::Number(Integer(7));
    let table = Table {
        kinds: vec![CellKind::Number],
        cells: vec![row; 128],
        count: RowCount(128),
    };
    let mut room = Room::default();
    room.held[0] = table.clone();
    room.out = table.clone();
    room.scratch.right = table.clone();
    room.scratch.walk.reached = vec![row; 64];
    room.scratch.key = vec![row; 64];
    assert!(room.scratch.dfs_walk.visited.insert(row));
    let mut pool = Pool {
        steps: vec![table.clone(); 16],
        frontier: vec![
            table.clone(),
            Table {
                kinds: vec![CellKind::Number],
                ..Table::default()
            },
        ],
        frontier_pending: vec![true, true],
        rooms: vec![Some(Box::new(room))],
        ..Pool::default()
    };
    pool.made.classes.push(Slots(vec![3, 1, 3]));
    clear_rule(&mut pool);
    assert!(
        pool.steps
            .iter()
            .all(|table| table.count.0 == 0 && table.cells.is_empty())
    );
    assert_eq!(pool.frontier[0], table);
    assert_eq!(pool.frontier[1].kinds, [CellKind::Number]);
    assert_eq!(pool.made.classes[0].0, [3, 1, 3]);
    let kept = retain_pool(&mut pool, limit);
    assert!(kept <= limit);
    assert_eq!(pool.frontier[0].count.0, 128);
    assert_eq!(pool.frontier[1].kinds, [CellKind::Number]);
    pool.frontier_pending.fill(false);
    for frontier in &mut pool.frontier {
        frontier.cells.clear();
        frontier.count.0 = 0;
    }
    assert_eq!(retain_pool(&mut pool, 0), 0);
    assert!(
        pool.steps
            .iter()
            .chain(&pool.frontier)
            .all(|table| table.cells.capacity() == 0)
    );
    assert!(pool.rooms.iter().all(Option::is_none));
    Ok(1)
}

/// Zero, restrictive and sufficient aggregate allowances over the same owned buffers.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "zero_budget_drops_dead_buffers_without_losing_live_frontiers_or_shared_classes",
        ),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("aggregate_many_slot_capacity_is_bounded_not_each_slot_independently"),
        word: Word::Succeeds,
        input: || 4096,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("reusable_capacity_survives_when_aggregate_allowance_fits"),
        word: Word::Succeeds,
        input: || 1024 * 1024,
        expect: Expect::Value(1),
    },
];

/// Run every registered retirement and ownership case.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
