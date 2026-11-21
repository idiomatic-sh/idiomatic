//! The fill side's bench workloads: whole fills through the seam's `made`, on packs the contract's encoding writes. Each
//! makes one table from a relation or from the file's nodes that are there already, so a run times the pipeline from its
//! scan to its write (and a grouping fill's grouping) and neither the language nor an index build.

use self::tables::{PRODUCT_LEFT, bench_table, bench_text, called, told_rows, tree};
use crate::domain::code::types::RelationNumber;
use crate::domain::filled::errors::FilledError;
use crate::domain::made_tables::fixtures::{Telling, World, framed, site_of, world_of};
use crate::domain::made_tables::sources::service::{made, matched, scanned, table_of};
use crate::domain::made_tables::types::{
    FillFrame, KeyFrom, MadeState, PatternAsk, PatternFound, PatternRows, ReadSource, TableNeed,
};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;

pub(crate) mod tables;

/// The fills the bench times, each one table made through the seam.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::{FILL_BENCHES, FillBench};
///
/// assert!(FILL_BENCHES.contains(&FillBench::RowsProbe));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillBench {
    /// A rows fill: a scan of a relation, two reads, the write.
    RowsRead,
    /// A rows fill that keeps the rows a comparison holds for: a scan, two reads, a keep, the write.
    RowsKeep,
    /// A rows fill that probes the relation by a column of the batch: a scan, a read, a probe, a read, the write.
    RowsProbe,
    /// A rows fill that drops repeats: a scan, a read of a column of eight rows a cell, the write.
    RowsDropped,
    /// A counted grouping, eight rows a key, through its pipeline.
    GroupCount,
    /// A sorted table, eight rows a key, through its pipeline.
    Sorted,
    /// A rows fill with a `Use` of a pattern in call form, eight rows a key: the loop's stand-in answers one row a key.
    UseRows,
    /// A rows fill that walks to the children of every node.
    WalkChildren,
    /// A rows fill that walks to the ancestors of every token.
    WalkAncestors,
    /// A rows fill that reads a node row's kind and keeps one kind.
    NodeKeep,
    /// A rows fill with a `Call` of `SpanBounds` for every token: one row for one, two numbers.
    CallSpanBounds,
    /// A rows fill with a `Call` of `Replace` for every token's text: one row for one, a word of the file.
    CallReplace,
    /// A rows fill with a `Call` of `Capture` for every token's text: one way a text, two slots.
    CallCapture,
    /// A rows fill with a `Call` of `SourceMatches` for every group of seven tokens: seven occurrences a group.
    CallSourceMatches,
    /// An ordered match of the relation with itself, eight rows a key on each side.
    OrderedMatch,
    /// The rows at the greatest end of each key's rows, eight rows a key.
    GroupEnds,
    /// A rows fill with a `Product`: thirty-two rows each paired with every row of the relation, a read and a keep for
    /// each pair, one pair in each row's kept.
    Product,
}

/// Every fill the bench times.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::FILL_BENCHES;
///
/// assert_eq!(FILL_BENCHES.len(), 17);
/// ```
pub const FILL_BENCHES: &[FillBench] = &[
    FillBench::RowsRead,
    FillBench::RowsKeep,
    FillBench::RowsProbe,
    FillBench::RowsDropped,
    FillBench::GroupCount,
    FillBench::Sorted,
    FillBench::UseRows,
    FillBench::WalkChildren,
    FillBench::WalkAncestors,
    FillBench::NodeKeep,
    FillBench::CallSpanBounds,
    FillBench::CallReplace,
    FillBench::CallCapture,
    FillBench::CallSourceMatches,
    FillBench::OrderedMatch,
    FillBench::GroupEnds,
    FillBench::Product,
];

/// The name a fill's measure carries.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::{FillBench, fill_name};
///
/// assert_eq!(fill_name(FillBench::RowsProbe), "rows_probe");
/// ```
#[must_use]
pub const fn fill_name(bench: FillBench) -> &'static str {
    match bench {
        FillBench::RowsRead => "rows_read",
        FillBench::RowsKeep => "rows_keep",
        FillBench::RowsProbe => "rows_probe",
        FillBench::RowsDropped => "rows_repeats_dropped",
        FillBench::GroupCount => "group_count",
        FillBench::Sorted => "sorted",
        FillBench::UseRows => "use_rows",
        FillBench::WalkChildren => "walk_children",
        FillBench::WalkAncestors => "walk_ancestors",
        FillBench::NodeKeep => "node_read_keep",
        FillBench::CallSpanBounds => "call_span_bounds",
        FillBench::CallReplace => "call_replace",
        FillBench::CallCapture => "call_capture",
        FillBench::CallSourceMatches => "call_source_matches",
        FillBench::OrderedMatch => "ordered_match",
        FillBench::GroupEnds => "group_ends",
        FillBench::Product => "product_pairs",
    }
}

/// What a fill's time over `rows` rows is divided by: the rows of its source, or for the product the pairs it makes
/// (thirty-two rows, or every row of fewer, each with every row).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::{FillBench, fill_units};
///
/// assert_eq!(fill_units(FillBench::RowsRead, 100), 100);
/// assert_eq!(fill_units(FillBench::Product, 100), 3_200);
/// ```
#[must_use]
pub const fn fill_units(bench: FillBench, rows: usize) -> usize {
    let rows = if rows == 0 { 1 } else { rows };
    match bench {
        FillBench::Product => {
            let left = if rows < PRODUCT_LEFT {
                rows
            } else {
                PRODUCT_LEFT
            };
            left.saturating_mul(rows)
        }
        FillBench::RowsRead
        | FillBench::RowsKeep
        | FillBench::RowsProbe
        | FillBench::RowsDropped
        | FillBench::GroupCount
        | FillBench::Sorted
        | FillBench::UseRows
        | FillBench::WalkChildren
        | FillBench::WalkAncestors
        | FillBench::NodeKeep
        | FillBench::CallSpanBounds
        | FillBench::CallReplace
        | FillBench::CallCapture
        | FillBench::CallSourceMatches
        | FillBench::OrderedMatch
        | FillBench::GroupEnds => rows,
    }
}

/// A fill's world, made once and filled many times: the pack, the relation's rows and the file's nodes.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::{FillBench, FillWorkload, fill_workload};
///
/// let work: Option<FillWorkload> = fill_workload(FillBench::RowsRead, 8);
/// assert!(work.is_some());
/// ```
#[derive(Debug)]
pub struct FillWorkload(World);

/// The loop's stand-in of the bench: every table may have a row, and a pattern has one row for a key, which holds the
/// key's first cell. It keeps nothing, so a run times the `Use` stage and not the stand-in.
#[derive(Debug)]
struct OneRow;

impl PatternRows for OneRow {
    fn table_need(
        &mut self,
        _: &mut FillFrame<'_>,
        _: MadeTableId,
    ) -> Result<TableNeed, FilledError> {
        Ok(TableNeed::Rows)
    }

    fn pattern_rows(
        &mut self,
        _: &mut FillFrame<'_>,
        asked: PatternAsk<'_>,
        rows: &mut Vec<CellWord>,
    ) -> Result<PatternFound, FilledError> {
        rows.extend(asked.key.first());
        Ok(PatternFound::Rows)
    }
}

/// The workload of a fill over about `rows` rows: its relation filled whole and the index its probe reads built, so a
/// run times the fill alone. None when the encoding refuses the workload's pack.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::{FillBench, fill_workload, run_fill};
///
/// let mut work = fill_workload(FillBench::RowsRead, 100).expect("the pack encodes");
/// assert_eq!(run_fill(&mut work), 100);
/// ```
#[must_use]
pub fn fill_workload(bench: FillBench, rows: usize) -> Option<FillWorkload> {
    let world = world_of(
        vec![bench_table(bench)],
        vec![called()],
        told_rows(bench, rows),
        tree(rows),
        Telling::Keyed,
    );
    let Ok(mut world) = world else {
        return None;
    };
    world.text = bench_text(rows);
    let reads_told = match bench {
        FillBench::RowsRead
        | FillBench::RowsKeep
        | FillBench::RowsProbe
        | FillBench::RowsDropped
        | FillBench::GroupCount
        | FillBench::Sorted
        | FillBench::UseRows
        | FillBench::OrderedMatch
        | FillBench::GroupEnds
        | FillBench::Product => true,
        FillBench::WalkChildren
        | FillBench::WalkAncestors
        | FillBench::NodeKeep
        | FillBench::CallSpanBounds
        | FillBench::CallReplace
        | FillBench::CallCapture
        | FillBench::CallSourceMatches => false,
    };
    if reads_told {
        let (mut frame, _) = framed(&mut world);
        // The sources are there before the first timed run: the relation whole, the probe's index built.
        let _filled = scanned(
            &mut frame,
            &mut OneRow,
            ReadSource::Filled(RelationNumber(0)),
        );
        if bench == FillBench::RowsProbe {
            let site = site_of(frame.programs.code, 1, &[0]);
            let _built = matched(
                &mut frame,
                &mut OneRow,
                site,
                &[CellWord(0)],
                KeyFrom::Constants,
            );
        }
    }
    Some(FillWorkload(world))
}

/// Makes the workload's table again and answers how many rows it holds: the table alone is unmade first, its sources
/// stay.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::{FillBench, fill_workload, run_fill};
///
/// let mut work = fill_workload(FillBench::GroupCount, 80).expect("the pack encodes");
/// assert_eq!(run_fill(&mut work), 10);
/// // Over 800 elements (100 groups of seven tokens) or 800 rows (100 keys of eight rows): a row for each token from
/// // the three calls of one row for one, seven occurrences a group, seven pairs a key (the last left row has no later
/// // right row), one greatest row a key, and of 32 times 800 pairs the 32 a row makes with itself.
/// let made = |bench| run_fill(&mut fill_workload(bench, 800).expect("the pack encodes"));
/// let fills = [
///     FillBench::CallSpanBounds,
///     FillBench::CallReplace,
///     FillBench::CallCapture,
///     FillBench::CallSourceMatches,
///     FillBench::OrderedMatch,
///     FillBench::GroupEnds,
///     FillBench::Product,
/// ];
/// assert_eq!(fills.map(made), [700, 700, 700, 700, 700, 100, 32]);
/// ```
pub fn run_fill(work: &mut FillWorkload) -> usize {
    if let Some(slot) = work.0.store.tables.first_mut() {
        slot.state = MadeState::Unmade;
    }
    let (mut frame, _) = framed(&mut work.0);
    let _now = made(&mut frame, &mut OneRow, MadeTableId(0));
    table_of(frame.store, MadeTableId(0)).map_or(0, |slot| slot.rows.count.0)
}
