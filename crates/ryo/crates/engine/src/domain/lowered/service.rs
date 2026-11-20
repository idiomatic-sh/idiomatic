//! The services of the `lowered` concept: the cost class of a fill and the code it is written and counted under, each a
//! property of the fill alone. Every `match` here names every fill: a new fill does not compile without its class and its code.

use super::types::{GraphFill, GroupFill, MadeFill, OrderedFill};
use crate::domain::code::types::FillCode;
use crate::domain::steps::types::CostClass;

/// What bounds a fill's work each time its table is made: a definition's rows are the file's; a group, an order and the fixed
/// points are one group's rows, sorted once; the other graphs walk the file's edges; written rows are made once for the run.
#[must_use]
pub(crate) const fn fill_class(fill: &MadeFill) -> CostClass {
    match fill {
        MadeFill::Rows(..) => CostClass::File,
        MadeFill::Group(group, _) => match group {
            GroupFill::Count { .. }
            | GroupFill::Digest { .. }
            | GroupFill::Ends(_)
            | GroupFill::Places => CostClass::Group,
        },
        MadeFill::Ordered(ordered) => match ordered {
            OrderedFill::Sorted(_) | OrderedFill::Match(..) => CostClass::Group,
        },
        MadeFill::Graph(graph, _) => match graph {
            GraphFill::FixedPoints => CostClass::Group,
            GraphFill::OrderedDfs | GraphFill::Worklist | GraphFill::Cycles => CostClass::File,
        },
        MadeFill::Constant(_) => CostClass::Constant,
    }
}

/// The code of a fill: what a table's code names its fill by, and what the statistics key a table's unit with.
#[must_use]
pub(crate) const fn fill_code(fill: &MadeFill) -> FillCode {
    FillCode(match fill {
        MadeFill::Rows(..) => 0,
        MadeFill::Group(group, _) => match group {
            GroupFill::Count { .. } => 1,
            GroupFill::Digest { .. } => 2,
            GroupFill::Ends(_) => 3,
            GroupFill::Places => 4,
        },
        MadeFill::Ordered(ordered) => match ordered {
            OrderedFill::Sorted(_) => 5,
            OrderedFill::Match(..) => 6,
        },
        MadeFill::Graph(graph, _) => match graph {
            GraphFill::FixedPoints => 7,
            GraphFill::OrderedDfs => 8,
            GraphFill::Worklist => 9,
            GraphFill::Cycles => 10,
        },
        MadeFill::Constant(_) => 11,
    })
}
