//! The services of the `forms` part: one access in each emitter's form: a step of a program, a stage of a pipeline.

use crate::domain::lowering::planning::types::Enumerated;
use crate::domain::lowering::planning::types::KindAnchor;
use crate::domain::lowering::planning::types::WrittenAccess;
use crate::domain::lowering::planning::types::{NearestWalked, WalkEdges};
use crate::domain::steps::stages::types::{FillStage, TreeWalk, UseMode};
use crate::domain::steps::types::{
    NearestEdges, Polarity, ProgramStep, StepBody, StepMemo, StepOperand,
};
use crate::domain::steps::types::{PlacedStep, RegisterList, StepProgram};

/// Steps written in place as a body: each row of them ends in a `Yield` of `row`.
fn inline_body(steps: Vec<ProgramStep>, row: RegisterList) -> StepBody {
    let written = steps.into_iter().chain([ProgramStep::Yield { row }]);
    StepBody::Inline(StepProgram(
        written
            .map(|step| PlacedStep { point: None, step })
            .collect(),
    ))
}

/// A source asked whether it holds a row: the read is the body of an `Exists`, each row of it a `Yield` of nothing.
fn asked(polarity: Polarity, read: ProgramStep) -> ProgramStep {
    ProgramStep::Exists {
        polarity,
        body: inline_body(vec![read], RegisterList::default()),
        memo: StepMemo::None,
    }
}

/// One access as a step of a program.
pub(crate) fn step_of(access: WrittenAccess) -> ProgramStep {
    match access {
        WrittenAccess::Rows {
            source,
            columns,
            values,
            kinds,
            mode,
            out,
        } => {
            let read = match kinds {
                Some(KindAnchor { relation, kinds }) => ProgramStep::EachKind {
                    relation,
                    kinds,
                    out,
                },
                None if columns.0.is_empty() => ProgramStep::EachRow { source, out },
                None => ProgramStep::EachMatch {
                    source,
                    columns,
                    values,
                    out,
                },
            };
            match mode {
                UseMode::Rows => read,
                UseMode::Present => asked(Polarity::Present, read),
                UseMode::Absent => asked(Polarity::Absent, read),
            }
        }
        WrittenAccess::Use {
            definition,
            bound,
            collected,
            values,
            outputs,
            mode,
        } => {
            let body = StepBody::Definition {
                definition,
                bound,
                collected,
                values,
            };
            match mode {
                UseMode::Present => ProgramStep::Exists {
                    polarity: Polarity::Present,
                    body,
                    memo: StepMemo::None,
                },
                UseMode::Absent => ProgramStep::Exists {
                    polarity: Polarity::Absent,
                    body,
                    memo: StepMemo::None,
                },
                UseMode::Rows => ProgramStep::Or {
                    bodies: [body].into(),
                    outputs,
                    memo: StepMemo::None,
                },
            }
        }
        WrittenAccess::Latest {
            table,
            key,
            bound,
            side,
            out,
        } => ProgramStep::Latest {
            table,
            key,
            bound,
            side,
            out,
        },
        WrittenAccess::Read {
            source,
            column,
            row,
            out,
        } => ProgramStep::Read {
            source,
            column,
            row,
            out,
        },
        WrittenAccess::Test { test, left, right } => ProgramStep::Test { test, left, right },
        WrittenAccess::Axis {
            relation,
            direction,
            start,
            out,
        } => ProgramStep::Axis {
            relation,
            direction,
            start,
            out,
        },
        WrittenAccess::Children {
            relation,
            parent,
            out,
        } => ProgramStep::Children {
            relation,
            parent,
            field: None,
            out,
        },
        WrittenAccess::Within {
            relation,
            ancestor,
            descendant,
        } => ProgramStep::Within {
            relation,
            ancestor,
            descendant,
        },
        WrittenAccess::Nearest {
            edges,
            condition,
            element,
            start,
            out,
        } => ProgramStep::Nearest {
            edges: match edges {
                NearestWalked::Tree(relation) => NearestEdges::Tree(relation),
                NearestWalked::Derived { edges, from } => NearestEdges::Derived {
                    edges: walk_body(edges),
                    from,
                },
            },
            // The condition asks whether a row is there: read in place, each row is a `Yield` of nothing.
            condition: match condition {
                WalkEdges::Use(_) => walk_body(condition),
                WalkEdges::Inline { steps, .. } => inline_body(
                    steps.into_iter().map(step_of).collect(),
                    RegisterList::default(),
                ),
            },
            element,
            start,
            out,
            memo: StepMemo::None,
        },
        WrittenAccess::Closure {
            edges,
            from,
            seed,
            out,
        } => ProgramStep::Closure {
            edges: walk_body(edges),
            from,
            seed,
            out,
        },
        WrittenAccess::Call {
            meaning,
            arguments,
            outputs,
        } => ProgramStep::Call {
            meaning,
            arguments,
            outputs,
        },
        WrittenAccess::Render { node, context, out } => ProgramStep::Render { node, context, out },
        WrittenAccess::Let { from, out } => ProgramStep::Let { from, out },
    }
}

/// One access as a stage of a pipeline: the first enumeration is the scan; a later read by a register is a probe, and one
/// with no register is every row of its source, a product. None for a `Let`, which no pipeline holds.
pub(crate) fn stage_of(access: WrittenAccess, first: Enumerated) -> Option<FillStage> {
    Some(match access {
        WrittenAccess::Rows {
            source,
            columns,
            values,
            mode,
            out,
            ..
        } => {
            let keyed = values
                .0
                .iter()
                .any(|value| matches!(value, StepOperand::Register(_)));
            // A source asked whether it holds a row is a probe in that mode, whatever its values; it is no scan.
            let decides = mode != UseMode::Rows;
            match (first, keyed || decides) {
                (Enumerated::NotYet, _) if decides => return None,
                (Enumerated::NotYet, _) => FillStage::Scan {
                    source,
                    columns,
                    values,
                    out,
                },
                (Enumerated::Already, true) => FillStage::Probe {
                    source,
                    columns,
                    values,
                    range: None,
                    mode,
                    out,
                },
                (Enumerated::Already, false) => FillStage::Product { source, out },
            }
        }
        WrittenAccess::Use {
            definition,
            bound,
            collected,
            values,
            outputs,
            mode,
        } => FillStage::Use {
            definition,
            bound,
            collected,
            values,
            mode,
            outputs,
        },
        WrittenAccess::Latest {
            table,
            key,
            bound,
            side,
            out,
        } => FillStage::Latest {
            table,
            key,
            bound,
            side,
            out,
        },
        WrittenAccess::Read {
            source,
            column,
            row,
            out,
        } => FillStage::Read {
            source,
            column,
            row,
            out,
        },
        WrittenAccess::Test { test, left, right } => FillStage::Keep { test, left, right },
        WrittenAccess::Axis {
            relation,
            direction,
            start,
            out,
        } => FillStage::Walk {
            relation,
            walk: TreeWalk::Axis(direction),
            start,
            out,
        },
        WrittenAccess::Children {
            relation,
            parent,
            out,
        } => FillStage::Walk {
            relation,
            walk: TreeWalk::Children(None),
            start: parent,
            out,
        },
        WrittenAccess::Call {
            meaning,
            arguments,
            outputs,
        } => FillStage::Call {
            meaning,
            arguments,
            outputs,
        },
        WrittenAccess::Render { node, context, out } => FillStage::Render { node, context, out },
        // A pipeline holds no test of containment, no walk with a condition, no closure and no `Let`.
        WrittenAccess::Within { .. }
        | WrittenAccess::Nearest { .. }
        | WrittenAccess::Closure { .. }
        | WrittenAccess::Let { .. } => return None,
    })
}

/// A closure's edges as the body its step runs: a keyed use of a definition, or the accesses written in place, each row
/// ending in a `Yield` of the next element.
fn walk_body(edges: WalkEdges) -> StepBody {
    match edges {
        WalkEdges::Use(used) => StepBody::Definition {
            definition: used.definition,
            bound: used.bound,
            collected: used.collected,
            values: used.values,
        },
        WalkEdges::Inline { steps, gives } => inline_body(
            steps.into_iter().map(step_of).collect(),
            RegisterList([gives].into()),
        ),
    }
}
