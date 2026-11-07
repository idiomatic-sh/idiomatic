//! The `closure` service's cases, on values alone.

use super::service::{build, capacities};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

#[derive(Clone, Copy)]
enum Scenario {
    SourcesAreDistinct,
    PositionsSelectEdges,
    RecycledPartitionIsRewritten,
    AbsentSource,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_sources_are_distinct"),
        word: Word::Succeeds,
        input: || Scenario::SourcesAreDistinct,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("positions_select_the_edges_of_the_partition"),
        word: Word::Succeeds,
        input: || Scenario::PositionsSelectEdges,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_recycled_partition_keeps_the_capacity_it_grew"),
        word: Word::Succeeds,
        input: || Scenario::RecycledPartitionIsRewritten,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_source_column_the_rows_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentSource,
        expect: Expect::Error(DerivedFault::NoSuchColumn { column: ColumnIndex(6) }),
    },
];

fn edges() -> Table {
    Table {
        kinds: vec![CellKind::Number, CellKind::Number],
        cells: [1, 2, 1, 3, 2, 3]
            .map(|cell| Cell::Number(Integer(cell)))
            .to_vec(),
        count: RowCount(3),
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let (from, to) = (ColumnIndex(0), ColumnIndex(1));
    let mut buffer = Vec::new();
    Ok(match input {
        Scenario::SourcesAreDistinct => build(&edges(), None, &[], from, to, None, &mut buffer)?
            .sources
            .len(),
        Scenario::PositionsSelectEdges => build(
            &edges(),
            Some(&[RowIndex(2)]),
            &[],
            from,
            to,
            None,
            &mut buffer,
        )?
        .sources
        .len(),
        Scenario::RecycledPartitionIsRewritten => {
            let grown = build(&edges(), None, &[], from, to, None, &mut buffer)?;
            let before = capacities(&grown).sources;
            let again = build(&edges(), Some(&[RowIndex(0)]), &[], from, to, Some(grown), &mut buffer)?;
            usize::from(capacities(&again).sources >= before)
        }
        Scenario::AbsentSource => build(&edges(), None, &[], ColumnIndex(6), to, None, &mut buffer)?
            .sources
            .len(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
