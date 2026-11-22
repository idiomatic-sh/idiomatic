//! The services of the `making` part: a table made for the file, on its first read and once. What its fill reads is made,
//! filled or indexed first, in the order its stages read it; then its pipelines run. A pipeline whose scan has no row
//! runs nothing and has nothing made for it.

use crate::domain::code::filling::service::stage_scan_of;
use crate::domain::code::records::service::visit_columns_at;
use crate::domain::code::types::{CodeEntry, IndexSite, PoolAt, RelationNumber};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::service::whole;
use crate::domain::filled::types::{FillRuns, MissingCode, RelationAsk, UnbuiltFill};
use crate::domain::indexes::service::state_of;
use crate::domain::indexes::types::IndexBuilt;
use crate::domain::made_tables::built::service::runs;
use crate::domain::made_tables::built::types::FillPart;
use crate::domain::made_tables::carrying::service::stage_reach;
use crate::domain::made_tables::columns::types::CellsLie;
use crate::domain::made_tables::fanning::service::scan_read;
use crate::domain::made_tables::fills::service::{
    constant_rows, fill_at, fill_code_at, grouped_rows, input_at, input_width, laid,
};
use crate::domain::made_tables::fills::types::{FillInputs, FillKind, TableFill};
use crate::domain::made_tables::indexing::service::index_built;
use crate::domain::made_tables::matching::service::match_rows;
use crate::domain::made_tables::matching::types::MatchSide;
use crate::domain::made_tables::pipelines::service::piped;
use crate::domain::made_tables::reading::service::{
    pipeline_words, relation_at, site_at, site_source,
};
use crate::domain::made_tables::service::clear_rows;
use crate::domain::made_tables::staging::types::BatchFrame;
use crate::domain::made_tables::types::PipeRun;
use crate::domain::made_tables::types::{
    FillFrame, FillKept, FillWork, MadeNow, MadeState, PatternRows, Reached, ReadSource,
    SourceWork, TableNeed, TableRows, TableSlot,
};
use crate::domain::made_tables::visiting::service::{VisitTold, visit_groups};
use crate::domain::made_tables::writing::service::{fill_begun, fill_ended};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::relations::types::Count;

/// A table's slot, there for any table the code numbers.
fn slot_mut<'s>(frame: &'s mut FillFrame<'_>, table: MadeTableId) -> Option<&'s mut TableSlot> {
    let place = usize::try_from(table.0).unwrap_or(usize::MAX);
    if place >= frame.programs.code.tables.len() {
        return None;
    }
    if frame.store.tables.len() <= place {
        frame
            .store
            .tables
            .resize_with(place.saturating_add(1), TableSlot::default);
    }
    frame.store.tables.get_mut(place)
}

/// Fills a relation whole for the file if it is not whole. Answers the rows the language wrote for it now.
fn filled_whole(frame: &mut FillFrame<'_>, number: RelationNumber) -> Result<Count, FilledError> {
    let relation = relation_at(frame.programs.code, number)?;
    let ask = RelationAsk {
        number,
        relation,
        stamp: frame.store.stamp,
        facts: &mut *frame.facts,
        words: &mut *frame.words,
    };
    whole(&mut frame.store.relations, ask).map(|rows| rows.filled)
}

/// Makes a table the fill side reaches itself, unless the file has it: the loop is asked whether it may have a row; a
/// table that has none is kept as made with no row, and no fill runs.
fn reached_table(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
) -> Result<(), FilledError> {
    let stamp = frame.store.stamp;
    let slot = slot_mut(frame, table).ok_or(FilledError::Code {
        named: MissingCode::Table,
    })?;
    if slot.stamp == stamp && slot.state == MadeState::Made {
        return Ok(());
    }
    match uses.table_need(frame, table)? {
        TableNeed::Rows => made_now(frame, uses, table).map(|_| ()),
        TableNeed::Empty => {
            let width = fill_at(frame.programs.code, table)?.width;
            let slot = slot_mut(frame, table).ok_or(FilledError::Code {
                named: MissingCode::Table,
            })?;
            // The decision may have made it: a need that asks the table itself.
            if slot.stamp == stamp && slot.state == MadeState::Made {
                return Ok(());
            }
            slot.stamp = stamp;
            slot.state = MadeState::Made;
            slot.work = FillWork::default();
            slot.reached.clear();
            clear_rows(&mut slot.rows, width);
            Ok(())
        }
    }
}

/// Has a thing the fill side reaches there for the file: a table made, a relation whole, an index built over its source.
/// Answers what that made the engine do now: nothing when it was there.
///
/// # Errors
///
/// The source could not be read, or the code does not hold it.
pub(crate) fn ensured(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    reached: Reached,
) -> Result<SourceWork, FilledError> {
    let mut work = SourceWork::default();
    match reached {
        Reached::Nodes => {}
        Reached::Relation(number) => {
            // SPIKE (track X, a count under RYO_X_FILLS): a reader reached the relation.
            if let Ok(relation) = relation_at(frame.programs.code, number) {
                ryo_language::domain::fillers::service::fill_read(relation);
            }
            work.filled = filled_whole(frame, number)?;
        }
        Reached::Table(table) => reached_table(frame, uses, table)?,
        Reached::Index(site) => {
            // SPIKE (track X, a count under RYO_X_FILLS): a reader reached the index's relation.
            if let Ok(ReadSource::Filled(number)) = site_at(frame.programs.code, site)
                .and_then(|index| site_source(frame.programs.code, index))
                && let Ok(relation) = relation_at(frame.programs.code, number)
            {
                ryo_language::domain::fillers::service::fill_read(relation);
            }
            if state_of(&frame.store.indexes, site, frame.store.stamp) == IndexBuilt::Built {
                return Ok(work);
            }
            work.filled = indexed_source(frame, uses, site)?;
            work.indexed = index_built(frame, site)?;
        }
    }
    Ok(work)
}

/// Has the source of an index there for the file, the index itself not built: the table it is over made, the relation
/// whole. Answers how many rows of a relation that filled now.
fn indexed_source(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    site: IndexSite,
) -> Result<Count, FilledError> {
    let programs = frame.programs;
    match site_source(programs.code, site_at(programs.code, site)?)? {
        ReadSource::Nodes(_) => Ok(Count(0)),
        ReadSource::Filled(number) => filled_whole(frame, number),
        ReadSource::Made(table) => reached_table(frame, uses, table).map(|()| Count(0)),
        ReadSource::Written(_) => Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Written,
        }),
    }
}

/// Runs the pipelines of one input into `into`. First each pipeline's sources are there (`sources_there`), then the
/// pipelines that have a row run, in turn: every table the fill reaches by a stage is made BEFORE its first row is
/// written, so its rows lie in one stretch of the arena unmoved; the same tables are made as before.
fn input_piped(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
    entries: FillInputs<'_>,
    into: &mut TableRows,
    kept: FillKept<'_>,
) -> Result<(), FilledError> {
    let FillKept { work, reached } = kept;
    let mut running = frame.store.running.pop().unwrap_or_default();
    running.clear();
    let mut there = Ok(());
    for entry in entries.entries {
        let entry = CodeEntry(entry.0);
        match sources_there(frame, uses, entry, reached) {
            Ok(Count(0)) => {}
            Ok(_) => running.push(entry),
            Err(fault) => {
                there = Err(fault);
                break;
            }
        }
    }
    let mut done = there;
    for entry in &running {
        if done.is_err() {
            break;
        }
        let mut room = frame.store.rooms.pop().unwrap_or_default();
        done = piped(
            frame,
            uses,
            *entry,
            &mut PipeRun {
                table,
                into,
                work,
                room: &mut room,
            },
        );
        frame.store.rooms.push(room);
    }
    frame.store.running.push(running);
    done
}

/// PROTOTYPE (ryo-fmx3.1, join order): `RYO_ORDER_OFF` set to anything turns the planner rule off, read once.
fn order_off() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_ORDER_OFF").is_some())
}

/// PROTOTYPE (ryo-fmx3.1, join order): how many times less work than an unmade scan a source must need at least to be
/// made before it.
const AHEAD_OF_SCAN: u64 = 8;

/// PROTOTYPE (ryo-fmx3.1, join order): what the file already knows of a source a stage reads, nothing made for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Known {
    /// It is there and holds no row.
    Empty,
    /// It is there (or is being made): it cannot show the pipeline empty for less than it cost already.
    There,
    /// It is not there yet: making it is a way to learn whether the pipeline has a row.
    Unknown,
}

/// A source of a pipeline every one of whose rows must find a row in it: the scan's, a product's, a probe that keeps or
/// joins a row only when the probed source has one. An absence probe, a `Latest`, a walk and a call-form use are not.
fn required(code: &crate::domain::code::types::ProgramCode, word: crate::domain::code::types::CodeWord) -> Result<Option<Reached>, FilledError> {
    use crate::domain::made_tables::pipelines::types::StageKind;
    use crate::domain::made_tables::staging::service::mode;
    use crate::domain::steps::stages::types::UseMode;
    let kind = crate::domain::made_tables::carrying::service::stage_kind(word.op);
    let wanted = match kind {
        Some(StageKind::Scan | StageKind::Product) => true,
        Some(StageKind::Probe) => matches!(mode(word.flag)?, UseMode::Rows | UseMode::Present),
        _ => false,
    };
    if wanted {
        stage_reach(code, word)
    } else {
        Ok(None)
    }
}

/// A made table's slot for the file, when it has one.
fn slot_now<'s>(frame: &'s FillFrame<'_>, table: MadeTableId) -> Option<&'s TableSlot> {
    let slot = frame.store.tables.get(usize::try_from(table.0).unwrap_or(usize::MAX))?;
    (slot.stamp == frame.store.stamp).then_some(slot)
}

/// What the file knows of a table: made with or without a row, being made, or not made.
fn table_known(frame: &FillFrame<'_>, table: MadeTableId) -> Known {
    match slot_now(frame, table) {
        Some(slot) if slot.state == MadeState::Made && slot.rows.count.0 == 0 => Known::Empty,
        Some(slot) if slot.state == MadeState::Made || slot.state == MadeState::Making => Known::There,
        Some(_) | None => Known::Unknown,
    }
}

/// What the file knows of a relation: whole with or without a row, or not whole.
fn relation_known(frame: &FillFrame<'_>, number: RelationNumber) -> Known {
    use crate::domain::filled::service::{held_state, whole_rows};
    use crate::domain::filled::types::FilledState;
    if held_state(&frame.store.relations, number, frame.store.stamp) != FilledState::Whole {
        return Known::Unknown;
    }
    let run = whole_rows(&frame.store.relations, number, frame.store.stamp);
    if run.end.0 > run.start.0 {
        Known::There
    } else {
        Known::Empty
    }
}

/// What the file knows of a source a stage reaches. A scan's index must be built before its bucket is read; a probe's
/// source must be there, its index is built by the first batch that reaches it.
fn known(frame: &FillFrame<'_>, reach: Reached, scan: bool) -> Known {
    match reach {
        Reached::Nodes => Known::There,
        Reached::Table(table) => table_known(frame, table),
        Reached::Relation(number) => relation_known(frame, number),
        Reached::Index(site) => {
            if scan {
                return if state_of(&frame.store.indexes, site, frame.store.stamp) == IndexBuilt::Built {
                    Known::There
                } else {
                    Known::Unknown
                };
            }
            match site_at(frame.programs.code, site).and_then(|index| site_source(frame.programs.code, index)) {
                Ok(ReadSource::Made(table)) => table_known(frame, table),
                Ok(ReadSource::Filled(number)) => relation_known(frame, number),
                Ok(ReadSource::Nodes(_) | ReadSource::Written(_)) | Err(_) => Known::There,
            }
        }
    }
}

/// The rows a source holds for the file when it is there: what a scan of it reads, the least it costs to fill.
fn rows_there(frame: &FillFrame<'_>, reach: Reached) -> u64 {
    match reach {
        Reached::Nodes => ryo_language::domain::node_table::service::element_count(frame.nodes).0 as u64,
        Reached::Table(table) => slot_now(frame, table).map_or(0, |slot| slot.rows.count.0 as u64),
        Reached::Relation(number) => {
            let run = crate::domain::filled::service::whole_rows(&frame.store.relations, number, frame.store.stamp);
            run.end.0.saturating_sub(run.start.0) as u64
        }
        Reached::Index(_) => 0,
    }
}

/// The work the file is known to need before a source is there, at least: nothing for what is there; the file's elements
/// for a relation to fill; the source of an index to build; and for a table not made, what its fill needs at least
/// (`table_least`). Every figure is a count the file holds now, or the least the fill can cost, never an estimate.
fn least(frame: &FillFrame<'_>, reach: Reached, scan: bool, depth: u32, high: bool) -> u64 {
    let elements = || ryo_language::domain::node_table::service::element_count(frame.nodes).0 as u64;
    match reach {
        Reached::Nodes => 0,
        Reached::Table(table) => match table_known(frame, table) {
            Known::Empty | Known::There => 0,
            Known::Unknown => table_least(frame, table, depth, high),
        },
        Reached::Relation(number) => match relation_known(frame, number) {
            Known::Empty | Known::There => 0,
            Known::Unknown => elements(),
        },
        Reached::Index(site) => {
            let built = state_of(&frame.store.indexes, site, frame.store.stamp) == IndexBuilt::Built;
            if built {
                return 0;
            }
            let source = site_at(frame.programs.code, site).and_then(|index| site_source(frame.programs.code, index));
            let (there, rows) = match source {
                Ok(ReadSource::Made(table)) => match table_known(frame, table) {
                    Known::Unknown => (table_least(frame, table, depth, high), 0),
                    Known::Empty | Known::There => (0, rows_there(frame, Reached::Table(table))),
                },
                Ok(ReadSource::Filled(number)) => match relation_known(frame, number) {
                    Known::Unknown => (elements(), 0),
                    Known::Empty | Known::There => (0, rows_there(frame, Reached::Relation(number))),
                },
                // An index over the node table is built over every element.
                Ok(ReadSource::Nodes(_)) => (0, elements()),
                Ok(ReadSource::Written(_)) | Err(_) => (0, 0),
            };
            // A scan's bucket needs the index built over every row of its source.
            if scan { there.saturating_add(rows) } else { there }
        }
    }
}

/// The least work making a table needs: for each pipeline of its fill's first input, the least of what any source it
/// cannot write a row without costs to be there (it may show the pipeline empty), or, when every such source is there,
/// the rows its scan reads. A pipeline with a source there and empty costs nothing. Past `depth` nothing is counted.
pub(crate) fn table_least(frame: &FillFrame<'_>, table: MadeTableId, depth: u32, high: bool) -> u64 {
    let stand_in = if high { ryo_language::domain::node_table::service::element_count(frame.nodes).0 as u64 } else { 0 };
    // Past the depth nothing more is known: the least is none (a stand-in above the least would put a source behind
    // one it is not known to cost more than: measured, r2a, T898's scan).
    if depth == 0 {
        return stand_in;
    }
    let code = frame.programs.code;
    let Ok(fill) = fill_at(code, table) else {
        return stand_in;
    };
    if fill.kind == FillKind::Constant {
        return 0;
    }
    let Ok(input) = input_at(code, fill.inputs) else {
        return stand_in;
    };
    input.entries.iter().fold(0_u64, |sum, entry| {
        sum.saturating_add(pipeline_least(frame, CodeEntry(entry.0), depth.saturating_sub(1), high).unwrap_or(stand_in))
    })
}

/// The rows a pipeline's scan reads, when its source is there: a bucket's by its built index, else the source's rows.
fn scanned_there(frame: &FillFrame<'_>, word: crate::domain::code::types::CodeWord, reach: Reached) -> u64 {
    match reach {
        Reached::Index(_) => {
            let batch = BatchFrame {
                programs: frame.programs,
                store: frame.store,
                nodes: frame.nodes,
                file: frame.file,
                path: frame.path,
                text: frame.text,
                source: frame.source,
            };
            scan_read(batch, stage_scan_of(word)).map_or(0, |read| read.count.0 as u64)
        }
        Reached::Nodes | Reached::Table(_) | Reached::Relation(_) => rows_there(frame, reach),
    }
}

/// The least work one pipeline needs before it writes a row or is shown to have none: `table_least`'s. A source not
/// there may show it empty for what that source costs; when every source is there, it reads its scan's rows.
fn pipeline_least(frame: &FillFrame<'_>, entry: CodeEntry, depth: u32, high: bool) -> Result<u64, FilledError> {
    let code = frame.programs.code;
    let words = pipeline_words(code, entry)?;
    let mut witness: Option<u64> = None;
    let mut scanned = 0_u64;
    for (at, word) in words.iter().enumerate() {
        let Some(reach) = required(code, *word)? else {
            continue;
        };
        match known(frame, reach, at == 0) {
            Known::Empty => return Ok(0),
            Known::There => {
                if at == 0 {
                    scanned = scanned_there(frame, *word, reach);
                    if scanned == 0 {
                        return Ok(0);
                    }
                }
            }
            Known::Unknown => {
                let cost = least(frame, reach, at == 0, depth, high);
                witness = Some(witness.map_or(cost, |held| held.min(cost)));
            }
        }
    }
    Ok(witness.unwrap_or(scanned))
}

/// PROTOTYPE (ryo-fmx3.1, join order), the planner rule "the cheapest way to learn a join is empty goes first": what a
/// pipeline cannot write a row without (its scan's source, a product's, a probe's that keeps or joins on a find) is
/// made in the order of the least work each needs, as the file knows it now (`least`), and the first that holds no row
/// ends the pipeline: nothing else of it is made, and it runs nothing. Sound for any query: a made table and a relation
/// are the file's rows whoever asks and whenever, so the order they are made in changes no row; and a pipeline whose
/// required source has no row can write none. What the pipeline reads otherwise is there, in stage order, before it runs.
fn sources_ordered(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    entry: CodeEntry,
    reached: &mut Vec<Reached>,
) -> Result<Count, FilledError> {
    let programs = frame.programs;
    let words = pipeline_words(programs.code, entry)?;
    let scan_count = |frame: &mut FillFrame<'_>, word| -> Result<Count, FilledError> {
        let batch = BatchFrame {
            programs,
            store: frame.store,
            nodes: frame.nodes,
            file: frame.file,
            path: frame.path,
            text: frame.text,
            source: frame.source,
        };
        Ok(scan_read(batch, stage_scan_of(word))?.count)
    };
    // What is there: an empty source ends the pipeline now; the rest that is not there is ordered by its least work.
    let mut waiting: Vec<(u64, usize, Reached)> = Vec::new();
    let mut scanned: Option<Count> = None;
    for (at, word) in words.iter().enumerate() {
        let Some(reach) = required(programs.code, *word)? else {
            continue;
        };
        match known(frame, reach, at == 0) {
            Known::Empty => return Ok(Count(0)),
            Known::There => {}
            Known::Unknown => waiting.push((0, at, reach)),
        }
    }
    // Only a choice between two or more sources not there is worth its figures. A source goes before an unmade scan only
    // when the least work it needs is under an eighth of the scan's: a least figure leaves out what each row costs, so a
    // source just cheaper than the scan may cost more made than the scan would have (measured, r1a/r1b: the binding
    // rules' tables). Behind a scan that is there with a row, every such source is made anyway: there the least goes first.
    if waiting.len() >= 2 {
        for each in &mut waiting {
            // The scan's figure counts nothing past what is known, a source's that would go ahead counts the file's elements
            // there: a source goes ahead only when even so it is far under the scan.
            each.0 = least(frame, each.2, each.1 == 0, 3, each.1 != 0);
        }
        if let Some(scan) = waiting.iter().position(|each| each.1 == 0) {
            let bar = waiting.get(scan).map_or(0, |each| each.0);
            for each in &mut waiting {
                if each.1 != 0 && each.0.saturating_mul(AHEAD_OF_SCAN) >= bar {
                    // Not far enough under the scan: it waits behind it, in its own order of least work.
                    each.0 = bar.saturating_add(1).saturating_add(each.0);
                }
            }
        }
    }
    let scan_there = waiting.first().is_none_or(|first| first.1 != 0);
    if scan_there {
        let first = *words.first().ok_or(FilledError::Code { named: MissingCode::Word })?;
        if let Some(read) = stage_reach(programs.code, first)? {
            let _work = ensured(frame, uses, read)?;
            reached.push(read);
        }
        let count = scan_count(frame, first)?;
        if count.0 == 0 {
            return Ok(count);
        }
        scanned = Some(count);
    }
    waiting.sort_by_key(|(cost, at, _)| (*cost, *at));
    for (_, at, reach) in waiting {
        match reach {
            Reached::Index(site) if at != 0 => {
                let _filled = indexed_source(frame, uses, site)?;
            }
            Reached::Index(_) | Reached::Nodes | Reached::Relation(_) | Reached::Table(_) => {
                let _work = ensured(frame, uses, reach)?;
            }
        }
        reached.push(reach);
        if at == 0 {
            let word = *words.first().ok_or(FilledError::Code { named: MissingCode::Word })?;
            let count = scan_count(frame, word)?;
            if count.0 == 0 {
                return Ok(count);
            }
            scanned = Some(count);
        } else if known(frame, reach, false) == Known::Empty {
            return Ok(Count(0));
        }
    }
    // The pipeline has a row to run: what else it reads is there, in stage order.
    for (at, word) in words.iter().enumerate().skip(1) {
        if let Some(read) = stage_reach(programs.code, *word)? {
            match read {
                Reached::Index(site) => {
                    let _filled = indexed_source(frame, uses, site)?;
                }
                Reached::Nodes | Reached::Relation(_) | Reached::Table(_) => {
                    let _work = ensured(frame, uses, read)?;
                }
            }
            let _ = at;
            reached.push(read);
        }
    }
    Ok(scanned.unwrap_or(Count(0)))
}

/// Has what one pipeline reads there: what its scan reads first; when the scan has a row, what its other stages read,
/// in stage order. Answers how many rows the scan has.
fn sources_there(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    entry: CodeEntry,
    reached: &mut Vec<Reached>,
) -> Result<Count, FilledError> {
    if !order_off() {
        return sources_ordered(frame, uses, entry, reached);
    }
    sources_before(frame, uses, entry, reached)
}

/// The order before the planner rule: the scan's source first, the others in stage order once the scan has a row.
fn sources_before(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    entry: CodeEntry,
    reached: &mut Vec<Reached>,
) -> Result<Count, FilledError> {
    let programs = frame.programs;
    let words = pipeline_words(programs.code, entry)?;
    let mut scanned = Count(0);
    for (at, word) in words.iter().enumerate() {
        if let Some(read) = stage_reach(programs.code, *word)? {
            match read {
                // The index a stage after the scan asks is built by the first batch that reaches the stage with a
                // row (`indexing::service::index_ready`): many never do. Its source is there now, so that no table
                // is made while this fill writes.
                Reached::Index(site) if at != 0 => {
                    let _filled = indexed_source(frame, uses, site)?;
                }
                Reached::Index(_) | Reached::Nodes | Reached::Relation(_) | Reached::Table(_) => {
                    let _work = ensured(frame, uses, read)?;
                }
            }
            reached.push(read);
        }
        if at == 0 {
            let batch = BatchFrame {
                programs,
                store: frame.store,
                nodes: frame.nodes,
                file: frame.file,
                path: frame.path,
                text: frame.text,
                source: frame.source,
            };
            scanned = scan_read(batch, stage_scan_of(*word))?.count;
            if scanned.0 == 0 {
                break;
            }
        }
    }
    Ok(scanned)
}

/// Gathers one input of a grouping fill into `into`: emptied as the rows its pipelines write, its columns laid, its
/// pipelines run.
fn gathered_in(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
    input: FillInputs<'_>,
    into: &mut TableRows,
    kept: FillKept<'_>,
) -> Result<(), FilledError> {
    clear_rows(into, input_width(frame.programs.code, input)?);
    laid(frame, input, into)?;
    input_piped(frame, uses, table, input, into, kept)
}

/// Runs a table's fill into `rows`.
fn filled_now(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
    fill: TableFill,
    rows: &mut TableRows,
    kept: FillKept<'_>,
) -> Result<(), FilledError> {
    let programs = frame.programs;
    match fill.kind {
        FillKind::Constant => constant_rows(programs.code, fill, rows),
        FillKind::Rows => {
            let input = input_at(programs.code, fill.inputs)?;
            laid(frame, input, rows)?;
            input_piped(frame, uses, table, input, rows, kept)
        }
        FillKind::Visits => {
            let FillKept { work, reached } = kept;
            let wrong = FilledError::Code {
                named: MissingCode::Column,
            };
            // The record's parameter is the place of the told columns; its inputs are the roots, then the edges.
            let columns = visit_columns_at(&programs.code.pool, PoolAt(fill.parameter.0))
                .ok_or(wrong.clone())?;
            let roots = input_at(programs.code, fill.inputs)?;
            let edges = input_at(programs.code, roots.after)?;
            let told = VisitTold {
                columns,
                root_keys: roots.keys,
                edge_keys: edges.keys,
                order: edges.order.ok_or(wrong)?,
            };
            let mut gathered = [
                frame.store.inputs.pop().unwrap_or_default(),
                frame.store.inputs.pop().unwrap_or_default(),
            ];
            let mut piped = Ok(());
            for (input, into) in [roots, edges].into_iter().zip(gathered.iter_mut()) {
                if piped.is_ok() {
                    let kept = FillKept {
                        work: &mut *work,
                        reached: &mut *reached,
                    };
                    piped = gathered_in(frame, uses, table, input, into, kept);
                }
            }
            let [gathered_roots, gathered_edges] = gathered;
            let done = piped.and_then(|()| {
                visit_groups(&gathered_roots, &gathered_edges, told, rows)
                    .map_err(|fault| FilledError::Meaning { fault })
            });
            work.read = Count(
                work.read
                    .0
                    .saturating_add(gathered_roots.count.0)
                    .saturating_add(gathered_edges.count.0),
            );
            work.given = Count(work.given.0.saturating_add(rows.count.0));
            frame.store.inputs.push(gathered_roots);
            frame.store.inputs.push(gathered_edges);
            done
        }
        FillKind::Match => {
            let FillKept { work, reached } = kept;
            let wrong = FilledError::Code {
                named: MissingCode::Column,
            };
            let left = input_at(programs.code, fill.inputs)?;
            let right = input_at(programs.code, left.after)?;
            let left_side = MatchSide {
                keys: left.keys,
                order: left.order.ok_or(wrong.clone())?,
            };
            let right_side = MatchSide {
                keys: right.keys,
                order: right.order.ok_or(wrong)?,
            };
            let mut gathered = [
                frame.store.inputs.pop().unwrap_or_default(),
                frame.store.inputs.pop().unwrap_or_default(),
            ];
            let mut piped = Ok(());
            for (input, into) in [left, right].into_iter().zip(gathered.iter_mut()) {
                if piped.is_ok() {
                    let kept = FillKept {
                        work: &mut *work,
                        reached: &mut *reached,
                    };
                    piped = gathered_in(frame, uses, table, input, into, kept);
                }
            }
            let mut room = std::mem::take(&mut frame.store.room);
            let [gathered_left, gathered_right] = gathered;
            let done = piped.and_then(|()| {
                match_rows(
                    &gathered_left,
                    &gathered_right,
                    left_side,
                    right_side,
                    &mut room.matching,
                    rows,
                )
            });
            work.read = Count(
                work.read
                    .0
                    .saturating_add(gathered_left.count.0)
                    .saturating_add(gathered_right.count.0),
            );
            work.given = Count(work.given.0.saturating_add(rows.count.0));
            work.left = gathered_left.count;
            work.right = gathered_right.count;
            frame.store.room = room;
            frame.store.inputs.push(gathered_left);
            frame.store.inputs.push(gathered_right);
            done
        }
        FillKind::Count
        | FillKind::Sorted
        | FillKind::Ends
        | FillKind::Places
        | FillKind::Follow => {
            let FillKept { work, reached } = kept;
            let input = input_at(programs.code, fill.inputs)?;
            let mut gathered = frame.store.inputs.pop().unwrap_or_default();
            let kept = FillKept {
                work: &mut *work,
                reached,
            };
            let piped = gathered_in(frame, uses, table, input, &mut gathered, kept);
            let mut room = std::mem::take(&mut frame.store.room);
            let done = piped.and_then(|()| {
                grouped_rows(programs.code, fill, &gathered, input, &mut room, rows, work)
            });
            frame.store.room = room;
            frame.store.inputs.push(gathered);
            done
        }
    }
}

/// Makes `table` for the file if no read of the file made it: its fill runs once. The table's slot then holds its rows,
/// the work of its own fill and what the fill reached directly.
///
/// # Errors
///
/// A source its fill reads could not be read; the fill is one the fill side does not run yet; or the table is read while
/// its own fill runs.
pub(crate) fn made_now(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
) -> Result<MadeNow, FilledError> {
    let stamp = frame.store.stamp;
    // A table whose fill the fill side does not run is refused before anything of it is read.
    let written = fill_code_at(frame.programs.code, table)?;
    if runs(frame.programs.code, FillPart::Fill(written)) == FillRuns::NotYet {
        return Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Fill(written),
        });
    }
    let fill = fill_at(frame.programs.code, table)?;
    let slot = slot_mut(frame, table).ok_or(FilledError::Code {
        named: MissingCode::Table,
    })?;
    if slot.stamp == stamp {
        match slot.state {
            MadeState::Made => return Ok(MadeNow::Held),
            MadeState::Making => {
                return Err(FilledError::Unbuilt {
                    unbuilt: UnbuiltFill::Written,
                });
            }
            MadeState::Unmade => {}
        }
    }
    slot.stamp = stamp;
    slot.state = MadeState::Making;
    let mut rows = std::mem::take(&mut slot.rows);
    let mut reached = std::mem::take(&mut slot.reached);
    clear_rows(&mut rows, fill.width);
    let straight = if fill.kind == FillKind::Rows {
        CellsLie::Arena(Count(frame.store.table_cells.len()))
    } else {
        CellsLie::Own
    };
    fill_begun(frame.store, &mut rows, straight);
    reached.clear();
    let mut work = FillWork::default();
    uses.table_started(table);
    let done = filled_now(
        frame,
        uses,
        table,
        fill,
        &mut rows,
        FillKept {
            work: &mut work,
            reached: &mut reached,
        },
    );
    uses.table_ended(table);
    fill_ended(frame.store, &mut rows);
    work.made = rows.count;
    if let Some(slot) = slot_mut(frame, table) {
        slot.rows = rows;
        slot.reached = reached;
        slot.work = work;
        slot.state = match done {
            Ok(()) => MadeState::Made,
            Err(_) => MadeState::Unmade,
        };
    }
    done.map(|()| MadeNow::Made)
}
