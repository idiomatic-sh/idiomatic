//! The `band_join` item's cases: which lowerings run the kernel, and that a band join answers the rows of the hash join and
//! filter it replaces, in their order, every one or the nearest with its ties.
use super::item::{BandJoinItem, banded};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{BandJoinNode, DistinctNode, Lowering};
use crate::domain::physical_plan::types::{InputRows, PlanNodeId, RowSource};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    BandAnswer, BandEdge, BandTest, ColumnIndex, JoinColumn, KeyFan, Side,
};
use ryo_values::domain::rows::types::{RowCount, Table};

/// What a case gives the part.
#[derive(Debug)]
enum Given {
    /// A lowering to reach.
    Reached(Lowering),
    /// The band join of the case rows by this band, answering this way.
    Banded(&'static [BandEdge], BandAnswer, Rights),
}

/// Which right rows a band join reads.
#[derive(Clone, Copy, Debug)]
enum Rights {
    /// The case's right rows.
    Some,
    /// No row.
    None,
}

/// What the part answered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// Whether the lowering runs the kernel.
    Reach(Reach),
    /// How many rows the band join answered, the same rows in the same order as the hash join and filter it replaces.
    Rows(usize),
}

/// What a check found broken.
#[derive(Debug, PartialEq)]
enum Breach {
    /// The kernel refused the rows.
    Refused(PortableOperatorFault),
    /// The band join's rows differ from the hash join and filter's.
    Differs,
}

type Input = Given;
type Value = Answer;
type Failure = Breach;

/// `right test left` of column `right` of the right rows and column `left` of the left rows.
const fn edge(left: u32, test: BandTest, right: u32) -> BandEdge {
    BandEdge {
        left: ColumnIndex(left),
        test,
        right: ColumnIndex(right),
    }
}

const BELOW: &[BandEdge] = &[edge(1, BandTest::Below, 1)];
const AT_OR_ABOVE: &[BandEdge] = &[edge(1, BandTest::AtOrAbove, 1)];
const INSIDE: &[BandEdge] = &[
    edge(1, BandTest::AtOrAbove, 1),
    edge(2, BandTest::AtOrBelow, 2),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Given::Reached(Lowering::Distinct(DistinctNode {
                sources: [
                    RowSource::Node(PlanNodeId(0)),
                    RowSource::Node(PlanNodeId(1)),
                ],
            }))
        },
        expect: Expect::Value(Answer::Reach(Reach::Skips)),
    },
    Case {
        name: CaseName("every_row_below_is_the_hash_join_and_filter"),
        word: Word::Succeeds,
        input: || Given::Banded(BELOW, BandAnswer::Every, Rights::Some),
        expect: Expect::Value(Answer::Rows(5)),
    },
    Case {
        name: CaseName("every_row_at_or_above_is_the_hash_join_and_filter"),
        word: Word::Succeeds,
        input: || Given::Banded(AT_OR_ABOVE, BandAnswer::Every, Rights::Some),
        expect: Expect::Value(Answer::Rows(5)),
    },
    Case {
        name: CaseName("an_interval_of_two_edges_is_the_hash_join_and_filter"),
        word: Word::Succeeds,
        input: || Given::Banded(INSIDE, BandAnswer::Every, Rights::Some),
        expect: Expect::Value(Answer::Rows(1)),
    },
    Case {
        name: CaseName("the_nearest_row_below_keeps_every_tie"),
        word: Word::Succeeds,
        input: || Given::Banded(BELOW, BandAnswer::Nearest, Rights::Some),
        expect: Expect::Value(Answer::Rows(4)),
    },
    Case {
        name: CaseName("the_nearest_row_at_or_above_keeps_every_tie"),
        word: Word::Succeeds,
        input: || Given::Banded(AT_OR_ABOVE, BandAnswer::Nearest, Rights::Some),
        expect: Expect::Value(Answer::Rows(4)),
    },
    Case {
        name: CaseName("no_right_row_answers_no_row"),
        word: Word::Succeeds,
        input: || Given::Banded(BELOW, BandAnswer::Every, Rights::None),
        expect: Expect::Value(Answer::Rows(0)),
    },
];

/// A table of rows of three numbers: a key and two orders.
fn numbers(rows: &[[i64; 3]]) -> Table {
    Table {
        kinds: vec![CellKind::Number; 3],
        cells: rows
            .iter()
            .flat_map(|row| row.iter().map(|cell| Cell::Number(Integer(*cell))))
            .collect(),
        count: RowCount(rows.len()),
    }
}

/// The cell at `row` and `column` of `table`, a number.
fn number(table: &Table, row: usize, column: ColumnIndex) -> i64 {
    match table.cells.get(row * 3 + usize::from(column)) {
        Some(Cell::Number(Integer(number))) => *number,
        _ => i64::MIN,
    }
}

/// The hash join on the key, then the filter by every edge, then for a nearest band the extreme rows of the first edge: the
/// rows the band join replaces, from the same rows.
fn joined_then_filtered(
    band: &[BandEdge],
    answer: BandAnswer,
    left: &Table,
    right: &Table,
) -> Vec<Cell> {
    let holds = |test: BandTest, theirs: i64, ours: i64| match test {
        BandTest::Below => theirs < ours,
        BandTest::AtOrBelow => theirs <= ours,
        BandTest::Above => theirs > ours,
        BandTest::AtOrAbove => theirs >= ours,
    };
    let mut cells = Vec::new();
    for l in 0..left.count.0 {
        let pairs: Vec<usize> = (0..right.count.0)
            .filter(|r| number(left, l, ColumnIndex(0)) == number(right, *r, ColumnIndex(0)))
            .filter(|r| {
                band.iter().all(|edge| {
                    holds(
                        edge.test,
                        number(right, *r, edge.right),
                        number(left, l, edge.left),
                    )
                })
            })
            .collect();
        let first = band.first().copied().unwrap_or(edge(0, BandTest::Below, 0));
        let values = pairs.iter().map(|r| number(right, *r, first.right));
        let nearest = match first.test {
            BandTest::Below | BandTest::AtOrBelow => values.max(),
            BandTest::Above | BandTest::AtOrAbove => values.min(),
        };
        for r in pairs {
            if answer == BandAnswer::Nearest && Some(number(right, r, first.right)) != nearest {
                continue;
            }
            cells.extend([
                Cell::Number(Integer(number(left, l, ColumnIndex(1)))),
                Cell::Number(Integer(number(right, r, ColumnIndex(1)))),
                Cell::Number(Integer(number(right, r, ColumnIndex(2)))),
            ]);
        }
    }
    cells
}

fn subject(given: Input) -> Result<Value, Failure> {
    let (band, answer, rights) = match given {
        Given::Reached(lowering) => return Ok(Answer::Reach(BandJoinItem.reached_by(&lowering))),
        Given::Banded(band, answer, rights) => (band, answer, rights),
    };
    let left = numbers(&[[1, 5, 9], [1, 3, 4], [2, 7, 8], [3, 1, 1]]);
    let right = match rights {
        Rights::Some => numbers(&[
            [1, 3, 6],
            [1, 1, 2],
            [1, 3, 4],
            [1, 7, 12],
            [2, 9, 10],
            [2, 2, 3],
        ]),
        Rights::None => numbers(&[]),
    };
    let output = [
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(1),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(2),
        },
    ];
    let key = [ColumnIndex(0)];
    let parts = crate::domain::band_join::service::lower(
        [&key, &key],
        band,
        answer,
        &output,
        &[],
        KeyFan::Shared,
    )
    .ok_or(Breach::Differs)?;
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(b""),
        paths: &[],
        arenas: &[],
    };
    let mut made = Table::default();
    banded(
        &parts,
        InputRows::Table(&left),
        InputRows::Table(&right),
        texts,
        &mut made,
    )
    .map_err(Breach::Refused)?;
    if made.cells == joined_then_filtered(band, answer, &left, &right) {
        Ok(Answer::Rows(made.count.0))
    } else {
        Err(Breach::Differs)
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for BandJoinNode {
    /// One instance per fan a band join answers: the nearest, and every row of a shared key, of the file alone and declared.
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        let output = [
            JoinColumn {
                side: Side::Left,
                column: columns.id,
            },
            JoinColumn {
                side: Side::Right,
                column: columns.id,
            },
        ];
        let band = [BandEdge {
            left: columns.order,
            test: BandTest::Below,
            right: columns.order,
        }];
        [
            (BandAnswer::Nearest, KeyFan::Shared),
            (BandAnswer::Every, KeyFan::Shared),
            (BandAnswer::Every, KeyFan::FileOnly),
            (BandAnswer::Every, KeyFan::AllPairs),
        ]
        .into_iter()
        .filter_map(|(answer, fan)| {
            crate::domain::band_join::service::lower(
                [&[columns.key], &[columns.key]],
                &band,
                answer,
                &output,
                &[],
                fan,
            )
        })
        .map(|band| Witnessed {
            node: Self {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                band,
            },
            roles: vec![InputRole::Rows, InputRole::Rows],
        })
        .collect()
    }
}
