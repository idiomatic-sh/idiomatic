//! The fill side's bench workloads: whole fills through the seam's `made`, on packs the contract's encoding writes. Each
//! makes one table from a relation or from the file's nodes that are there already, so a run times the pipeline from its
//! scan to its write (and a grouping fill's grouping) and neither the language nor an index build.

use crate::domain::code::types::RelationNumber;
use crate::domain::code::types::ShapeChoice;
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{
    DefinitionPattern, Emptiness, FillInput, GroupFill, MadeFill, MadeTable, OrderedFill,
    PackDefinition, PatternForm,
};
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, keep_equal, node_read, of, pipeline, probe, read, rows, scan, scan_nodes, table,
    used, walk,
};
use crate::domain::made_tables::fixtures::{Telling, World, framed, site_of, world_of};
use crate::domain::made_tables::sources::service::{made, matched, scanned, table_of};
use crate::domain::made_tables::types::{
    FillFrame, KeyFrom, MadeState, PatternAsk, PatternFound, PatternRows, ReadSource, TableNeed,
};
use crate::domain::steps::stages::types::{FillStage, TreeWalk, UseMode};
use crate::domain::steps::types::{
    AxisDirection, ColumnList, ColumnMask, MadeColumn, MadeTableId, PlacedStep, ProgramStep,
    Register, RegisterList, StepProgram, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, token};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, NodeColumn, Offset, Relation};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;

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
}

/// Every fill the bench times.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::FILL_BENCHES;
///
/// assert_eq!(FILL_BENCHES.len(), 10);
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

/// How many rows a key holds where a workload groups.
const GROUP_ROWS: u64 = 8;

/// The relation of a workload of `rows` rows: the first cell a key in a scattered order, the second what the fill reads
/// beside it.
fn told_rows(bench: FillBench, rows: usize) -> Vec<[u64; 2]> {
    let all = u64::try_from(rows).unwrap_or(u64::MAX).max(1);
    (0..all)
        .map(|row| match bench {
            // Every row its own key; the second cell is another row's key, so a probe finds one row.
            FillBench::RowsRead | FillBench::RowsKeep | FillBench::RowsProbe => [
                row.wrapping_mul(7919) % all,
                row.wrapping_mul(104_729) % all,
            ],
            FillBench::RowsDropped
            | FillBench::GroupCount
            | FillBench::Sorted
            | FillBench::UseRows => [row / GROUP_ROWS, row.wrapping_mul(31) % 43],
            FillBench::WalkChildren | FillBench::WalkAncestors | FillBench::NodeKeep => [row, row],
        })
        .collect()
}

/// A file of about `elements` elements: a root (kind 1) whose children (kind 2) each hold seven tokens (kind 3).
fn tree(elements: usize) -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(4));
    let groups = u32::try_from(elements / 8).unwrap_or(u32::MAX).max(1);
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    for group in 0..groups {
        let start = group.saturating_mul(7);
        enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(start));
        for place in 0..7 {
            let at = start.saturating_add(place);
            token(
                &mut writer,
                KindBits(3),
                FieldCode(0),
                Offset(at),
                Offset(at.saturating_add(1)),
            );
        }
        leave_node(&mut writer, Offset(start.saturating_add(7)));
    }
    leave_node(&mut writer, Offset(groups.saturating_mul(7)));
    let _table = finished(&mut writer);
    made
}

/// The input of a grouping fill: the relation's two columns, keyed by the first.
fn grouped(order: Option<MadeColumn>) -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline(
            vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
            [of(1), of(2)],
        )]),
        keys: ColumnList(Box::new([MadeColumn(0)])),
        order,
    }
}

/// The table a workload makes.
fn bench_table(bench: FillBench) -> MadeTable {
    let tree = Relation::SemanticSyntax;
    let kept = |fill| table(1, 2, fill);
    match bench {
        FillBench::RowsRead => rows(
            2,
            pipeline(
                vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                [of(1), of(2)],
            ),
        ),
        FillBench::RowsKeep => rows(
            2,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    read(SECOND, 0, 2),
                    FillStage::Keep {
                        test: TestOperation::Less,
                        left: of(1),
                        right: of(2),
                    },
                ],
                [of(1), of(2)],
            ),
        ),
        FillBench::RowsProbe => rows(
            2,
            pipeline(
                vec![
                    scan(0),
                    read(SECOND, 0, 2),
                    probe(2, None, UseMode::Rows, 3),
                    read(SECOND, 3, 4),
                ],
                [of(2), of(4)],
            ),
        ),
        FillBench::RowsDropped => table(
            1,
            2,
            MadeFill::Rows(
                Box::new([pipeline(
                    vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
                    [of(1), of(2)],
                )]),
                RepeatedRows::Dropped,
            ),
        ),
        FillBench::GroupCount => kept(MadeFill::Group(
            GroupFill::Count { distinct: None },
            grouped(None),
        )),
        FillBench::Sorted => kept(MadeFill::Ordered(OrderedFill::Sorted(grouped(Some(
            MadeColumn(1),
        ))))),
        FillBench::UseRows => rows(
            2,
            pipeline(
                vec![scan(0), read(FIRST, 0, 1), used(1, UseMode::Rows, &[5])],
                [of(1), of(5)],
            ),
        ),
        FillBench::WalkChildren => {
            let mut all = Vec::from(scan_nodes(tree));
            all.push(walk(tree, TreeWalk::Children(None), 0, 2));
            rows(2, pipeline(all, [of(0), of(2)]))
        }
        FillBench::WalkAncestors => {
            let mut all = Vec::from(scan_nodes(tree));
            all.push(keep_equal(1, 13));
            all.push(walk(tree, TreeWalk::Axis(AxisDirection::Ancestor), 0, 2));
            rows(2, pipeline(all, [of(0), of(2)]))
        }
        FillBench::NodeKeep => {
            let mut all = Vec::from(scan_nodes(tree));
            all.push(keep_equal(1, 13));
            all.push(node_read(tree, NodeColumn::Start, 0, 2));
            rows(2, pipeline(all, [of(0), of(2)]))
        }
    }
}

/// A definition of two columns with one pattern in call form, bound by its first column and collecting its second.
fn called() -> PackDefinition {
    PackDefinition {
        node: NodeDigest([80; 16]),
        formals: RegisterList(Box::new([Register(800), Register(801)])),
        emptiness: Emptiness::Unknown,
        patterns: vec![DefinitionPattern {
            bound: ColumnMask(Box::new([MadeColumn(0)])),
            collected: ColumnMask(Box::new([MadeColumn(1)])),
            form: PatternForm::Call(StepProgram(Box::new([PlacedStep {
                point: None,
                step: ProgramStep::Yield {
                    row: RegisterList(Box::new([Register(801)])),
                },
            }]))),
            shape: ShapeChoice::Decided,
        }],
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
    let reads_told = match bench {
        FillBench::RowsRead
        | FillBench::RowsKeep
        | FillBench::RowsProbe
        | FillBench::RowsDropped
        | FillBench::GroupCount
        | FillBench::Sorted
        | FillBench::UseRows => true,
        FillBench::WalkChildren | FillBench::WalkAncestors | FillBench::NodeKeep => false,
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
            let site = site_of(&frame.programs.code, 1, &[0]);
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
/// ```
pub fn run_fill(work: &mut FillWorkload) -> usize {
    if let Some(slot) = work.0.store.tables.first_mut() {
        slot.state = MadeState::Unmade;
    }
    let (mut frame, _) = framed(&mut work.0);
    let _now = made(&mut frame, &mut OneRow, MadeTableId(0));
    table_of(frame.store, MadeTableId(0)).map_or(0, |slot| slot.rows.count.0)
}
