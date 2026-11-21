//! The fill side's bench workloads: whole fills through the seam's `made`, on packs the contract's encoding writes. Each
//! makes one table from a relation or from the file's nodes that are there already, so a run times the pipeline from its
//! scan to its write (and a grouping fill's grouping) and neither the language nor an index build.

use self::kinds::FillBench;
use self::tables::{PRODUCT_LEFT, bench_table, bench_text, called, told_rows, tree};
use crate::domain::code::types::RelationNumber;
use crate::domain::filled::errors::FilledError;
use crate::domain::made_tables::fixtures::packs::FILE;
use crate::domain::made_tables::fixtures::sites::{node_site, site_of};
use crate::domain::made_tables::fixtures::{Telling, World, framed, world_of};
use crate::domain::made_tables::sources::service::{made, matched, next_file, scanned, table_of};
use crate::domain::made_tables::types::{
    FillFrame, KeyFrom, MadeState, PatternAsk, PatternFound, PatternRows, ReadSource, TableNeed,
};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::service::{base_words, clear_words, file_words};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::fixed_names::service::checked;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::terms::types::NameText;

pub mod kinds;
pub(crate) mod tables;

/// Every fill the bench times.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::FILL_BENCHES;
///
/// assert_eq!(FILL_BENCHES.len(), 24);
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
    FillBench::RowsFile,
    FillBench::RowsFileDropped,
    FillBench::KeepGlobFewWords,
    FillBench::KeepGlobOwnWords,
    FillBench::IndexNodesKind,
    FillBench::IndexNodesParent,
    FillBench::IndexNodesId,
];

/// The name a fill's measure carries.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::fill_name;
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
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
        FillBench::RowsFile => "rows_with_the_file",
        FillBench::RowsFileDropped => "rows_with_the_file_repeats_dropped",
        FillBench::KeepGlobFewWords => "keep_glob_few_words",
        FillBench::KeepGlobOwnWords => "keep_glob_own_words",
        FillBench::IndexNodesKind => "index_nodes_by_kind",
        FillBench::IndexNodesParent => "index_nodes_by_parent",
        FillBench::IndexNodesId => "index_nodes_by_id",
    }
}

/// What a fill's time over `rows` rows is divided by: the rows of its source, or for the product the pairs it makes
/// (thirty-two rows, or every row of fewer, each with every row).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::fill_units;
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
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
        | FillBench::GroupEnds
        | FillBench::RowsFile
        | FillBench::RowsFileDropped
        | FillBench::KeepGlobFewWords
        | FillBench::KeepGlobOwnWords
        | FillBench::IndexNodesKind
        | FillBench::IndexNodesParent
        | FillBench::IndexNodesId => rows,
    }
}

/// A fill's world, made once and filled many times: the pack, the relation's rows and the file's nodes.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
/// use ryo_engine::domain::made_tables::fixtures::benches::{FillWorkload, fill_workload};
///
/// let work: Option<FillWorkload> = fill_workload(FillBench::RowsRead, 8);
/// assert!(work.is_some());
/// ```
#[derive(Debug)]
pub struct FillWorkload(World, FillBench);

/// The word of a token's kind in the bench's vocabulary.
const TOKEN_KIND: CellWord = CellWord(13);

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

    fn table_started(&mut self, _: MadeTableId) {}

    fn table_ended(&mut self, _: MadeTableId) {}
}

/// The workload of a fill over about `rows` rows: its relation filled whole and the index its probe reads built, so a
/// run times the fill alone. None when the encoding refuses the workload's pack.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
/// use ryo_engine::domain::made_tables::fixtures::benches::{fill_workload, run_fill};
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
        | FillBench::RowsFile
        | FillBench::RowsFileDropped
        | FillBench::Product => true,
        FillBench::WalkChildren
        | FillBench::WalkAncestors
        | FillBench::NodeKeep
        | FillBench::CallSpanBounds
        | FillBench::CallReplace
        | FillBench::CallCapture
        | FillBench::CallSourceMatches
        | FillBench::KeepGlobFewWords
        | FillBench::KeepGlobOwnWords
        | FillBench::IndexNodesKind
        | FillBench::IndexNodesParent
        | FillBench::IndexNodesId => false,
    };
    if matches!(
        bench,
        FillBench::KeepGlobFewWords | FillBench::KeepGlobOwnWords
    ) {
        // The words of these two: 26 literal kinds (words 1 to 26) and one more (27). A token's kind is the 26th, a
        // group's and the root's the one that is not among them.
        let names: Vec<String> = (b'a'..=b'z')
            .map(|letter| format!("kind_{}", char::from(letter)))
            .chain(std::iter::once("kind_other".to_owned()))
            .collect();
        let texts: Vec<NameText<'_>> = names.iter().map(|name| NameText(name)).collect();
        world.words = file_words(&base_words(checked(&[]).ok()?, &texts));
        world.programs.kinds = [0_u64, 27, 27, 26].map(CellWord).to_vec();
    }
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
    Some(FillWorkload(world, bench))
}

/// Makes the workload's table again and answers how many rows it holds: the table alone is unmade first, its sources
/// stay.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
/// use ryo_engine::domain::made_tables::fixtures::benches::{fill_workload, run_fill};
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
/// // An index of the nodes, built and asked once: the 700 tokens by kind, the root's 100 children and the root (its
/// // own parent) by parent, one node by id.
/// // A keep of a glob of 26 literals: the 700 tokens, whose kind's name is the last of them; and the 100 groups and
/// // the root, whose own texts match none.
/// let kept = [FillBench::KeepGlobFewWords, FillBench::KeepGlobOwnWords];
/// assert_eq!(kept.map(made), [700, 101]);
/// let nodes = [FillBench::IndexNodesKind, FillBench::IndexNodesParent, FillBench::IndexNodesId];
/// assert_eq!(nodes.map(made), [700, 101, 1]);
/// ```
pub fn run_fill(work: &mut FillWorkload) -> usize {
    let asked = match work.1 {
        FillBench::IndexNodesKind => Some(TOKEN_KIND),
        FillBench::IndexNodesParent => Some(CellWord(node_id(FILE, Position(0)).0)),
        FillBench::IndexNodesId => Some(CellWord(node_id(FILE, Position(1)).0)),
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
        | FillBench::GroupEnds
        | FillBench::RowsFile
        | FillBench::RowsFileDropped
        | FillBench::KeepGlobFewWords
        | FillBench::KeepGlobOwnWords
        | FillBench::Product => None,
    };
    if let Some(key) = asked {
        // A new file: nothing of the last one is held, so the index is built again, then asked once.
        next_file(&mut work.0.store);
        let (mut frame, _) = framed(&mut work.0);
        let site = node_site(frame.programs.code);
        return matched(&mut frame, &mut OneRow, site, &[key], KeyFrom::Constants)
            .map_or(0, |found| {
                found.rows.end.0.saturating_sub(found.rows.start.0)
            });
    }
    if work.1 == FillBench::KeepGlobOwnWords {
        // A new file: its words are interned again, and nothing decided of the last file's words is held.
        next_file(&mut work.0.store);
        clear_words(&mut work.0.words);
    }
    if let Some(slot) = work.0.store.tables.first_mut() {
        slot.state = MadeState::Unmade;
    }
    let (mut frame, _) = framed(&mut work.0);
    let _now = made(&mut frame, &mut OneRow, MadeTableId(0));
    table_of(frame.store, MadeTableId(0)).map_or(0, |slot| slot.rows.count.0)
}
