//! The services of the `forms` part: one access in each emitter's form: a step of a program, a stage of a pipeline.

use crate::domain::lowering::planning::types::Enumerated;
use crate::domain::lowering::planning::types::KindAnchor;
use crate::domain::lowering::planning::types::WrittenAccess;
use crate::domain::steps::stages::types::{FillStage, TreeWalk, UseMode};
use crate::domain::steps::types::{
    ColumnMask, NearestEdges, Polarity, ProgramStep, StepBody, StepMemo, StepOperand, StepOperands,
};

/// One access as a step of a program.
pub(crate) fn step_of(access: WrittenAccess) -> ProgramStep {
    match access {
        WrittenAccess::Rows {
            source,
            columns,
            values,
            kinds,
            out,
        } => match kinds {
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
        },
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
            relation,
            domain,
            column,
            element,
            start,
            out,
        } => ProgramStep::Nearest {
            edges: NearestEdges::Tree(relation),
            condition: StepBody::Definition {
                definition: domain,
                bound: ColumnMask([column].into()),
                collected: ColumnMask::default(),
                values: StepOperands([StepOperand::Register(element)].into()),
            },
            element,
            start,
            out,
            memo: StepMemo::None,
        },
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
            out,
            ..
        } => {
            let keyed = values
                .0
                .iter()
                .any(|value| matches!(value, StepOperand::Register(_)));
            match (first, keyed) {
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
                    mode: UseMode::Rows,
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
        // A pipeline holds no test of containment, no walk with a condition and no `Let`.
        WrittenAccess::Within { .. }
        | WrittenAccess::Nearest { .. }
        | WrittenAccess::Let { .. } => return None,
    })
}
