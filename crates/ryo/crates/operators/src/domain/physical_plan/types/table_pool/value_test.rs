//! A pool keeps a dropped table's room for the next node, emptied; it drops a table past either cap, and keeps no more
//! tables than its cap.
use super::value::{POOL_TABLE_CELLS, POOL_TABLE_KINDS, POOL_TABLES, PooledTables, TablePool};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PredicateOutcome;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The tables a case gives the pool.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One table of a row, within both caps.
    Small,
    /// One table of room for more cells than the cap.
    ManyCells,
    /// One table of room for more kinds than the cap.
    ManyKinds,
    /// More small tables than the pool keeps.
    PastTheCount,
}
type Input = Given;
type Value = Observation;
type Failure = ();

/// What the pool keeps after the case gave it its tables, and the table it hands out next.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// How many tables it keeps, whether each is empty and within both caps, and whether the next table is empty.
    Kept {
        tables: PooledTables,
        within: PredicateOutcome,
        fresh_empty: bool,
    },
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_dropped_table_is_kept_emptied_for_the_next_node"),
        word: Word::Succeeds,
        input: || Given::Small,
        expect: Expect::Value(Observation::Kept {
            tables: PooledTables(1),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("a_table_of_too_many_cells_is_dropped_not_pooled"),
        word: Word::Succeeds,
        input: || Given::ManyCells,
        expect: Expect::Value(Observation::Kept {
            tables: PooledTables(0),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("a_table_of_too_many_kinds_is_dropped_not_pooled"),
        word: Word::Succeeds,
        input: || Given::ManyKinds,
        expect: Expect::Value(Observation::Kept {
            tables: PooledTables(0),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
    Case {
        name: CaseName("the_pool_keeps_no_more_tables_than_its_cap"),
        word: Word::Succeeds,
        input: || Given::PastTheCount,
        expect: Expect::Value(Observation::Kept {
            tables: PooledTables(POOL_TABLES),
            within: PredicateOutcome::Holds,
            fresh_empty: true,
        }),
    },
];

/// A table of one number row.
fn small() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: vec![Cell::Number(Integer(1))],
        count: RowCount(1),
    }
}

fn subject(given: Input) -> Result<Value, Failure> {
    let mut pool = TablePool::default();
    match given {
        Given::Small => pool.pooled(small()),
        Given::ManyCells => {
            let mut table = small();
            table.cells.reserve_exact(POOL_TABLE_CELLS * 32);
            pool.pooled(table);
        }
        Given::ManyKinds => {
            let mut table = small();
            table.kinds.reserve_exact(POOL_TABLE_KINDS * 2);
            pool.pooled(table);
        }
        Given::PastTheCount => {
            for _ in 0..POOL_TABLES + 44 {
                pool.pooled(small());
            }
        }
    }
    let tables = pool.kept();
    let within = pool.within_caps();
    let fresh = pool.fresh();
    Ok(Observation::Kept {
        tables,
        within,
        fresh_empty: fresh.cells.is_empty() && fresh.kinds.is_empty() && fresh.count == RowCount(0),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
