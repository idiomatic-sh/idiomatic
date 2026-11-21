//! The services of the `lowered` concept: the cost class of a fill and the code it is written and counted under, each a
//! property of the fill alone; where a column stands in an ordered match's row; and the walk of everything a pack's steps
//! and stages touch. Every `match` here names every
//! fill: a new fill does not compile without its class, its code and its pipelines.

use super::types::{
    FillInput, GraphFill, GroupFill, LoweredPack, MadeFill, OrderedFill, PatternForm,
};
use crate::domain::code::types::FillCode;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::traversal::service::{each_step, stage_touched};
use crate::domain::steps::traversal::types::Touched;
use crate::domain::steps::types::{CostClass, MadeColumn};
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::table_words::types::ColumnCount;

/// What bounds a fill's work each time its table is made: a definition's rows are the file's; a group, an order (a sorted
/// table, and an ordered match, whose two inputs are each sorted once and paired in one pass) and the fixed points are one
/// group's rows, sorted once; the other graphs walk the file's edges; written rows are made once for the run.
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

/// How wide the rows of an ordered input are: the width of each of its pipelines' rows. None for an input with no pipeline,
/// with pipelines whose rows differ in width, with no order column, or with a key or an order column past its rows.
fn ordered_width(input: &FillInput) -> Option<ColumnCount> {
    let mut widths = input.pipelines.iter().map(|pipeline| pipeline.row.0.len());
    let width = widths.next()?;
    let within = |column: &MadeColumn| usize::from(column.0) < width;
    let ordered = within(&input.order?) && input.keys.0.iter().all(within);
    (ordered && widths.all(|other| other == width)).then_some(ColumnCount(width))
}

/// The width of the left row of an ordered match whose table has `width` columns: what `match_column` places a column by.
/// None when the fill is not one the contract states: an input with no pipeline, or whose pipelines' rows differ in width;
/// an input with no order column; key lists of unequal length; a key or an order column past its row; or a table that is
/// not as wide as the two rows together. The encoding writes no such table.
#[must_use]
pub(crate) fn match_left_width(
    left: &FillInput,
    right: &FillInput,
    width: ColumnCount,
) -> Option<ColumnCount> {
    let (left_width, right_width) = (ordered_width(left)?, ordered_width(right)?);
    let paired = left.keys.0.len() == right.keys.0.len();
    let whole = left_width.0.checked_add(right_width.0) == Some(width.0);
    (paired && whole).then_some(left_width)
}

/// The column of an ordered match's row that holds column `column` of the row of `side`: the left row's cells stand first,
/// in their order, and the right row's after them. None for a column past what a made column numbers.
#[must_use]
pub(crate) fn match_column(
    left_width: ColumnCount,
    side: Side,
    column: MadeColumn,
) -> Option<MadeColumn> {
    match side {
        Side::Left => Some(column),
        Side::Right => {
            let place = left_width.0.checked_add(usize::from(column.0))?;
            match u16::try_from(place) {
                Ok(made) => Some(MadeColumn(made)),
                Err(_) => None,
            }
        }
    }
}

/// The pipelines of a fill, in its inputs' order.
fn fill_pipelines(fill: &MadeFill) -> Vec<&Pipeline> {
    let inputs: Vec<&FillInput> = match fill {
        MadeFill::Rows(pipelines, _) => return pipelines.iter().collect(),
        MadeFill::Group(_, input) | MadeFill::Ordered(OrderedFill::Sorted(input)) => vec![input],
        MadeFill::Ordered(OrderedFill::Match(left, right)) => vec![left, right],
        MadeFill::Graph(_, inputs) => inputs.iter().collect(),
        MadeFill::Constant(_) => Vec::new(),
    };
    inputs
        .into_iter()
        .flat_map(|input| &input.pipelines)
        .collect()
}

/// What every step and every stage of a pack touches, each met once: the programs of its definitions' call forms, then of its
/// rules' branches, each in pre-order through its inline bodies; then the stages of its tables' pipelines, in order. The row a
/// pipeline writes after its last stage (`Pipeline::row`) is no stage and is not met. The walk the six passes share.
pub(crate) fn each_touched<'a>(pack: &'a LoweredPack, visit: &mut impl FnMut(&Touched<'a>)) {
    let patterns = pack
        .definitions
        .iter()
        .flat_map(|definition| &definition.patterns);
    for pattern in patterns {
        match &pattern.form {
            PatternForm::Call(program) => each_step(program, &mut |_, touched| visit(touched)),
            PatternForm::Table(_) => {}
        }
    }
    for branch in pack.rules.iter().flat_map(|rule| &rule.branches) {
        each_step(&branch.program, &mut |_, touched| visit(touched));
    }
    for table in &pack.tables {
        for pipeline in fill_pipelines(&table.fill) {
            for placed in &pipeline.stages {
                visit(&stage_touched(&placed.stage));
            }
        }
    }
}
