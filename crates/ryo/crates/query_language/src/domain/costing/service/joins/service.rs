//! The `costing` service's joins part: what the keys of a join and the `where` of its statement say of the pairs it makes.

use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, JoinColumn, Operand, PlanComparison, Side, Test,
};

use super::super::super::types::{
    AssembledSteps, CostShape, JoinFacts, JoinKeys, PlanCostClass, RangeBound, Refining, SelfOrder,
    ShapeClass, SideRows,
};
use crate::domain::planning::service::fields_of;
use crate::domain::resolving::service::column_of;

/// What the column `column` of `input` holds; none for a column the input does not have. The columns of an input are the
/// planner's (`planning::fields_of`): the one reading of a relation's, a step's and a bound input's columns.
pub(crate) fn kind_at(
    assembled: AssembledSteps<'_>,
    input: Input,
    column: ColumnIndex,
) -> Option<CellKind> {
    fields_of(assembled.lowered, assembled.observations, input)
        .get(usize::from(column))
        .map(|field| field.kind)
}

/// Whether rows met on these columns of `input` are met on the file alone: every one of them holds the file. No column at all
/// is the file alone too: nothing keeps a row of the file from any other.
pub(crate) fn file_alone(
    assembled: AssembledSteps<'_>,
    input: Input,
    on: &[ColumnIndex],
) -> JoinKeys {
    if on
        .iter()
        .all(|column| kind_at(assembled, input, *column) == Some(CellKind::File))
    {
        JoinKeys::FileAlone
    } else {
        JoinKeys::Beyond
    }
}

/// Two sets of keys together, the two sides' or a join's and its `where`'s: on the file alone only when both are.
pub(crate) fn both(left: JoinKeys, right: JoinKeys) -> JoinKeys {
    match (left, right) {
        (JoinKeys::FileAlone, JoinKeys::FileAlone) => JoinKeys::FileAlone,
        (JoinKeys::Beyond, JoinKeys::FileAlone | JoinKeys::Beyond)
        | (JoinKeys::FileAlone, JoinKeys::Beyond) => JoinKeys::Beyond,
    }
}

/// What the comparisons of a `where` add to the keys of the join whose rows it filters: `sides` are the join's two inputs and
/// `output` the columns it gives, each of one side. Only a comparison of a column of one side with a column of the other adds
/// anything: an equality is a further key unless both columns hold the file, a column bounded from below and from above is a
/// range, and in a join of an input with itself an order between a column and itself makes ordered pairs.
pub(crate) fn refining(
    assembled: AssembledSteps<'_>,
    sides: [Input; 2],
    output: &[JoinColumn],
    comparisons: &[PlanComparison],
) -> Refining {
    let [left, right] = sides;
    let mut found = Refining {
        keys: JoinKeys::FileAlone,
        range: RangeBound::Open,
        order: SelfOrder::Unordered,
    };
    let (mut lower, mut upper): (Vec<JoinColumn>, Vec<JoinColumn>) = (Vec::new(), Vec::new());
    for comparison in comparisons {
        let (Operand::Column(first), Operand::Column(second)) =
            (&comparison.left, &comparison.right)
        else {
            continue;
        };
        let (first, second) = (*column_of(output, *first), *column_of(output, *second));
        if first.side == second.side {
            continue;
        }
        let holds = |column: JoinColumn| {
            let input = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            kind_at(assembled, input, column.column)
        };
        match comparison.test {
            Test::Equal => {
                if holds(first) != Some(CellKind::File) || holds(second) != Some(CellKind::File) {
                    found.keys = JoinKeys::Beyond;
                }
            }
            Test::LessThan | Test::LessThanOrEqual => {
                upper.push(first);
                lower.push(second);
            }
            Test::GreaterThan | Test::GreaterThanOrEqual => {
                lower.push(first);
                upper.push(second);
            }
            Test::NotEqual
            | Test::FloatTextGreaterThanOrEqual
            | Test::In
            | Test::NotIn
            | Test::GlobAny
            | Test::NotGlobAny
            | Test::StartsWith
            | Test::EndsWith
            | Test::Matches
            | Test::NumericGreaterThanOrEqual
            | Test::IntegerSuccessor
            | Test::NaturalLessThan => continue,
        }
        if left == right && first.column == second.column && comparison.test != Test::Equal {
            found.order = SelfOrder::Ordered;
        }
    }
    if lower.iter().any(|column| upper.contains(column)) {
        found.range = RangeBound::TwoSided;
    }
    found
}

/// The shape and class of a join from its keys and what the `where` of its statement adds; none when it is linear. A side proved
/// few rows for a file makes any join linear. Otherwise a join on the file alone is a product, unless one column is bounded on
/// both ends; and a join keyed beyond the file is linear, unless it is of an input with itself and orders a column against
/// itself, which makes every ordered pair of a key group, again unless one column is bounded on both ends.
pub(crate) fn join_verdict(facts: JoinFacts, refining: Refining) -> Option<ShapeClass> {
    if facts.side == SideRows::Bounded {
        return None;
    }
    let sort = ShapeClass {
        shape: CostShape::Sort,
        class: PlanCostClass::LinearWithSorts,
    };
    match (
        both(facts.keys, refining.keys),
        refining.order,
        refining.range,
    ) {
        (JoinKeys::Beyond, SelfOrder::Unordered, RangeBound::TwoSided | RangeBound::Open) => None,
        (JoinKeys::Beyond, SelfOrder::Ordered, RangeBound::TwoSided)
        | (JoinKeys::FileAlone, SelfOrder::Ordered | SelfOrder::Unordered, RangeBound::TwoSided) => {
            Some(sort)
        }
        (JoinKeys::Beyond, SelfOrder::Ordered, RangeBound::Open) => Some(ShapeClass {
            shape: CostShape::OrderedPairs,
            class: PlanCostClass::QuadraticInGroup,
        }),
        (JoinKeys::FileAlone, SelfOrder::Ordered | SelfOrder::Unordered, RangeBound::Open) => {
            Some(ShapeClass {
                shape: CostShape::Product,
                class: PlanCostClass::QuadraticInFile,
            })
        }
    }
}

/// The one of two verdicts of the greater class; the first when they are of one class.
pub(crate) fn worse(first: Option<ShapeClass>, second: Option<ShapeClass>) -> Option<ShapeClass> {
    match (first, second) {
        (Some(one), Some(other)) if one.class < other.class => Some(other),
        (None, Some(other)) => Some(other),
        (Some(one), Some(_) | None) => Some(one),
        (None, None) => None,
    }
}
