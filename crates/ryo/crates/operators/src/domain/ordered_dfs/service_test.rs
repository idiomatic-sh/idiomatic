//! Each distinct root is visited once; each edge it reaches is one row with the ordinal of the visit.
use super::service::evaluate;
use super::types::DfsShape;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::RowScratch;
use crate::domain::evaluation::fixtures::{FakeDriver, number, written};
use crate::domain::ordered_dfs::types::OrderedEdge;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// Which roots the engine's graph holds.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// One root reaching two edges.
    OneRoot,
    /// The same root twice in sorted order.
    RepeatedRoot,
    /// A root row narrower than its root columns.
    NarrowRoot,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_reached_edge_is_one_row_with_its_ordinal"),
        word: Word::Succeeds,
        input: || Scenario::OneRoot,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 5 5 6 9 1|1 5 6 7 8 2")),
    },
    Case {
        name: CaseName("a_repeated_root_is_visited_once"),
        word: Word::Succeeds,
        input: || Scenario::RepeatedRoot,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 5 5 6 9 1|1 5 6 7 8 2")),
    },
    Case {
        name: CaseName("a_root_row_without_its_root_column_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NarrowRoot,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(0),
        }),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = vec![number(1), number(5)];
    let mut driver = FakeDriver {
        roots: match scenario {
            Scenario::OneRoot => vec![root],
            Scenario::RepeatedRoot => vec![root.clone(), root],
            Scenario::NarrowRoot => vec![Vec::new()],
        },
        root_columns: vec![ColumnIndex(0), ColumnIndex(1)],
        edges: vec![
            vec![number(5), number(6), number(9)],
            vec![number(6), number(7), number(8)],
        ],
        visits: vec![vec![
            OrderedEdge {
                edge: RowIndex(0),
                ordinal: Integer(1),
            },
            OrderedEdge {
                edge: RowIndex(1),
                ordinal: Integer(2),
            },
        ]],
        ..FakeDriver::default()
    };
    let shape = DfsShape {
        root: ColumnIndex(1),
        edge_columns: [
            ColumnIndex(0),
            ColumnIndex(1),
            ColumnIndex(1),
            ColumnIndex(2),
        ],
        root_keys: &[ColumnIndex(0)],
        edge_keys: &[],
    };
    evaluate(shape, &mut driver, &mut RowScratch::default())?;
    Ok(std::borrow::Cow::Owned(written(&driver.emitted)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
