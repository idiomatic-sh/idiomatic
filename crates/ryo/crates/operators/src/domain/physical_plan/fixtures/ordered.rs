//! The ordered existence programs: an antijoin of a later row over a self join of `semantic-child` edges, the shape the
//! `ordered_existence` rewrite probes as an ordered probe, and the rows the cases run it over.

use super::{Step, node, step};
use crate::domain::physical_plan::types::PlanInput;
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, Operand, PlanComparison,
    RepeatedRows, Side, Test,
};
use ryo_values::domain::plans::types::Operation;

/// The relation an ordered existence reads: parent, child, ordinal (a number), field, file.
pub(crate) const EDGES: Relation = Relation::SemanticChild;

/// What an ordered existence program varies: the comparison of the self join's two order cells, the order column, the
/// antijoin's key columns, and whether a second node reads the select.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OrderedShape {
    /// The comparison of the left order cell with the right one.
    pub(crate) test: Test,
    /// The order column of `EDGES`.
    pub(crate) order: u32,
    /// The antijoin's key columns, the same on both sides.
    pub(crate) key: &'static [u32],
    /// Whether a second node reads the select.
    pub(crate) shared_select: bool,
    /// How the self join pairs each row with those past it.
    pub(crate) pairing: Pairing,
}

/// How an ordered existence's self join pairs each row with the rows past it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pairing {
    /// A join on the parent with a select of the order comparison after it.
    JoinThenSelect,
    /// The band join the band lowering writes for the same `where`: the comparison its one edge, no select.
    Band,
    /// The same band join read by the antijoin itself, with no projection between: the band join answers the select's
    /// columns, so the lowering writes no projection after it.
    BandRead,
}

/// The ordered existence shape over `EDGES`, starting at node `first`: a self join on the parent answering the left parent,
/// child and ordinal and the right order cell, a select comparing the two order cells, the distinct projection of the left
/// columns, and the antijoin of `EDGES` with it on `shape.key`; with `shared_select`, a projection of the select after them.
pub(crate) fn ordered_existence(first: u32, shape: OrderedShape) -> Vec<Step> {
    let edges = (StepInput::Relation(EDGES), PlanInput::Relation(EDGES));
    let left = |column| JoinColumn {
        side: Side::Left,
        column: ColumnIndex(column),
    };
    if matches!(shape.pairing, Pairing::Band | Pairing::BandRead) {
        return banded_existence(first, shape);
    }
    let mut steps = vec![
        step(
            Operation::Join {
                left: edges.0,
                right: edges.0,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    left(0),
                    left(1),
                    left(2),
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(shape.order),
                    },
                ],
                filter: Vec::new(),
                view: Vec::new(),
                fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
                reach: None,
            },
            &[edges.1, edges.1],
        ),
        step(
            Operation::Select {
                from: node(first).0,
                comparisons: vec![PlanComparison {
                    test: shape.test,
                    left: Operand::Column(ColumnIndex(shape.order)),
                    right: Operand::Column(ColumnIndex(3)),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            },
            &[node(first).1],
        ),
        step(
            Operation::Project {
                from: node(first + 1).0,
                columns: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)],
                distinct: RepeatedRows::Dropped,
            },
            &[node(first + 1).1],
        ),
        step(
            Operation::Antijoin {
                left: edges.0,
                right: node(first + 2).0,
                left_on: shape.key.iter().copied().map(ColumnIndex).collect(),
                right_on: shape.key.iter().copied().map(ColumnIndex).collect(),
            },
            &[edges.1, node(first + 2).1],
        ),
    ];
    if shape.shared_select {
        steps.push(step(
            Operation::Project {
                from: node(first + 1).0,
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            &[node(first + 1).1],
        ));
    }
    steps
}

/// The ordered existence shape as the band lowering writes it, starting at node `first`: a band join of `EDGES` with itself on
/// the parent whose one edge is the order comparison, answering the left parent, child and ordinal, the distinct projection of
/// them (none for `Pairing::BandRead`), and the antijoin of `EDGES` with it on `shape.key`.
fn banded_existence(first: u32, shape: OrderedShape) -> Vec<Step> {
    use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge, BandJoin, BandTest};
    let edges = (StepInput::Relation(EDGES), PlanInput::Relation(EDGES));
    // `left test right`, read as `right test' left`.
    let test = match shape.test {
        Test::LessThan => BandTest::Above,
        Test::LessThanOrEqual => BandTest::AtOrAbove,
        Test::GreaterThan => BandTest::Below,
        _ => BandTest::AtOrBelow,
    };
    let mut steps = vec![
        step(
            Operation::BandJoin(BandJoin {
                left: edges.0,
                right: edges.0,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                band: vec![BandEdge {
                    left: ColumnIndex(shape.order),
                    test,
                    right: ColumnIndex(shape.order),
                }],
                answer: BandAnswer::Every,
                output: [0, 1, 2]
                    .map(|column| JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(column),
                    })
                    .to_vec(),
                filter: Vec::new(),
                view: Vec::new(),
                // Keyed on the parent, a column many rows share: the fan the query compiler proves for it.
                fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
            }),
            &[edges.1, edges.1],
        ),
    ];
    if shape.pairing == Pairing::BandRead {
        steps.push(antijoin_of(first, &shape));
        return steps;
    }
    steps.push(step(
        Operation::Project {
            from: node(first).0,
            columns: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)],
            distinct: RepeatedRows::Dropped,
        },
        &[node(first).1],
    ));
    steps.push(antijoin_of(first + 1, &shape));
    steps
}

/// The antijoin of `EDGES` with the pairs at node `pairs` on `shape.key`.
fn antijoin_of(pairs: u32, shape: &OrderedShape) -> Step {
    step(
        Operation::Antijoin {
            left: StepInput::Relation(EDGES),
            right: node(pairs).0,
            left_on: shape.key.iter().copied().map(ColumnIndex).collect(),
            right_on: shape.key.iter().copied().map(ColumnIndex).collect(),
        },
        &[PlanInput::Relation(EDGES), node(pairs).1],
    )
}

/// Both ordered existence shapes, the antijoins nodes 3 and 7: per parent the rows of the greatest ordinal, then of the least.
pub(crate) fn ordered_existence_pair() -> Vec<Step> {
    let shape = |test| OrderedShape {
        test,
        order: 2,
        key: &[0, 1, 2],
        shared_select: false,
        pairing: Pairing::JoinThenSelect,
    };
    let mut steps = ordered_existence(0, shape(Test::LessThan));
    steps.extend(ordered_existence(4, shape(Test::GreaterThan)));
    steps
}

/// The rows of `EDGES` the ordered existence cases read, the field `field`: parents 1, 6 and 8, ties at the greatest
/// ordinal of 1 and the least of 8.
pub(crate) fn ordered_rows(
    field: ryo_relations::domain::relations::types::Value,
) -> ryo_values::domain::rows::types::Table {
    use ryo_relations::domain::relations::fixtures::{kinds, node_id};
    use ryo_relations::domain::relations::types::{FileId, Position, Value};
    let rows = [
        (1, 2, 0),
        (1, 3, 2),
        (1, 4, 2),
        (1, 5, 1),
        (6, 7, 5),
        (8, 9, 3),
        (8, 10, 3),
        (8, 11, 4),
    ];
    let mut cells = Vec::with_capacity(rows.len() * kinds(EDGES).len());
    for (parent, child, ordinal) in rows {
        cells.extend([
            Value::Node(node_id(FileId(0), Position(parent))),
            Value::Node(node_id(FileId(0), Position(child))),
            Value::Number(Integer(ordinal)),
            field,
            Value::File(FileId(0)),
        ]);
    }
    ryo_values::domain::rows::types::Table {
        kinds: kinds(EDGES).to_vec(),
        cells,
        count: ryo_values::domain::rows::types::RowCount(rows.len()),
    }
}
