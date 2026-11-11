//! The routed program the presence cases run.
use super::*;

/// The recogniser's self-join anti-join shape over `source_location`: its select runs on the ordered-existence route.
pub(in super::super) fn routed() -> Plan {
    let schema = kinds(Relation::SourceLocation);
    let number = ColumnIndex(
        u32::try_from(
            schema
                .iter()
                .position(|kind| *kind == CellKind::Number)
                .expect("a numeric column"),
        )
        .unwrap_or(0),
    );
    plan(vec![
        select(Input::Relation(Relation::SourceLocation), vec![]),
        step(Operation::Join {
            left: at(0),
            right: at(0),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: number,
                },
                JoinColumn {
                    side: Side::Right,
                    column: number,
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        step(Operation::Select {
            from: at(1),
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: vec![],
        }),
        step(Operation::Project {
            from: at(2),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Antijoin {
            left: at(0),
            right: at(3),
            left_on: vec![number],
            right_on: vec![ColumnIndex(0)],
        }),
    ])
}
