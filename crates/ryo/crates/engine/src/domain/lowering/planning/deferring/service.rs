//! The services of the `deferring` part. The emitter writes the read of every cell a unit will need right after the
//! access that gives the row. A cell that only a later stage needs, or only the unit's own row at its end, is then read
//! for every row, also for the rows a test, a keep or a decision between drops. This pass moves each read down to just
//! before the first access that needs its cell, and never past an access that may give several rows for one: there a
//! later read would be made once for each of them. Nothing but the place of a read changes.

use crate::domain::lowering::planning::deferring::types::AccessRows;
use crate::domain::lowering::planning::types::{NearestWalked, WalkEdges, WrittenAccess};
use crate::domain::steps::calls::service::call_shape;
use crate::domain::steps::calls::types::CallRows;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{Register, StepOperand};

/// The registers among operands.
fn registers<'o>(operands: impl IntoIterator<Item = &'o StepOperand>) -> Vec<Register> {
    let read = operands.into_iter().filter_map(|operand| match operand {
        StepOperand::Register(register) => Some(*register),
        StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => None,
    });
    read.collect()
}

/// The registers an access reads, the ones its own accesses written in place read among them.
fn read_by(access: &WrittenAccess) -> Vec<Register> {
    match access {
        WrittenAccess::Rows { values, .. } | WrittenAccess::Use { values, .. } => {
            registers(values.0.iter())
        }
        WrittenAccess::Latest { key, bound, .. } => registers(key.0.iter().chain([bound])),
        WrittenAccess::Read { row, .. } => vec![*row],
        WrittenAccess::Test { left, right, .. } => registers([left, right]),
        WrittenAccess::Axis { start, .. } => vec![*start],
        WrittenAccess::Children { parent, .. } => vec![*parent],
        WrittenAccess::Within {
            ancestor,
            descendant,
            ..
        } => vec![*ancestor, *descendant],
        WrittenAccess::Nearest {
            edges,
            condition,
            start,
            ..
        } => {
            let mut read = vec![*start];
            if let NearestWalked::Derived { edges, .. } = edges {
                read.extend(edges_read(edges));
            }
            read.extend(edges_read(condition));
            read
        }
        WrittenAccess::Closure { edges, seed, .. } => {
            let mut read = vec![*seed];
            read.extend(edges_read(edges));
            read
        }
        WrittenAccess::Call { arguments, .. } => registers(arguments.0.iter()),
        WrittenAccess::Render { node, .. } => vec![*node],
        WrittenAccess::Let { from, .. } => registers([from]),
    }
}

/// The registers a walk's edges read.
fn edges_read(edges: &WalkEdges) -> Vec<Register> {
    match edges {
        WalkEdges::Use(used) => registers(used.values.0.iter()),
        WalkEdges::Inline { steps, .. } => steps.iter().flat_map(read_by).collect(),
    }
}

/// Whether an access gives at most one row for a row it reads: a read, a test, a decision, the nearest rows of an ordered
/// table, a call that gives no more than one row. A read may wait past it; past any other access it would be repeated.
fn rows_given(access: &WrittenAccess) -> AccessRows {
    match access {
        WrittenAccess::Read { .. }
        | WrittenAccess::Test { .. }
        | WrittenAccess::Within { .. }
        | WrittenAccess::Latest { .. }
        | WrittenAccess::Nearest { .. }
        | WrittenAccess::Render { .. }
        | WrittenAccess::Let { .. } => AccessRows::AtMostOne,
        WrittenAccess::Use { mode, .. } | WrittenAccess::Rows { mode, .. } => match mode {
            UseMode::Present | UseMode::Absent => AccessRows::AtMostOne,
            UseMode::Rows => AccessRows::Several,
        },
        WrittenAccess::Call { meaning, .. } => match call_shape(*meaning).rows {
            CallRows::One | CallRows::AtMostOne => AccessRows::AtMostOne,
            CallRows::Several => AccessRows::Several,
        },
        WrittenAccess::Axis { .. }
        | WrittenAccess::Children { .. }
        | WrittenAccess::Closure { .. } => AccessRows::Several,
    }
}

/// The accesses of a unit with each read moved down to just before the first access that reads its cell, or to the end
/// when only the unit's own row does; a read is written before any access that may give several rows. The reads keep
/// their order among themselves.
#[must_use]
pub(crate) fn deferred(written: Vec<WrittenAccess>) -> Vec<WrittenAccess> {
    let mut placed = Vec::with_capacity(written.len());
    let mut waiting: Vec<WrittenAccess> = Vec::new();
    for access in written {
        if matches!(access, WrittenAccess::Read { .. }) {
            waiting.push(access);
            continue;
        }
        let needed = read_by(&access);
        let given = rows_given(&access);
        let (due, kept): (Vec<WrittenAccess>, Vec<WrittenAccess>) =
            waiting.into_iter().partition(|read| match read {
                WrittenAccess::Read { out, .. } => {
                    given == AccessRows::Several || needed.contains(out)
                }
                WrittenAccess::Rows { .. }
                | WrittenAccess::Use { .. }
                | WrittenAccess::Latest { .. }
                | WrittenAccess::Test { .. }
                | WrittenAccess::Axis { .. }
                | WrittenAccess::Children { .. }
                | WrittenAccess::Within { .. }
                | WrittenAccess::Nearest { .. }
                | WrittenAccess::Closure { .. }
                | WrittenAccess::Call { .. }
                | WrittenAccess::Render { .. }
                | WrittenAccess::Let { .. } => true,
            });
        waiting = kept;
        placed.extend(due);
        placed.push(access);
    }
    placed.extend(waiting);
    placed
}
