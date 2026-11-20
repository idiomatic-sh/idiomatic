//! The service of the `traversal` part: what each step and each stage touches, with no arm for "the rest", and the walk of
//! a program's steps.

use super::types::{Touched, TouchedBody};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{
    NearestEdges, PlacedStep, ProgramStep, RangeBound, Register, RegisterList, StepBody, StepMemo,
    StepOperand, StepOperands, StepProgram, StepRows,
};

/// The register an operand reads, when it is one.
fn operand_read(operand: &StepOperand, reads: &mut Vec<Register>) {
    match operand {
        StepOperand::Register(register) => reads.push(*register),
        StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => {}
    }
}

/// The registers among operands.
fn operand_reads(operands: &StepOperands, reads: &mut Vec<Register>) {
    for operand in &operands.0 {
        operand_read(operand, reads);
    }
}

/// The register a range's end reads.
fn bound_read(bound: Option<&RangeBound>, reads: &mut Vec<Register>) {
    if let Some(bound) = bound {
        operand_read(&bound.value, reads);
    }
}

/// The registers of a list.
fn listed(list: &RegisterList, registers: &mut Vec<Register>) {
    registers.extend(list.0.iter().copied());
}

/// The registers a memo is kept by.
fn memo_reads(memo: &StepMemo, reads: &mut Vec<Register>) {
    match memo {
        StepMemo::None => {}
        StepMemo::Keyed(keys) => listed(keys, reads),
    }
}

/// The definition whose table a source is, when it is one.
fn source_body(source: StepRows, bodies: &mut Vec<TouchedBody<'_>>) {
    match source {
        StepRows::Definition(definition) => bodies.push(TouchedBody::Definition(definition)),
        StepRows::Node(_) | StepRows::Filled(_) | StepRows::Made(_) | StepRows::Written(_) => {}
    }
}

/// A body: an inline one is a body to walk; a use of a definition reads its bound values and names the definition.
fn body_touched<'a>(body: &'a StepBody, touched: &mut Touched<'a>) {
    match body {
        StepBody::Inline(program) => touched.bodies.push(TouchedBody::Inline(program)),
        StepBody::Definition {
            definition,
            bound: _,
            collected: _,
            values,
        } => {
            operand_reads(values, &mut touched.reads);
            touched.bodies.push(TouchedBody::Definition(*definition));
        }
    }
}

/// What a step touches. A `Read` names its source to pick a column and runs nothing, so a definition's table it reads from
/// is no body of it.
pub(crate) fn step_touched(step: &ProgramStep) -> Touched<'_> {
    let mut touched = Touched::default();
    match step {
        ProgramStep::EachKind {
            relation: _,
            kinds: _,
            out,
        } => touched.writes.push(*out),
        ProgramStep::EachMatch {
            source,
            columns: _,
            values,
            out,
        } => {
            source_body(*source, &mut touched.bodies);
            operand_reads(values, &mut touched.reads);
            touched.writes.push(*out);
        }
        ProgramStep::EachRange {
            source,
            columns: _,
            values,
            range: _,
            low,
            high,
            out,
            range_out,
        } => {
            source_body(*source, &mut touched.bodies);
            operand_reads(values, &mut touched.reads);
            bound_read(low.as_ref(), &mut touched.reads);
            bound_read(high.as_ref(), &mut touched.reads);
            touched.writes.push(*out);
            touched.writes.extend(range_out);
        }
        ProgramStep::EachRow { source, out } => {
            source_body(*source, &mut touched.bodies);
            touched.writes.push(*out);
        }
        ProgramStep::Children {
            parent: read,
            field: _,
            out,
        }
        | ProgramStep::Axis {
            direction: _,
            start: read,
            out,
        }
        | ProgramStep::Read {
            source: _,
            column: _,
            row: read,
            out,
        }
        | ProgramStep::Render {
            node: read,
            context: _,
            out,
        } => {
            touched.reads.push(*read);
            touched.writes.push(*out);
        }
        ProgramStep::Within {
            ancestor,
            descendant,
        } => touched.reads.extend([*ancestor, *descendant]),
        ProgramStep::Nearest {
            edges,
            condition,
            element,
            start,
            out,
            memo,
        } => {
            touched.reads.push(*start);
            match edges {
                NearestEdges::Tree => {}
                NearestEdges::Derived { edges, from } => {
                    body_touched(edges, &mut touched);
                    touched.writes.push(*from);
                }
            }
            body_touched(condition, &mut touched);
            touched.writes.extend([*element, *out]);
            memo_reads(memo, &mut touched.reads);
        }
        ProgramStep::Closure {
            edges,
            from,
            seed,
            out,
        } => {
            touched.reads.push(*seed);
            body_touched(edges, &mut touched);
            touched.writes.extend([*from, *out]);
        }
        ProgramStep::Test {
            test: _,
            left,
            right,
        } => {
            operand_read(left, &mut touched.reads);
            operand_read(right, &mut touched.reads);
        }
        ProgramStep::Call {
            meaning: _,
            arguments,
            outputs,
        } => {
            operand_reads(arguments, &mut touched.reads);
            listed(outputs, &mut touched.writes);
        }
        ProgramStep::Let { from, out } => {
            operand_read(from, &mut touched.reads);
            touched.writes.push(*out);
        }
        ProgramStep::Exists {
            polarity: _,
            body,
            memo,
        } => {
            body_touched(body, &mut touched);
            memo_reads(memo, &mut touched.reads);
        }
        ProgramStep::Or {
            bodies,
            outputs,
            memo,
        } => {
            for body in bodies {
                body_touched(body, &mut touched);
            }
            listed(outputs, &mut touched.writes);
            memo_reads(memo, &mut touched.reads);
        }
        ProgramStep::Fork { slots } => touched.bodies.extend(slots.iter().map(TouchedBody::Inline)),
        ProgramStep::Yield { row } => listed(row, &mut touched.reads),
        ProgramStep::Latest {
            table: _,
            key,
            bound,
            side: _,
            out,
        } => {
            operand_reads(key, &mut touched.reads);
            operand_read(bound, &mut touched.reads);
            touched.writes.push(*out);
        }
        ProgramStep::Emit { target: _, columns } => operand_reads(columns, &mut touched.reads),
    }
    touched
}

/// What a stage touches. A probe's `out` and a use's `outputs` are its writes in every mode, as its operands are written.
pub(crate) fn stage_touched(stage: &FillStage) -> Touched<'_> {
    let mut touched = Touched::default();
    match stage {
        FillStage::Scan {
            source,
            columns: _,
            values,
            out,
        } => {
            source_body(*source, &mut touched.bodies);
            operand_reads(values, &mut touched.reads);
            touched.writes.push(*out);
        }
        FillStage::Probe {
            source,
            columns: _,
            values,
            range,
            mode: _,
            out,
        } => {
            source_body(*source, &mut touched.bodies);
            operand_reads(values, &mut touched.reads);
            if let Some(range) = range {
                operand_read(&range.low.value, &mut touched.reads);
                operand_read(&range.high.value, &mut touched.reads);
            }
            touched.writes.push(*out);
        }
        FillStage::Product { source, out } => {
            source_body(*source, &mut touched.bodies);
            touched.writes.push(*out);
        }
        FillStage::Use {
            definition,
            bound: _,
            collected: _,
            values,
            mode: _,
            outputs,
        } => {
            operand_reads(values, &mut touched.reads);
            touched.bodies.push(TouchedBody::Definition(*definition));
            listed(outputs, &mut touched.writes);
        }
        FillStage::Latest {
            table: _,
            key,
            bound,
            side: _,
            out,
        } => {
            operand_reads(key, &mut touched.reads);
            operand_read(bound, &mut touched.reads);
            touched.writes.push(*out);
        }
        FillStage::Read {
            source: _,
            column: _,
            row: read,
            out,
        }
        | FillStage::Render {
            node: read,
            context: _,
            out,
        }
        | FillStage::Walk {
            walk: _,
            start: read,
            out,
        } => {
            touched.reads.push(*read);
            touched.writes.push(*out);
        }
        FillStage::Keep {
            test: _,
            left,
            right,
        } => {
            operand_read(left, &mut touched.reads);
            operand_read(right, &mut touched.reads);
        }
        FillStage::Call {
            meaning: _,
            arguments,
            outputs,
        } => {
            operand_reads(arguments, &mut touched.reads);
            listed(outputs, &mut touched.writes);
        }
    }
    touched
}

/// Every step of a program, each once, in pre-order: a step, then the steps of its inline bodies in its operands' order. The
/// visitor takes the step and what it touches; a definition a step names is the pack's and is not gone into.
pub(crate) fn each_step<'a>(
    program: &'a StepProgram,
    visit: &mut impl FnMut(&'a PlacedStep, &Touched<'a>),
) {
    for placed in &program.0 {
        let touched = step_touched(&placed.step);
        visit(placed, &touched);
        for body in &touched.bodies {
            match body {
                TouchedBody::Inline(inner) => each_step(inner, visit),
                TouchedBody::Definition(_) => {}
            }
        }
    }
}
