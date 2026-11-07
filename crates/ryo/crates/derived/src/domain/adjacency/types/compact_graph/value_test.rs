//! The compact graph answers lookups of its partitions and vertices.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::adjacency::service::build_borrowed;
use crate::domain::vertices::types::{EdgeDirection, PartitionId, VertexId};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    VertexOfAValue,
    PartitionOfTheEmptyKey,
    UnknownVertex,
    OffsetsCoverEveryVertex,
}
type Input = Scenario;
type Value = Option<u32>;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_value_has_the_vertex_of_its_first_sighting"),
        word: Word::Succeeds,
        input: || Scenario::VertexOfAValue,
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("the_empty_key_is_the_first_partition"),
        word: Word::Succeeds,
        input: || Scenario::PartitionOfTheEmptyKey,
        expect: Expect::Value(Some(0)),
    },
    Case {
        name: CaseName("a_value_never_seen_has_no_vertex"),
        word: Word::Succeeds,
        input: || Scenario::UnknownVertex,
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("offsets_cover_every_vertex_and_one_more"),
        word: Word::Succeeds,
        input: || Scenario::OffsetsCoverEveryVertex,
        expect: Expect::Value(Some(4)),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let rows = Table {
        kinds: vec![CellKind::Number, CellKind::Number],
        cells: [1, 2, 2, 3].map(|cell| Cell::Number(Integer(cell))).to_vec(),
        count: RowCount(2),
    };
    let graph = build_borrowed(&rows, [ColumnIndex(0), ColumnIndex(1)], &[])
        .map_err(|_| ())?
        .ok_or(())?;
    Ok(match input {
        Scenario::VertexOfAValue => graph
            .vertex(PartitionId(0), Cell::Number(Integer(2)))
            .map(|vertex: VertexId| vertex.0),
        Scenario::PartitionOfTheEmptyKey => graph.partition(&[]).map(|partition| partition.0),
        Scenario::UnknownVertex => graph
            .vertex(PartitionId(0), Cell::Number(Integer(9)))
            .map(|vertex| vertex.0),
        Scenario::OffsetsCoverEveryVertex => {
            u32::try_from(graph.offsets(EdgeDirection::Forward).len()).ok()
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
