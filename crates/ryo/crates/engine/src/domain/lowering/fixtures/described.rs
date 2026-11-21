//! A lowered pack said in lines, for the lowering's cases and its evidence: each definition with its patterns, each made
//! table with its fill and its pipelines' stages, each rule with its branches' steps; and the codes of an encoded pack by
//! count. Every `match` names every step and stage: a new one does not compile without its name here.

use crate::domain::code::types::ProgramCode;
use crate::domain::lowered::service::fill_pipelines;
use crate::domain::lowered::types::{
    GraphFill, GroupFill, LoweredPack, MadeFill, OrderedFill, PatternForm,
};
use crate::domain::steps::service::{stage_class, step_class};
use crate::domain::steps::stages::types::{FillStage, Pipeline};
use crate::domain::steps::types::{ProgramStep, StepProgram};
use std::collections::BTreeMap;

/// The name of a step.
#[must_use]
pub(crate) const fn step_name(step: &ProgramStep) -> &'static str {
    match step {
        ProgramStep::EachKind { .. } => "EachKind",
        ProgramStep::EachMatch { .. } => "EachMatch",
        ProgramStep::EachRange { .. } => "EachRange",
        ProgramStep::EachRow { .. } => "EachRow",
        ProgramStep::Children { .. } => "Children",
        ProgramStep::Axis { .. } => "Axis",
        ProgramStep::Within { .. } => "Within",
        ProgramStep::Nearest { .. } => "Nearest",
        ProgramStep::Closure { .. } => "Closure",
        ProgramStep::Read { .. } => "Read",
        ProgramStep::Test { .. } => "Test",
        ProgramStep::Call { .. } => "Call",
        ProgramStep::Render { .. } => "Render",
        ProgramStep::Let { .. } => "Let",
        ProgramStep::Exists { .. } => "Exists",
        ProgramStep::Or { .. } => "Or",
        ProgramStep::Fork { .. } => "Fork",
        ProgramStep::Yield { .. } => "Yield",
        ProgramStep::Latest { .. } => "Latest",
        ProgramStep::Emit { .. } => "Emit",
    }
}

/// The name of a stage.
#[must_use]
pub(crate) const fn stage_name(stage: &FillStage) -> &'static str {
    match stage {
        FillStage::Scan { .. } => "Scan",
        FillStage::Probe { .. } => "Probe",
        FillStage::Product { .. } => "Product",
        FillStage::Use { .. } => "Use",
        FillStage::Latest { .. } => "Latest",
        FillStage::Read { .. } => "Read",
        FillStage::Keep { .. } => "Keep",
        FillStage::Call { .. } => "Call",
        FillStage::Render { .. } => "Render",
        FillStage::Walk { .. } => "Walk",
    }
}

/// The name of a fill.
#[must_use]
pub(crate) const fn fill_name(fill: &MadeFill) -> &'static str {
    match fill {
        MadeFill::Rows(..) => "Rows",
        MadeFill::Group(group, _) => match group {
            GroupFill::Count { .. } => "Group(Count)",
            GroupFill::Digest { .. } => "Group(Digest)",
            GroupFill::Ends(_) => "Group(Ends)",
            GroupFill::Places => "Group(Places)",
        },
        MadeFill::Ordered(OrderedFill::Sorted(_)) => "Ordered(Sorted)",
        MadeFill::Ordered(OrderedFill::Match(..)) => "Ordered(Match)",
        MadeFill::Graph(graph, _) => match graph {
            GraphFill::FixedPoints => "Graph(FixedPoints)",
            GraphFill::OrderedDfs => "Graph(OrderedDfs)",
            GraphFill::Worklist => "Graph(Worklist)",
            GraphFill::Cycles => "Graph(Cycles)",
        },
        MadeFill::Constant(_) => "Constant",
    }
}

/// The steps of a program by name, each with its class: `EachRow:File Read:Constant ..`. A `Fork`'s slots follow in braces.
#[must_use]
pub(crate) fn steps_said(program: &StepProgram) -> String {
    let each = program.0.iter().map(|placed| {
        let slots = if let ProgramStep::Fork { slots } = &placed.step {
            let said: Vec<String> = slots.iter().map(steps_said).collect();
            format!("{{{}}}", said.join(" | "))
        } else {
            String::new()
        };
        format!(
            "{}:{:?}{slots}",
            step_name(&placed.step),
            step_class(&placed.step)
        )
    });
    each.collect::<Vec<String>>().join(" ")
}

/// The stages of a pipeline by name, each with its class.
#[must_use]
pub(crate) fn stages_said(pipeline: &Pipeline) -> String {
    let each = pipeline.stages.iter().map(|placed| {
        format!(
            "{}:{:?}",
            stage_name(&placed.stage),
            stage_class(&placed.stage)
        )
    });
    each.collect::<Vec<String>>().join(" ")
}

/// A lowered pack in lines: its definitions and their patterns, its tables and their pipelines, its rules and their branches.
#[must_use]
pub(crate) fn described(pack: &LoweredPack) -> Vec<String> {
    let mut lines = Vec::new();
    for (number, definition) in pack.definitions.iter().enumerate() {
        lines.push(format!(
            "definition {number}: {} columns, emptiness {:?}",
            definition.formals.0.len(),
            definition.emptiness
        ));
        for pattern in &definition.patterns {
            let columns = |mask: &[crate::domain::steps::types::MadeColumn]| {
                mask.iter().map(|column| column.0).collect::<Vec<u16>>()
            };
            let form = match &pattern.form {
                PatternForm::Call(program) => format!("Call: {}", steps_said(program)),
                PatternForm::Table(table) => format!("Table {}", table.0),
            };
            lines.push(format!(
                "  pattern bound {:?} collected {:?}: {form}",
                columns(&pattern.bound.0),
                columns(&pattern.collected.0)
            ));
        }
    }
    for (number, table) in pack.tables.iter().enumerate() {
        let indexes: Vec<Vec<u16>> = table
            .indexes
            .iter()
            .map(|index| index.0.iter().map(|column| column.0).collect())
            .collect();
        lines.push(format!(
            "table {number}: {}, {} columns, indexes {indexes:?}",
            fill_name(&table.fill),
            table.width.0
        ));
        for pipeline in fill_pipelines(&table.fill) {
            lines.push(format!("  pipeline: {}", stages_said(pipeline)));
        }
    }
    for rule in &pack.rules {
        lines.push(format!(
            "rule {}: {} branches",
            rule.rule.0,
            rule.branches.len()
        ));
        for branch in &rule.branches {
            lines.push(format!("  branch: {}", steps_said(&branch.program)));
        }
    }
    lines
}

/// The words of an encoded pack by code, each with its count, in the codes' names' order.
#[must_use]
pub(crate) fn code_census(code: &ProgramCode) -> BTreeMap<String, usize> {
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    for word in &code.words {
        let count = census.entry(format!("{:?}", word.op)).or_default();
        *count = count.saturating_add(1);
    }
    census
}
