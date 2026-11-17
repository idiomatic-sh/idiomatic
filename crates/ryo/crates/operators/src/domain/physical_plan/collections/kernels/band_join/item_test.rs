//! The `band_join` item's cases: which lowerings run the kernel, and a generated table holding that a band join answers the
//! rows of the hash join and filter it replaces, in their order, every one or the nearest with its ties: each row of the table
//! plans and runs both programs through the production lowering and kernels and compares their answers.
use super::item::BandJoinItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::{BandJoinNode, DistinctNode, Lowering};
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId, RowSource};
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_relations::domain::relations::types::{CellKind, Flag, Integer, Value as Cell};
use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge, BandJoin, BandTest};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, HostedNumber, Input as StepInput, JoinColumn, KeyFan, Operand,
    PlanComparison, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// What a case gives the part: a lowering to reach.
type Input = Lowering;
/// Whether the lowering runs the kernel.
type Value = Reach;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_union_does_not_run_it"),
    word: Word::Succeeds,
    input: || {
        Lowering::Distinct(DistinctNode {
            sources: [
                RowSource::Node(PlanNodeId(0)),
                RowSource::Node(PlanNodeId(1)),
            ],
        })
    },
    expect: Expect::Value(Reach::Skips),
}];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; whether a lowering runs a kernel cannot fail"
)]
fn subject(lowering: Input) -> Result<Value, Failure> {
    Ok(BandJoinItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The class of the order cells of both sides.
#[derive(Clone, Copy, Debug)]
enum Cells {
    /// Numbers.
    Number,
    /// Texts.
    Text,
    /// Flags: ordered with nothing.
    Unordered,
    /// A number in each even row, a text in each odd one.
    Mixed,
}

/// Which rows each side holds.
#[derive(Clone, Copy, Debug)]
enum Sides {
    /// No left row.
    EmptyLeft,
    /// No right row.
    EmptyRight,
    /// Rows repeating a key and an order on each side, each left order also a right one of its key.
    Ties,
    /// Rows of distinct orders, no left order a right one of its key.
    Distinct,
}

/// One row of the generated table.
#[derive(Clone, Copy, Debug)]
struct Row {
    /// The test of the first edge.
    test: BandTest,
    /// How many edges the band has.
    edges: usize,
    /// Which right rows of the band it answers.
    answer: BandAnswer,
    /// The class of the order cells.
    cells: Cells,
    /// Which rows the sides hold.
    sides: Sides,
    /// How many key columns each side is joined on.
    width: usize,
}

/// Every row of the table: each test, one edge or two, every row or the nearest of one edge, each class of cells, each pair
/// of sides, and one key column or two.
fn table() -> Vec<Row> {
    let mut rows = Vec::new();
    for test in [
        BandTest::Below,
        BandTest::AtOrBelow,
        BandTest::Above,
        BandTest::AtOrAbove,
    ] {
        for (edges, answer) in [
            (1, BandAnswer::Every),
            (2, BandAnswer::Every),
            (1, BandAnswer::Nearest),
        ] {
            for cells in [Cells::Number, Cells::Text, Cells::Unordered, Cells::Mixed] {
                for sides in [
                    Sides::EmptyLeft,
                    Sides::EmptyRight,
                    Sides::Ties,
                    Sides::Distinct,
                ] {
                    for width in [1, 2] {
                        rows.push(Row {
                            test,
                            edges,
                            answer,
                            cells,
                            sides,
                            width,
                        });
                    }
                }
            }
        }
    }
    rows
}

/// The texts an order of text cells is made of, the order `at` the text `TEXTS[at]`.
const TEXTS: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h"];

/// A side's columns: its row's identity, two key columns and two order columns.
const WIDTH: u32 = 5;
/// The first order column of a side.
const FIRST_ORDER: u32 = 3;
/// The second order column of a side.
const SECOND_ORDER: u32 = 4;

/// The rows of a side, `(key, key, order, order)` each, as cells of `cells`, each row's identity counted from `first`.
fn side(rows: &[[usize; 4]], first: i64, cells: Cells, resources: &TextResources) -> Table {
    let order = |row: usize, at: usize| match cells {
        Cells::Number => Cell::Number(Integer(i64::try_from(at).unwrap_or(0))),
        Cells::Text => resources.symbol(TEXTS.get(at).copied().unwrap_or("a")),
        Cells::Unordered => Cell::Flag(if at % 2 == 0 {
            Flag::Raised
        } else {
            Flag::Lowered
        }),
        Cells::Mixed if row % 2 == 0 => Cell::Number(Integer(i64::try_from(at).unwrap_or(0))),
        Cells::Mixed => resources.symbol(TEXTS.get(at).copied().unwrap_or("a")),
    };
    let kind = match cells {
        Cells::Number | Cells::Mixed => CellKind::Number,
        Cells::Text => CellKind::Symbol,
        Cells::Unordered => CellKind::Flag,
    };
    let number = |value: usize| Cell::Number(Integer(i64::try_from(value).unwrap_or(0)));
    Table {
        kinds: vec![
            CellKind::Number,
            CellKind::Number,
            CellKind::Number,
            kind,
            kind,
        ],
        cells: rows
            .iter()
            .enumerate()
            .flat_map(|(at, [first_key, second_key, first_order, second_order])| {
                [
                    Cell::Number(Integer(first + i64::try_from(at).unwrap_or(0))),
                    number(*first_key),
                    number(*second_key),
                    order(at, *first_order),
                    order(at, *second_order),
                ]
            })
            .collect(),
        count: RowCount(rows.len()),
    }
}

/// The left and the right rows of `sides`.
fn sides_of(sides: Sides) -> (&'static [[usize; 4]], &'static [[usize; 4]]) {
    const LEFT: &[[usize; 4]] = &[[1, 1, 4, 6], [1, 2, 2, 5], [2, 1, 5, 7], [3, 1, 1, 1]];
    const RIGHT: &[[usize; 4]] = &[
        [1, 1, 3, 5],
        [1, 1, 1, 2],
        [1, 2, 5, 7],
        [1, 1, 6, 7],
        [2, 1, 7, 0],
        [2, 1, 2, 3],
        [1, 2, 0, 4],
    ];
    const TIED_LEFT: &[[usize; 4]] = &[[1, 1, 3, 5], [1, 1, 3, 5], [2, 1, 2, 3], [1, 2, 1, 1]];
    const TIED_RIGHT: &[[usize; 4]] = &[
        [1, 1, 3, 5],
        [1, 1, 3, 4],
        [1, 1, 1, 2],
        [1, 1, 5, 6],
        [1, 1, 5, 7],
        [2, 1, 2, 3],
        [2, 1, 2, 3],
        [1, 2, 1, 1],
    ];
    match sides {
        Sides::EmptyLeft => (&[], TIED_RIGHT),
        Sides::EmptyRight => (TIED_LEFT, &[]),
        Sides::Ties => (TIED_LEFT, TIED_RIGHT),
        Sides::Distinct => (LEFT, RIGHT),
    }
}

/// The step of `operation` over the earlier steps `at`.
fn over(operation: Operation, at: &[u32]) -> Step {
    Step {
        operation,
        inputs: at
            .iter()
            .map(|each| PlanInput::Node(PlanNodeId(*each)))
            .collect(),
    }
}

/// The input of an operation that reads step `at`.
const fn step(at: u32) -> StepInput {
    StepInput::Step(StepId(at))
}

/// The columns `0..count` of one side.
fn columns(count: u32) -> Vec<ColumnIndex> {
    (0..count).map(ColumnIndex).collect()
}

/// Every column of the left rows, then every column of the right rows.
fn both_sides() -> Vec<JoinColumn> {
    [Side::Left, Side::Right]
        .into_iter()
        .flat_map(|side| {
            columns(WIDTH)
                .into_iter()
                .map(move |column| JoinColumn { side, column })
        })
        .collect()
}

/// `left test right` of two columns.
const fn compared(left: u32, test: Test, right: u32) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(left)),
        right: Operand::Column(ColumnIndex(right)),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The band of `row`: the first edge's test on the first order columns, and for two edges the other direction on the second.
fn band_of(row: Row) -> Vec<BandEdge> {
    let other = match row.test {
        BandTest::Below | BandTest::AtOrBelow => BandTest::AtOrAbove,
        BandTest::Above | BandTest::AtOrAbove => BandTest::AtOrBelow,
    };
    [(FIRST_ORDER, row.test), (SECOND_ORDER, other)]
        .into_iter()
        .take(row.edges)
        .map(|(column, test)| BandEdge {
            left: ColumnIndex(column),
            test,
            right: ColumnIndex(column),
        })
        .collect()
}

/// The hosted left and right rows, and their band join by `row`.
fn banded(row: Row) -> Vec<Step> {
    let keys = columns(1 + u32::try_from(row.width).unwrap_or(1)).split_off(1);
    vec![
        over(
            Operation::Hosted {
                relation: HostedNumber(0),
            },
            &[],
        ),
        over(
            Operation::Hosted {
                relation: HostedNumber(1),
            },
            &[],
        ),
        over(
            Operation::BandJoin(BandJoin {
                left: step(0),
                right: step(1),
                left_on: keys.clone(),
                right_on: keys,
                band: band_of(row),
                answer: row.answer,
                output: both_sides(),
                filter: Vec::new(),
                view: Vec::new(),
                fan: KeyFan::Shared,
            }),
            &[0, 1],
        ),
    ]
}

/// What the band join of `row` replaces: the hash join of the hosted rows on the key, the filter by every edge (`right test
/// left`, the language's test of each), and for the nearest, the rows left by an antijoin of those of a left row that another of
/// its rows is closer than.
fn joined_then_filtered(row: Row) -> Vec<Step> {
    let keys = columns(1 + u32::try_from(row.width).unwrap_or(1)).split_off(1);
    let mut steps = vec![
        over(
            Operation::Hosted {
                relation: HostedNumber(0),
            },
            &[],
        ),
        over(
            Operation::Hosted {
                relation: HostedNumber(1),
            },
            &[],
        ),
        over(
            Operation::Join {
                left: step(0),
                right: step(1),
                left_on: keys.clone(),
                right_on: keys,
                output: both_sides(),
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
                fan: KeyFan::Shared,
            },
            &[0, 1],
        ),
        over(
            Operation::Select {
                from: step(2),
                comparisons: band_of(row)
                    .into_iter()
                    .map(|edge| {
                        compared(
                            WIDTH + edge.right.0,
                            crate::domain::equality_join::service::test_of(edge.test),
                            edge.left.0,
                        )
                    })
                    .collect(),
                kept: Vec::new(),
            },
            &[2],
        ),
    ];
    if row.answer == BandAnswer::Nearest {
        let order = WIDTH + FIRST_ORDER;
        let closer = match row.test {
            BandTest::Below | BandTest::AtOrBelow => Test::GreaterThan,
            BandTest::Above | BandTest::AtOrAbove => Test::LessThan,
        };
        let mut paired = both_sides();
        paired.push(JoinColumn {
            side: Side::Right,
            column: ColumnIndex(order),
        });
        steps.extend([
            over(
                Operation::Join {
                    left: step(3),
                    right: step(3),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: (0..2 * WIDTH)
                        .map(|column| JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(column),
                        })
                        .chain([JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(order),
                        }])
                        .collect(),
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                    fan: KeyFan::Shared,
                },
                &[3, 3],
            ),
            over(
                Operation::Select {
                    from: step(4),
                    comparisons: vec![compared(2 * WIDTH, closer, order)],
                    kept: Vec::new(),
                },
                &[4],
            ),
            over(
                Operation::Project {
                    from: step(5),
                    columns: columns(2 * WIDTH),
                    distinct: RepeatedRows::Dropped,
                },
                &[5],
            ),
            over(
                Operation::Antijoin {
                    left: step(3),
                    right: step(6),
                    left_on: columns(2 * WIDTH),
                    right_on: columns(2 * WIDTH),
                },
                &[3, 6],
            ),
        ]);
    }
    steps
}

/// The rows the last step of `steps` answers over the hosted sides of `row`, or the fault of planning or running them.
fn answer_of(row: Row, steps: &[Step]) -> Result<Table, String> {
    let mut resources = TextResources::new(TEXTS);
    let (left, right) = sides_of(row.sides);
    let hosted = vec![
        side(left, 0, row.cells, &resources),
        side(right, 100, row.cells, &resources),
    ];
    let inputs = TableInputs::new(&[]).with_hosted(hosted);
    let last = PlanNodeId(u32::try_from(steps.len()).unwrap_or(1).saturating_sub(1));
    emitted(
        steps,
        &[last],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )
    .map_err(|fault| format!("{fault:?}"))?
    .pop()
    .ok_or_else(|| "no table".to_owned())
}

/// Every row of the generated table answers through the band kernel the rows the production hash join and filter answer, in
/// their order; a row whose answers differ is listed with both. The table is not empty, and it answers rows.
#[test]
fn every_row_of_the_table_answers_the_hash_join_and_filter() {
    let rows = table();
    let mut differing = Vec::new();
    let (mut answered, mut refused) = (0, 0);
    for row in &rows {
        let band = answer_of(*row, &banded(*row));
        let reference = answer_of(*row, &joined_then_filtered(*row));
        let shown = |answer: &Result<Table, String>| match answer {
            Ok(table) => format!("{} rows {:?}", table.count.0, table.cells),
            Err(fault) => fault.clone(),
        };
        if band.as_ref().map(|table| (&table.cells, table.count))
            != reference.as_ref().map(|table| (&table.cells, table.count))
        {
            differing.push(format!(
                "{row:?}\n  band:      {}\n  reference: {}",
                shown(&band),
                shown(&reference)
            ));
        } else if band.is_err() {
            refused += 1;
        } else if band.as_ref().is_ok_and(|table| table.count.0 > 0) {
            answered += 1;
        }
    }
    assert_eq!(differing, Vec::<String>::new(), "{}", differing.join("\n"));
    assert_eq!((rows.len(), answered, refused), (384, 120, 0));
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
            crate::domain::equality_join::service::banded(
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
