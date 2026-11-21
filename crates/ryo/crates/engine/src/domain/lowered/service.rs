//! The services of the `lowered` concept: the cost class of a fill and the code it is written and counted under, each a
//! property of the fill alone; where a column stands in an ordered match's row; the walk of everything a pack's steps and
//! stages touch; and the walk in the same order that lends each to write. Every `match` here names every
//! fill: a new fill does not compile without its class, its code and its pipelines.

use super::types::{
    FillInput, GraphFill, GroupFill, LentPlaced, LoweredPack, MadeFill, OrderedFill, PatternForm,
    WalkedUnit,
};
use crate::domain::code::types::FillCode;
use crate::domain::steps::lending::service::each_step_mut;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::traversal::service::{each_step, stage_touched};
use crate::domain::steps::traversal::types::Touched;
use crate::domain::steps::types::{CostClass, DefinitionId, MadeColumn, MadeTableId};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::table_words::types::ColumnCount;

/// What bounds a fill's work each time its table is made: a definition's rows are the file's; a group, an order (a sorted
/// table, and an ordered match, whose two inputs are each sorted once and paired in one pass) and the fixed points are one
/// group's rows, sorted once (the fixed points then walk back from each fixed point of the group, never past it: the
/// reason in rows is on `GraphFill::FixedPoints`); the other graphs walk the file's edges, each for the reason in rows its
/// variant of `GraphFill` gives; written rows are made once for the run.
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
            GraphFill::FixedPoints(_) => CostClass::Group,
            GraphFill::OrderedDfs(_) | GraphFill::Worklist(_) | GraphFill::Cycles(_) => {
                CostClass::File
            }
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
            GraphFill::FixedPoints(_) => 7,
            GraphFill::OrderedDfs(_) => 8,
            GraphFill::Worklist(_) => 9,
            GraphFill::Cycles(_) => 10,
        },
        MadeFill::Constant(_) => 11,
    })
}

/// How wide the rows of a fill's input are: the width of each of its pipelines' rows. None for an input with no pipeline,
/// with pipelines whose rows differ in width, or with a key, an order or one of `told` (the further columns of the row a
/// fill's kernel is told) past its rows.
#[must_use]
pub(crate) fn input_width(input: &FillInput, told: &[MadeColumn]) -> Option<ColumnCount> {
    let mut widths = input.pipelines.iter().map(|pipeline| pipeline.row.0.len());
    let width = widths.next()?;
    let within = |column: &MadeColumn| usize::from(column.0) < width;
    let held = told.iter().all(within)
        && input.keys.0.iter().all(within)
        && input.order.as_ref().is_none_or(within);
    (held && widths.all(|other| other == width)).then_some(ColumnCount(width))
}

/// How wide the rows of an ordered input are: `input_width` of an input that has an order column, none without one.
fn ordered_width(input: &FillInput) -> Option<ColumnCount> {
    input.order.and(input_width(input, &[]))
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
pub(crate) fn fill_pipelines(fill: &MadeFill) -> Vec<&Pipeline> {
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

/// The pipelines of a fill, to write, in its inputs' order: the order of `fill_pipelines`.
fn fill_pipelines_mut(fill: &mut MadeFill) -> Vec<&mut Pipeline> {
    let inputs: Vec<&mut FillInput> = match fill {
        MadeFill::Rows(pipelines, _) => return pipelines.iter_mut().collect(),
        MadeFill::Group(_, input) | MadeFill::Ordered(OrderedFill::Sorted(input)) => vec![input],
        MadeFill::Ordered(OrderedFill::Match(left, right)) => vec![left, right],
        MadeFill::Graph(_, inputs) => inputs.iter_mut().collect(),
        MadeFill::Constant(_) => Vec::new(),
    };
    inputs
        .into_iter()
        .flat_map(|input| &mut input.pipelines)
        .collect()
}

/// Lends every step of the definitions' call forms, in the definitions' and their patterns' order.
fn lend_patterns(pack: &mut LoweredPack, visit: &mut impl FnMut(WalkedUnit, LentPlaced<'_>)) {
    for (number, definition) in pack.definitions.iter_mut().enumerate() {
        for (place, pattern) in definition.patterns.iter_mut().enumerate() {
            let unit = WalkedUnit::Pattern(
                DefinitionId(u32::try_from(number).unwrap_or(u32::MAX)),
                Count(place),
            );
            match &mut pattern.form {
                PatternForm::Call(program) => {
                    each_step_mut(program, &mut |placed| visit(unit, LentPlaced::Step(placed)));
                }
                // A pattern in table form has no step of its own: its table's fill is the unit that runs.
                PatternForm::Table(_) => {}
            }
        }
    }
}

/// Lends every step of the rules' branches, in the rules' and their branches' order.
fn lend_rules(pack: &mut LoweredPack, visit: &mut impl FnMut(WalkedUnit, LentPlaced<'_>)) {
    for rule in &mut pack.rules {
        let unit = WalkedUnit::Rule(rule.rule);
        for branch in &mut rule.branches {
            each_step_mut(&mut branch.program, &mut |placed| {
                visit(unit, LentPlaced::Step(placed));
            });
        }
    }
}

/// Lends every stage of the pipelines of the tables' fills, in the tables' order and each fill's inputs' order.
fn lend_tables(pack: &mut LoweredPack, visit: &mut impl FnMut(WalkedUnit, LentPlaced<'_>)) {
    for (number, table) in pack.tables.iter_mut().enumerate() {
        let unit = WalkedUnit::Table(MadeTableId(u32::try_from(number).unwrap_or(u32::MAX)));
        for pipeline in fill_pipelines_mut(&mut table.fill) {
            for placed in &mut pipeline.stages {
                visit(unit, LentPlaced::Stage(placed));
            }
        }
    }
}

/// Every step and every stage of a pack, each once, lent to write, in the one order of `each_touched`: the programs of its
/// definitions' call forms, then of its rules' branches, each in pre-order through its inline bodies; then the stages of
/// its tables' pipelines, in order. A definition a step or a stage names is never gone into: it is met as its own unit. The
/// visitor is told the unit each is of: a unit's steps or stages are met one after the other, so a pass that numbers a unit
/// starts again when the unit changes. The walk a pass that numbers, reorders or rewrites steps goes through.
pub(crate) fn each_placed_mut(
    pack: &mut LoweredPack,
    visit: &mut impl FnMut(WalkedUnit, LentPlaced<'_>),
) {
    lend_patterns(pack, visit);
    lend_rules(pack, visit);
    lend_tables(pack, visit);
}
