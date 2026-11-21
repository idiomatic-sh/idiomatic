//! The services of the `fanning` part: one stage that gives several rows for one, over one batch from the batch's cursor
//! on. Each reads its word through the code's readers, then calls the one loop of its kind in `stages`.

use crate::domain::code::constants::{
    BOUND_CONSTANT, BUCKET, OWN_TREE, SIDE, WALK_AXIS, WALK_DIRECTION,
};
use crate::domain::code::filling::service::{
    stage_latest_of, stage_probe_of, stage_product_of, stage_walk_of,
};
use crate::domain::code::filling::types::ScanWord;
use crate::domain::code::service::list;
use crate::domain::code::small_codes::service::{direction_of, side_of};
use crate::domain::code::types::{CodeWord, IndexSite, PoolAt, WordFlag, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::indexes::reading::service::run_of;
use crate::domain::indexes::types::RowRun;
use crate::domain::made_tables::constants::BATCH_ROWS;
use crate::domain::made_tables::operands::service::{column_of, constant_at, key_of};
use crate::domain::made_tables::pipelines::types::{BatchLevel, FanKind, OperandRun};
use crate::domain::made_tables::reading::service::{
    index_at, latest_site, made_slot, rows_in, site_at, site_source, source_of, tag_of,
};
use crate::domain::made_tables::staging::service::{WRONG_WORD, probe_key, probe_range};
use crate::domain::made_tables::staging::types::BatchFrame;
use crate::domain::made_tables::types::ScanRead;
use crate::domain::stages::nodes::service::node_tag;
use crate::domain::stages::ranging::service::probe_between;
use crate::domain::stages::service::{latest, probe, product};
use crate::domain::stages::types::{BatchKey, BatchValue, Batches};
use crate::domain::stages::using::service::spread;
use crate::domain::stages::walking::service::{along, children};
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, NodeExtent};

/// A stage that gives several rows for one, over one batch from the batch's cursor on: the rows it gave are the batch's
/// `fan`. Answers whether every row in was read.
///
/// # Errors
///
/// The word names an operand, a site or a table the code or the file does not hold.
pub(crate) fn fan_stage(
    frame: BatchFrame<'_>,
    kind: FanKind,
    word: CodeWord,
    level: &mut BatchLevel,
    key: &mut Vec<CellWord>,
    spans: &[RowRun],
) -> Result<Batches, FilledError> {
    let BatchLevel {
        columns, fan, rows, ..
    } = level;
    let code = frame.programs.code;
    fan.up.clear();
    fan.out.clear();
    fan.limit = BATCH_ROWS;
    fan.tag = CellWord(0);
    let mut parts = Vec::new();
    match kind {
        FanKind::Probe => {
            let stage = stage_probe_of(word);
            fan.tag = tag_of(frame.file, site_source(code, site_at(code, stage.index)?)?);
            probe_key(frame, stage, columns, key, &mut parts)?;
            let asked = BatchKey {
                cells: key,
                parts: &parts,
            };
            let index = index_at(frame.store, stage.index);
            Ok(match probe_range(frame, stage, columns)? {
                None => probe(index, asked, *rows, fan),
                Some(range) => probe_between(index, asked, range, *rows, fan),
            })
        }
        FanKind::Product => {
            let stage = stage_product_of(word);
            let source = source_of(stage.flag, stage.read);
            let run = rows_in(frame.store, frame.nodes, source);
            fan.tag = tag_of(frame.file, source);
            Ok(product(*rows, run.start, run.end, fan))
        }
        FanKind::Latest => {
            let stage = stage_latest_of(word);
            let site = latest_site(code, stage.operands)?;
            let table = MadeTableId(site_at(code, site)?.number.0);
            let sorted = made_slot(frame.store, table).ok_or(FilledError::Code {
                named: MissingCode::Table,
            })?;
            key_of(
                columns,
                code,
                frame.file,
                OperandRun {
                    first: PoolAt(stage.operands.0.saturating_add(1)),
                    count: Count(usize::from(stage.keys.0)),
                },
                key,
                &mut parts,
            )?;
            let bound = if stage.flag.0 & BOUND_CONSTANT.0 == 0 {
                BatchValue::Column(column_of(columns, Register(stage.bound.0)))
            } else {
                BatchValue::Constant(constant_at(code, stage.bound)?)
            };
            let side = side_of(WordFlag(stage.flag.0 & SIDE.0)).ok_or(WRONG_WORD)?;
            Ok(latest(
                index_at(frame.store, site),
                &sorted.rows.orders,
                BatchKey {
                    cells: key,
                    parts: &parts,
                },
                bound,
                side,
                *rows,
                fan,
            ))
        }
        FanKind::Walk => {
            let stage = stage_walk_of(word);
            let extent = if stage.flag.0 & OWN_TREE.0 == 0 {
                NodeExtent::Embedded
            } else {
                NodeExtent::Own
            };
            let starts = column_of(columns, stage.start);
            fan.tag = node_tag(frame.file);
            if stage.flag.0 & WALK_AXIS.0 == 0 {
                return Ok(children(
                    frame.nodes,
                    extent,
                    stage.field,
                    starts,
                    *rows,
                    fan,
                ));
            }
            let direction =
                direction_of(WordFlag((stage.flag.0 & WALK_DIRECTION.0) >> 1)).ok_or(WRONG_WORD)?;
            Ok(along(frame.nodes, extent, direction, starts, *rows, fan))
        }
        FanKind::Use | FanKind::Call => Ok(spread(spans, *rows, fan)),
    }
}

/// What a pipeline's scan reads, as the store is: every row of its source, or the rows of one key of an index of it.
///
/// # Errors
///
/// The word names a site or a constant the code does not hold.
pub(crate) fn scan_read(frame: BatchFrame<'_>, stage: ScanWord) -> Result<ScanRead, FilledError> {
    let code = frame.programs.code;
    if stage.flag.0 & BUCKET.0 == 0 {
        let source = source_of(stage.flag, stage.read);
        let rows = rows_in(frame.store, frame.nodes, source);
        return Ok(ScanRead {
            source,
            site: None,
            rows,
            count: Count(rows.end.0.saturating_sub(rows.start.0)),
            tag: tag_of(frame.file, source),
        });
    }
    let site = IndexSite(stage.read.0);
    let key: Result<Vec<CellWord>, FilledError> = list(&code.pool, WordSlot(stage.constants.0))
        .iter()
        .map(|number| constant_at(code, *number))
        .collect();
    let rows = run_of(index_at(frame.store, site), &key?);
    let source = site_source(code, site_at(code, site)?)?;
    Ok(ScanRead {
        source,
        site: Some(site),
        rows,
        count: Count(rows.end.0.saturating_sub(rows.start.0)),
        tag: tag_of(frame.file, source),
    })
}
