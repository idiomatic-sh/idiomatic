//! The services of the `pipelines` part: one pipeline run from its scan to its write, over batches. The scan hands batches
//! of rows as columns; each stage after it is one loop over the batch; a stage that gives several rows for one fills the
//! batch of the next depth and the rest of the pipeline runs over it, as often as the stage has rows to give. What a
//! pipeline reads is there before it starts: nothing is made, filled or indexed here but through a `Use` stage's callback.

use crate::domain::code::constants::BUCKET;
use crate::domain::code::filling::service::{
    stage_latest_of, stage_probe_of, stage_product_of, stage_scan_of, stage_use_of, stage_walk_of,
};
use crate::domain::code::filling::types::ScanWord;
use crate::domain::code::service::list;
use crate::domain::code::types::{CodeEntry, CodeWord, IndexSite, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::UnbuiltFill;
use crate::domain::indexes::reading::service::{rows_of, run_of};
use crate::domain::indexes::types::ColumnView;
use crate::domain::made_tables::carrying::service::{carried, stage_kind};
use crate::domain::made_tables::constants::BATCH_ROWS;
use crate::domain::made_tables::fanning::service::{fan_stage, tag_of};
use crate::domain::made_tables::operands::service::{column_of, constant_at, registers_at};
use crate::domain::made_tables::pipelines::types::{BatchLevel, FanKind, PipelineRoom, StageKind};
use crate::domain::made_tables::reading::service::{
    index_at, pipeline_words, rows_in, site_at, site_source, source_of,
};
use crate::domain::made_tables::staging::service::{
    WRONG_WORD, keep_stage, mode, probe_keep_stage, read_stage, write_stage,
};
use crate::domain::made_tables::staging::types::{BatchFrame, KeepScratch};
use crate::domain::made_tables::types::{FillFrame, PatternRows};
use crate::domain::made_tables::types::{PipeRun, ScanRead};
use crate::domain::made_tables::using::service::use_asked;
use crate::domain::stages::service::{compact, gather, read, scan, scan_bucket};
use crate::domain::stages::types::{Batches, FanCursor, HandleRun, KeptWhen};
use crate::domain::stages::using::service::keep_marked;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// What a stage reads of the frame while it runs: the store as it is.
fn batch_frame<'s>(frame: &'s FillFrame<'_>) -> BatchFrame<'s> {
    BatchFrame {
        programs: frame.programs,
        store: frame.store,
        nodes: frame.nodes,
        file: frame.file,
        text: frame.text,
    }
}

/// The batch at `depth`, with a column for each register of the pack.
fn level_at(room: &mut PipelineRoom, depth: Count, registers: Count) -> Option<&mut BatchLevel> {
    if room.levels.len() <= depth.0 {
        room.levels
            .resize_with(depth.0.saturating_add(1), BatchLevel::default);
    }
    let level = room.levels.get_mut(depth.0)?;
    if level.columns.len() < registers.0 {
        level.columns.resize_with(registers.0, Vec::new);
    }
    Some(level)
}

/// The column of a register, to write.
fn column_mut(columns: &mut [Vec<CellWord>], register: Register) -> Option<&mut Vec<CellWord>> {
    columns.get_mut(usize::try_from(register.0).unwrap_or(usize::MAX))
}

/// After a stage that keeps some rows: the columns the stage carries on hold the rows kept, and the batch has that many.
fn kept(room: &mut PipelineRoom, at: Count, depth: Count) {
    let PipelineRoom {
        levels,
        live,
        starts,
        ..
    } = room;
    let Some(level) = levels.get_mut(depth.0) else {
        return;
    };
    let start = starts.get(at.0).map_or(0, |start| start.0);
    let end = starts
        .get(at.0.saturating_add(1))
        .map_or(start, |end| end.0);
    for register in live.get(start..end).unwrap_or_default() {
        if let Some(column) = column_mut(&mut level.columns, *register) {
            compact(column, &level.keep);
        }
    }
    level.rows = Count(level.keep.len());
}

/// What a pipeline's scan reads, as the store is: every row of its source, or the rows of one key of an index of it.
///
/// # Errors
///
/// The word names a site or a constant the code does not hold.
pub(crate) fn scan_run(frame: BatchFrame<'_>, stage: ScanWord) -> Result<ScanRead, FilledError> {
    let code = &frame.programs.code;
    if stage.flag.0 & BUCKET.0 == 0 {
        let source = source_of(stage.flag, stage.read);
        let rows = rows_in(frame.store, frame.nodes, source);
        return Ok(ScanRead {
            source,
            site: None,
            rows,
            count: Count(rows.end.0.saturating_sub(rows.start.0)),
            tag: tag_of(frame, source),
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
        tag: tag_of(frame, source),
    })
}

/// Runs the pipeline that starts at `entry`: each batch of its scan through its stages into `run.into`. The rows its
/// stages read and gave are added to `run.work` once per batch.
///
/// # Errors
///
/// A word of the pipeline names what the code or the file does not hold, a stage is one the fill side does not run yet,
/// or a source a `Use` stage's pattern read could not be read.
pub(crate) fn piped(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    entry: CodeEntry,
    run: &mut PipeRun<'_>,
) -> Result<(), FilledError> {
    let programs = frame.programs;
    let code = &programs.code;
    let words = pipeline_words(code, entry)?;
    carried(code, words, run.room)?;
    let registers = Count(usize::try_from(code.counts.registers.0).unwrap_or(usize::MAX));
    let first = *words.first().ok_or(WRONG_WORD)?;
    if stage_kind(first.op) != Some(StageKind::Scan) {
        return Err(WRONG_WORD);
    }
    let stage = stage_scan_of(first);
    let scanned = scan_run(batch_frame(frame), stage)?;
    let (site, all, tag, from) = (scanned.site, scanned.count, scanned.tag, scanned.rows.start);
    let mut done = 0_usize;
    while done < all.0 {
        let rows = BATCH_ROWS.0.min(all.0.saturating_sub(done));
        let level = level_at(run.room, Count(0), registers).ok_or(WRONG_WORD)?;
        let out = column_mut(&mut level.columns, stage.out).ok_or(WRONG_WORD)?;
        match site {
            Some(site) => {
                let index = index_at(frame.store, site);
                let held = rows_of(index, scanned.rows);
                scan_bucket(
                    held.get(done..done.saturating_add(rows))
                        .unwrap_or_default(),
                    tag,
                    out,
                );
            }
            None => scan(
                HandleRun {
                    first: Count(from.0.saturating_add(done)),
                    end: Count(from.0.saturating_add(done).saturating_add(rows)),
                    tag,
                },
                out,
            ),
        }
        level.rows = Count(rows);
        run.work.read = Count(run.work.read.0.saturating_add(rows));
        run.work.given = Count(run.work.given.0.saturating_add(rows));
        staged(frame, uses, words, Count(1), Count(0), run)?;
        done = done.saturating_add(rows);
    }
    Ok(())
}

/// Runs the stages from `from` on over the batch at `depth`, to the write or to a stage that gives several rows for one,
/// which runs the rest over each batch it gives.
fn staged(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    words: &[CodeWord],
    from: Count,
    depth: Count,
    run: &mut PipeRun<'_>,
) -> Result<(), FilledError> {
    let mut at = from.0;
    loop {
        let word = *words.get(at).ok_or(WRONG_WORD)?;
        let kind = stage_kind(word.op).ok_or(WRONG_WORD)?;
        let level = run.room.levels.get_mut(depth.0).ok_or(WRONG_WORD)?;
        let rows = level.rows;
        if rows.0 == 0 {
            return Ok(());
        }
        let selected = match kind {
            StageKind::Scan => return Err(WRONG_WORD),
            StageKind::Call | StageKind::Render => {
                return Err(FilledError::Unbuilt {
                    unbuilt: UnbuiltFill::Stage(word.op),
                });
            }
            StageKind::Write => {
                write_stage(batch_frame(frame), word, level, run.into)?;
                run.work.read = Count(run.work.read.0.saturating_add(rows.0));
                return Ok(());
            }
            StageKind::Read => {
                let batch = BatchFrame {
                    programs: frame.programs,
                    store: frame.store,
                    nodes: frame.nodes,
                    file: frame.file,
                    text: frame.text,
                };
                read_stage(batch, frame.words, word, level)?;
                false
            }
            StageKind::Keep => {
                keep_stage(
                    batch_frame(frame),
                    frame.words,
                    word,
                    level,
                    KeepScratch {
                        set: &mut run.room.set,
                        compiled: &mut run.room.compiled,
                        matcher: frame.matcher,
                    },
                )?;
                true
            }
            StageKind::Product => {
                return fanned(frame, uses, words, FanKind::Product, Count(at), depth, run);
            }
            StageKind::Latest => {
                return fanned(frame, uses, words, FanKind::Latest, Count(at), depth, run);
            }
            StageKind::Walk => {
                return fanned(frame, uses, words, FanKind::Walk, Count(at), depth, run);
            }
            StageKind::Probe => match mode(word.flag)? {
                UseMode::Rows => {
                    return fanned(frame, uses, words, FanKind::Probe, Count(at), depth, run);
                }
                UseMode::Present => {
                    let present = KeptWhen::Present;
                    probe_keep_stage(batch_frame(frame), word, present, level, &mut run.room.key)?;
                    true
                }
                UseMode::Absent => {
                    let absent = KeptWhen::Absent;
                    probe_keep_stage(batch_frame(frame), word, absent, level, &mut run.room.key)?;
                    true
                }
            },
            StageKind::Use => {
                use_asked(frame, uses, run.table, word, run.room, depth)?;
                let collects = stage_use_of(word).outputs.0 != 0;
                let when = match mode(word.flag)? {
                    UseMode::Rows if collects => {
                        return fanned(frame, uses, words, FanKind::Use, Count(at), depth, run);
                    }
                    // A use that collects nothing asks only whether its pattern has a row.
                    UseMode::Rows | UseMode::Present => KeptWhen::Present,
                    UseMode::Absent => KeptWhen::Absent,
                };
                let level = run.room.levels.get_mut(depth.0).ok_or(WRONG_WORD)?;
                keep_marked(&level.found, when, rows, &mut level.keep);
                true
            }
        };
        run.work.read = Count(run.work.read.0.saturating_add(rows.0));
        if selected {
            kept(run.room, Count(at), depth);
        }
        let given = run.room.levels.get(depth.0).map_or(0, |level| level.rows.0);
        run.work.given = Count(run.work.given.0.saturating_add(given));
        at = at.saturating_add(1);
    }
}

/// The register a stage that gives several rows for one writes each row's new cell to; none of a `Use`, which writes one
/// for each column it collects.
fn fan_out(kind: FanKind, word: CodeWord) -> Option<Register> {
    match kind {
        FanKind::Probe => Some(stage_probe_of(word).out),
        FanKind::Product => Some(stage_product_of(word).out),
        FanKind::Latest => Some(stage_latest_of(word).out),
        FanKind::Walk => Some(stage_walk_of(word).out),
        FanKind::Use => None,
    }
}

/// Runs a stage that gives several rows for one, and the rest of the pipeline over each batch it gives: the columns the
/// stage carries on are gathered into the batch of the next depth beside the stage's own.
fn fanned(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    words: &[CodeWord],
    kind: FanKind,
    at: Count,
    depth: Count,
    run: &mut PipeRun<'_>,
) -> Result<(), FilledError> {
    let programs = frame.programs;
    let code = &programs.code;
    let word = *words.get(at.0).ok_or(WRONG_WORD)?;
    let registers = Count(usize::try_from(code.counts.registers.0).unwrap_or(usize::MAX));
    let below = Count(depth.0.saturating_add(1));
    let _next = level_at(run.room, below, registers).ok_or(WRONG_WORD)?;
    let level = run.room.levels.get_mut(depth.0).ok_or(WRONG_WORD)?;
    level.fan.cursor = FanCursor::default();
    run.work.read = Count(run.work.read.0.saturating_add(level.rows.0));
    if kind == FanKind::Product {
        // The two sides of the product, for the notice of a join with no key.
        let stage = stage_product_of(word);
        let source = rows_in(frame.store, frame.nodes, source_of(stage.flag, stage.read));
        run.work.left = Count(run.work.left.0.saturating_add(level.rows.0));
        run.work.right = Count(source.end.0.saturating_sub(source.start.0));
    }
    loop {
        let PipelineRoom { levels, key, .. } = &mut *run.room;
        let level = levels.get_mut(depth.0).ok_or(WRONG_WORD)?;
        let spans = std::mem::take(&mut level.spans);
        let more = fan_stage(batch_frame(frame), kind, word, level, key, &spans);
        level.spans = spans;
        let more = more?;
        let (above, under) = run.room.levels.split_at_mut(below.0);
        let (Some(level), Some(next)) = (above.last_mut(), under.first_mut()) else {
            return Err(WRONG_WORD);
        };
        let given = level.fan.out.len();
        for register in live_at_of(&run.room.live, &run.room.starts, at) {
            if let Some(column) = column_mut(&mut next.columns, *register) {
                gather(column_of(&level.columns, *register), &level.fan.up, column);
            }
        }
        match fan_out(kind, word) {
            Some(register) => {
                let column = column_mut(&mut next.columns, register).ok_or(WRONG_WORD)?;
                std::mem::swap(column, &mut level.fan.out);
            }
            None => {
                let stage = stage_use_of(word);
                let width = Count(usize::from(stage.outputs.0));
                for (place, register) in registers_at(code, stage.registers).enumerate() {
                    let column = column_mut(&mut next.columns, register).ok_or(WRONG_WORD)?;
                    let view = ColumnView {
                        cells: &level.answers,
                        first: Count(place),
                        stride: width,
                    };
                    read(view, &level.fan.out, column);
                }
            }
        }
        next.rows = Count(given);
        run.work.given = Count(run.work.given.0.saturating_add(given));
        if given > 0 {
            staged(
                frame,
                uses,
                words,
                Count(at.0.saturating_add(1)),
                below,
                run,
            )?;
        }
        if more == Batches::Done {
            return Ok(());
        }
    }
}

/// The registers the stage at `at` carries on, read from the room's two lists.
fn live_at_of<'r>(live: &'r [Register], starts: &[Count], at: Count) -> &'r [Register] {
    let start = starts.get(at.0).map_or(0, |start| start.0);
    let end = starts
        .get(at.0.saturating_add(1))
        .map_or(start, |end| end.0);
    live.get(start..end).unwrap_or_default()
}
