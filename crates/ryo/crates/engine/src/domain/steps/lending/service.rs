//! The service of the `lending` part: the inline bodies of a step, to write, by a `match` that names every step; and the
//! walk that lends every step of a program once, in the pre-order of `traversal::service::each_step`.

use crate::domain::steps::types::{NearestEdges, PlacedStep, ProgramStep, StepBody, StepProgram};

/// The steps of a body, when they are written in place: a use of a definition is the pack's and is not gone into.
fn inline(body: &mut StepBody) -> Option<&mut StepProgram> {
    match body {
        StepBody::Inline(program) => Some(program),
        StepBody::Definition { .. } => None,
    }
}

/// The inline bodies of a step, to write, in the order `traversal::service::step_touched` lists them: a nearest walk's
/// derived edges, then its condition; a closure's edges; the body of an `Exists`; the bodies of an `Or` in their order; the
/// slots of a `Fork` in theirs.
fn inline_bodies(step: &mut ProgramStep) -> Vec<&mut StepProgram> {
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

/// Every step of a program, each once, lent to write, in the pre-order of `traversal::service::each_step`: a step, then the
/// steps of its inline bodies in its operands' order; a definition a step names is the pack's and is not gone into. The
/// visitor has the step before the walk goes into its bodies, so the bodies walked are the ones the step holds after the
/// visit.
pub(crate) fn each_step_mut(program: &mut StepProgram, visit: &mut impl FnMut(&mut PlacedStep)) {
    for placed in &mut program.0 {
        visit(placed);
        for body in inline_bodies(&mut placed.step) {
            each_step_mut(body, visit);
        }
    }
}
