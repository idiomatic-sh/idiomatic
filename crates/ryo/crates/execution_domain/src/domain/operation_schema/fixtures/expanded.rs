//! The fixture operations of the kinds this engine never runs (`EXPANDED`): one of each, over the fixture's mixed row and edges.
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;

/// The irrelevant logical source of a fixture operation.
const FROM: Input = Input::Relation(Relation::Function);

/// A column of the fixture's mixed row.
const fn col(value: u32) -> ColumnIndex {
    ColumnIndex(value)
}

/// One operation of each expanded kind: an as-of and a group bound and ordinal by the mixed row's number, a follow of the edges.
pub fn expanded_operations() -> Vec<Operation> {
    vec![
        Operation::AsOf {
            left: FROM,
            right: FROM,
            left_on: vec![col(5), col(1)],
            right_on: vec![col(5), col(1)],
            left_order: col(0),
            right_order: col(0),
            bound: OrderBound::AtOrBefore,
            output: vec![
                JoinColumn {
                    side: Side::Right,
                    column: col(5),
                },
                JoinColumn {
                    side: Side::Left,
                    column: col(1),
                },
            ],
        },
        Operation::GroupBound {
            from: FROM,
            keys: vec![col(5)],
            order: col(0),
            end: GroupEnd::Greatest,
        },
        Operation::GroupOrdinal {
            from: FROM,
            keys: vec![col(5)],
            order: col(0),
        },
        Operation::Follow {
            edges: FROM,
            keys: vec![col(4)],
            source: col(0),
            destination: col(1),
        },
    ]
}
