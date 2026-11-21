//! The services of the `carrying` part: which registers each stage of a pipeline must carry on. A stage that gives several
//! rows for one, or keeps some rows, moves the columns written before it and read after it; no other column is touched.

use crate::domain::code::constants::{
    BOUND_CONSTANT, BUCKET, LEFT_CONSTANT, PROBE_MANY, PROBE_RANGE, RIGHT_CONSTANT, RIGHT_SET,
};
use crate::domain::code::filling::service::{
    stage_call_of, stage_keep_of, stage_latest_of, stage_probe_of, stage_product_of, stage_read_of,
    stage_scan_of, stage_use_of, stage_walk_of, stage_write_of,
};
use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::{BodyRecord, PooledOperand};
use crate::domain::code::types::{CodeWord, IndexSite, Op, PoolAt, ProgramCode};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::made_tables::operands::service::{
    call_arguments, formal_value, operand, registers_at,
};
use crate::domain::made_tables::pipelines::types::{
    OperandRun, PipelineRoom, StageKind, StageRegisters,
};
use crate::domain::made_tables::reading::service::{latest_site, source_of};
use crate::domain::made_tables::types::{Reached, ReadSource};
use crate::domain::steps::types::Register;
use ryo_relations::domain::relations::types::Count;

/// The stage a word is; none for a word of a step program.
#[must_use]
pub(crate) const fn stage_kind(op: Op) -> Option<StageKind> {
    match op {
        Op::StageScan => Some(StageKind::Scan),
        Op::StageProbe => Some(StageKind::Probe),
        Op::StageLatest => Some(StageKind::Latest),
        Op::StageRead => Some(StageKind::Read),
        Op::StageKeep => Some(StageKind::Keep),
        Op::StageCall => Some(StageKind::Call),
        Op::StageProduct => Some(StageKind::Product),
        Op::StageUse => Some(StageKind::Use),
        Op::StageRender => Some(StageKind::Render),
        Op::StageWalk => Some(StageKind::Walk),
        Op::StageWrite => Some(StageKind::Write),
        Op::End
        | Op::EachKind
        | Op::EachMatchNode
        | Op::EachMatchFilled
        | Op::EachMatchFilledMany
        | Op::EachMatchMade
        | Op::EachRange
        | Op::EachRowNode
        | Op::EachRowFilled
        | Op::EachRowMade
        | Op::Children
        | Op::ChildrenOfField
        | Op::Axis
        | Op::Within
        | Op::Nearest
        | Op::NearestDerived
        | Op::Closure
        | Op::ReadNode
        | Op::ReadFilled
        | Op::ReadMade
        | Op::Test
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants
        | Op::Call
        | Op::Render
        | Op::Let
        | Op::Exists
        | Op::Or
        | Op::Fork
        | Op::Yield
        | Op::Latest
        | Op::Emit => None,
    }
}

/// Adds the registers among a run of operands to `read`.
fn operands_read(
    code: &ProgramCode,
    operands: OperandRun,
    read: &mut Vec<Register>,
) -> Result<(), FilledError> {
    for place in 0..operands.count.0 {
        match operand(code, operands.first, Count(place))? {
            PooledOperand::Register(register) => read.push(register),
            PooledOperand::Constant(_) | PooledOperand::Constants(_) | PooledOperand::File => {}
        }
    }
    Ok(())
}

/// The registers one stage reads and the registers it writes, added to `touched`.
///
/// # Errors
///
/// The word is no stage's, or names an operand the code does not hold.
pub(crate) fn touched(
    code: &ProgramCode,
    word: CodeWord,
    touched: &mut StageRegisters,
) -> Result<(), FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Word,
    };
    let StageRegisters { read, written } = touched;
    read.clear();
    written.clear();
    match stage_kind(word.op).ok_or(wrong.clone())? {
        StageKind::Scan => written.push(stage_scan_of(word).out),
        StageKind::Product => written.push(stage_product_of(word).out),
        StageKind::Probe => {
            let stage = stage_probe_of(word);
            written.push(stage.out);
            if stage.flag.0 & PROBE_MANY.0 == 0 {
                read.push(Register(stage.probed.0));
            } else {
                let ends = if stage.flag.0 & PROBE_RANGE.0 == 0 {
                    0
                } else {
                    2
                };
                operands_read(
                    code,
                    OperandRun {
                        first: PoolAt(stage.probed.0),
                        count: Count(usize::from(stage.values.0).saturating_add(ends)),
                    },
                    read,
                )?;
            }
        }
        StageKind::Latest => {
            let stage = stage_latest_of(word);
            written.push(stage.out);
            if stage.flag.0 & BOUND_CONSTANT.0 == 0 {
                read.push(Register(stage.bound.0));
            }
            operands_read(
                code,
                OperandRun {
                    first: PoolAt(stage.operands.0.saturating_add(1)),
                    count: Count(usize::from(stage.keys.0)),
                },
                read,
            )?;
        }
        StageKind::Read => {
            let stage = stage_read_of(word);
            read.push(stage.row);
            written.push(stage.out);
        }
        StageKind::Keep => {
            let stage = stage_keep_of(word);
            if stage.flag.0 & LEFT_CONSTANT.0 == 0 {
                read.push(Register(stage.left.0));
            }
            if stage.flag.0 & (RIGHT_CONSTANT.0 | RIGHT_SET.0) == 0 {
                read.push(Register(stage.right.0));
            }
        }
        StageKind::Walk => {
            let stage = stage_walk_of(word);
            read.push(stage.start);
            written.push(stage.out);
        }
        StageKind::Use => {
            let stage = stage_use_of(word);
            written.extend(registers_at(code, stage.registers));
            match body_at(&code.pool, stage.body).map(|body| body.record) {
                Some(BodyRecord::Call { bound, formals, .. }) => {
                    for place in 0..bound.0 {
                        let value = formal_value(
                            code,
                            formals,
                            Count(usize::try_from(place).unwrap_or(usize::MAX)),
                        )?;
                        match value {
                            PooledOperand::Register(register) => read.push(register),
                            PooledOperand::Constant(_)
                            | PooledOperand::Constants(_)
                            | PooledOperand::File => {}
                        }
                    }
                }
                Some(BodyRecord::Inline { .. } | BodyRecord::Table { .. }) | None => {
                    return Err(wrong);
                }
            }
        }
        StageKind::Write => {
            let stage = stage_write_of(word);
            operands_read(
                code,
                OperandRun {
                    first: stage.operands,
                    count: Count(usize::from(stage.columns.0)),
                },
                read,
            )?;
        }
        StageKind::Call => {
            let stage = stage_call_of(word);
            written.extend(registers_at(code, stage.outputs));
            operands_read(code, call_arguments(code, stage.arguments)?, read)?;
        }
        // A rendering's registers are read when the stage is built.
        StageKind::Render => {}
    }
    Ok(())
}

/// Writes into the room, for each stage of a pipeline, the registers a stage before it wrote and a stage after it reads:
/// what the stage carries on when it changes which rows the batch holds.
///
/// # Errors
///
/// A word that is no stage's, or an operand the code does not hold.
pub(crate) fn carried(
    code: &ProgramCode,
    words: &[CodeWord],
    room: &mut PipelineRoom,
) -> Result<(), FilledError> {
    let mut all = std::mem::take(&mut room.touched);
    all.resize_with(words.len(), StageRegisters::default);
    for (word, each) in words.iter().zip(all.iter_mut()) {
        touched(code, *word, each)?;
    }
    room.live.clear();
    room.starts.clear();
    for at in 0..all.len() {
        room.starts.push(Count(room.live.len()));
        let (before, after) = (
            all.get(..at).unwrap_or_default(),
            all.get(at.saturating_add(1)..).unwrap_or_default(),
        );
        for register in before.iter().flat_map(|stage| stage.written.iter()) {
            let read_after = after.iter().any(|stage| stage.read.contains(register));
            if read_after
                && !room
                    .live
                    .get(room.starts.last().map_or(0, |start| start.0)..)
                    .unwrap_or_default()
                    .contains(register)
            {
                room.live.push(*register);
            }
        }
    }
    room.starts.push(Count(room.live.len()));
    room.touched = all;
    Ok(())
}

/// What a source the fill side reaches as a thing reached: a relation, a table, the node table.
const fn reached_of(source: ReadSource) -> Reached {
    match source {
        ReadSource::Nodes(_) => Reached::Nodes,
        ReadSource::Filled(number) => Reached::Relation(number),
        ReadSource::Made(table) | ReadSource::Written(table) => Reached::Table(table),
    }
}

/// What one stage reads directly, when it reads a source: a scan and a product their source or their bucket's index, a
/// probe and a `Latest` their index. A `Read` reads by a handle another stage found, so its source is there.
pub(crate) fn stage_reach(
    code: &ProgramCode,
    word: CodeWord,
) -> Result<Option<Reached>, FilledError> {
    Ok(
        match stage_kind(word.op).ok_or(FilledError::Code {
            named: MissingCode::Word,
        })? {
            StageKind::Scan => {
                let stage = stage_scan_of(word);
                Some(if stage.flag.0 & BUCKET.0 == 0 {
                    reached_of(source_of(stage.flag, stage.read))
                } else {
                    Reached::Index(IndexSite(stage.read.0))
                })
            }
            StageKind::Product => {
                let stage = stage_product_of(word);
                Some(reached_of(source_of(stage.flag, stage.read)))
            }
            StageKind::Probe => Some(Reached::Index(stage_probe_of(word).index)),
            StageKind::Latest => {
                let stage = stage_latest_of(word);
                let site = latest_site(code, stage.operands)?;
                Some(Reached::Index(site))
            }
            StageKind::Walk => Some(Reached::Nodes),
            StageKind::Use
            | StageKind::Read
            | StageKind::Keep
            | StageKind::Call
            | StageKind::Render
            | StageKind::Write => None,
        },
    )
}
