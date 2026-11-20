//! The services of the `lowered` concept: the cost class of a fill and the code it is written and counted under, each a
//! property of the fill alone; and the walk of everything a pack's steps and stages touch. Every `match` here names every
//! fill: a new fill does not compile without its class, its code and its pipelines.

use super::types::{
    FillInput, GraphFill, GroupFill, LoweredPack, MadeFill, OrderedFill, PatternForm,
};
use crate::domain::code::types::FillCode;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::traversal::service::{each_step, stage_touched};
use crate::domain::steps::traversal::types::Touched;
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
