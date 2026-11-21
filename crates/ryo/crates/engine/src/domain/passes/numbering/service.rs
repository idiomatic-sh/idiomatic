//! The service of the `numbering` pass: every step and stage of a unit takes its plan point, its place in the unit's
//! pre-order. A unit is what is counted as one program: a definition's pattern in call form; a rule's own program, all its
//! branches; a table's fill, all its pipelines. The order is the shared walk's (`lowered::service::each_touched`): a step, then
//! the steps of its inline bodies in its operands' order; a fill's pipelines in its inputs' order. That walk lends a pack it
//! cannot write, so the pass goes into the same bodies and the same pipelines by its own two matches, which name every step
//! and every fill; its cases read the points back through the shared walk's order.

use crate::domain::lowered::types::{FillInput, LoweredPack, MadeFill, OrderedFill, PatternForm};
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::types::{NearestEdges, PlanPoint, ProgramStep, StepBody, StepProgram};

/// Takes the next point of a unit: `next` is the point the next step or stage takes, none once the unit holds more than a
/// point can number.
fn taken(next: &mut Option<PlanPoint>) -> Option<PlanPoint> {
    let point = *next;
    *next = point
        .and_then(|taken| taken.0.checked_add(1))
        .map(PlanPoint);
    point
}

/// The steps of a body, when they are written in place: a use of a definition is the pack's, numbered as its own unit.
fn inline(body: &mut StepBody) -> Option<&mut StepProgram> {
    match body {
        StepBody::Inline(program) => Some(program),
        StepBody::Definition { .. } => None,
    }
}

/// The inline bodies of a step, in the order the shared walk goes into them.
fn bodies(step: &mut ProgramStep) -> Vec<&mut StepProgram> {
    match step {
        ProgramStep::Nearest {
            edges, condition, ..
        } => {
            let derived = match edges {
                NearestEdges::Tree(_) => None,
                NearestEdges::Derived { edges, .. } => inline(edges),
            };
            derived.into_iter().chain(inline(condition)).collect()
        }
        ProgramStep::Closure { edges: body, .. } | ProgramStep::Exists { body, .. } => {
            inline(body).into_iter().collect()
        }
        ProgramStep::Or { bodies, .. } => bodies.iter_mut().filter_map(inline).collect(),
        ProgramStep::Fork { slots } => slots.iter_mut().collect(),
        ProgramStep::EachKind { .. }
        | ProgramStep::EachMatch { .. }
        | ProgramStep::EachRange { .. }
        | ProgramStep::EachRow { .. }
        | ProgramStep::Children { .. }
        | ProgramStep::Axis { .. }
        | ProgramStep::Within { .. }
        | ProgramStep::Read { .. }
        | ProgramStep::Test { .. }
        | ProgramStep::Call { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. }
        | ProgramStep::Yield { .. }
        | ProgramStep::Latest { .. }
        | ProgramStep::Emit { .. } => Vec::new(),
    }
}

/// Numbers a program's steps from `next` on, in pre-order: a step, then the steps of its inline bodies.
fn numbered(program: &mut StepProgram, next: &mut Option<PlanPoint>) {
    for placed in &mut program.0 {
        placed.point = taken(next);
        for body in bodies(&mut placed.step) {
            numbered(body, next);
        }
    }
}

/// The pipelines of a fill, in its inputs' order.
fn pipelines(fill: &mut MadeFill) -> Vec<&mut Pipeline> {
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

/// Every step and stage takes its plan point: its place in its unit's pre-order, from nought in each unit. A point is the
/// step's own from here on: a pass that moves a placed step moves its point with it, and a step a later pass adds has none.
pub(crate) fn passed(pack: &mut LoweredPack) {
    let patterns = pack
        .definitions
        .iter_mut()
        .flat_map(|definition| &mut definition.patterns);
    for pattern in patterns {
        match &mut pattern.form {
            PatternForm::Call(program) => numbered(program, &mut Some(PlanPoint(0))),
            // A pattern in table form has no step of its own: its table's fill is the unit that runs.
            PatternForm::Table(_) => {}
        }
    }
    for rule in &mut pack.rules {
        let mut next = Some(PlanPoint(0));
        for branch in &mut rule.branches {
            numbered(&mut branch.program, &mut next);
        }
    }
    for table in &mut pack.tables {
        let mut next = Some(PlanPoint(0));
        for pipeline in pipelines(&mut table.fill) {
            for placed in &mut pipeline.stages {
                placed.point = taken(&mut next);
            }
        }
    }
}
