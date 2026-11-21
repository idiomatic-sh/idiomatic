//! The tables of the fill bench: what each workload's relation holds, its file of nodes and its text, and the table it makes.

use crate::domain::code::types::ShapeChoice;
use crate::domain::lowered::types::{
    DefinitionPattern, Emptiness, FillInput, GroupFill, MadeFill, MadeTable, OrderedFill,
    PackDefinition, PatternForm,
};
use crate::domain::made_tables::fixtures::benches::FillBench;
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, TOLD, keep_equal, node_read, of, operands, pipeline, probe, read, rows, scan,
    scan_nodes, table, used, walk, word,
};
use crate::domain::steps::calls::types::{SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::stages::types::{FillStage, TreeWalk, UseMode};
use crate::domain::steps::types::{
    AxisDirection, CallMeaning, ColumnList, ColumnMask, MadeColumn, PlacedStep, ProgramStep,
    RegexId, Register, RegisterList, StepOperand, StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, token};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, NodeColumn, Offset, Relation};
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;

/// How many rows the product's workload pairs with every row.
pub(crate) const PRODUCT_LEFT: usize = 32;

/// How many bytes a token of a workload's file covers.
const TOKEN_BYTES: u32 = 2;

/// How many rows a key holds where a workload groups.
const GROUP_ROWS: u64 = 8;

/// The relation of a workload of `rows` rows: the first cell a key in a scattered order, the second what the fill reads
/// beside it.
pub(crate) fn told_rows(bench: FillBench, rows: usize) -> Vec<[u64; 2]> {
    let all = u64::try_from(rows).unwrap_or(u64::MAX).max(1);
    (0..all)
        .map(|row| match bench {
            // Every row its own key; the second cell is another row's key, so a probe finds one row.
            FillBench::RowsRead
            | FillBench::RowsKeep
            | FillBench::RowsProbe
            | FillBench::Product => [
                row.wrapping_mul(7919) % all,
                row.wrapping_mul(104_729) % all,
            ],
            FillBench::RowsDropped
            | FillBench::GroupCount
            | FillBench::Sorted
            | FillBench::UseRows
            | FillBench::OrderedMatch
            | FillBench::GroupEnds => [row / GROUP_ROWS, row.wrapping_mul(31) % 43],
            FillBench::WalkChildren
            | FillBench::WalkAncestors
            | FillBench::NodeKeep
            | FillBench::CallSpanBounds
            | FillBench::CallReplace
            | FillBench::CallCapture
            | FillBench::CallSourceMatches => [row, row],
        })
        .collect()
}

/// A file of about `elements` elements: a root (kind 1) whose children (kind 2) each hold seven tokens (kind 3), each
/// token two bytes of the text.
pub(crate) fn tree(elements: usize) -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(4));
    let groups = u32::try_from(elements / 8).unwrap_or(u32::MAX).max(1);
    let wide = TOKEN_BYTES.saturating_mul(7);
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    for group in 0..groups {
        let start = group.saturating_mul(wide);
        enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(start));
        for place in 0..7_u32 {
            let at = start.saturating_add(place.saturating_mul(TOKEN_BYTES));
            token(
                &mut writer,
                KindBits(3),
                FieldCode(0),
                Offset(at),
                Offset(at.saturating_add(TOKEN_BYTES)),
            );
        }
        leave_node(&mut writer, Offset(start.saturating_add(wide)));
    }
    leave_node(&mut writer, Offset(groups.saturating_mul(wide)));
    let _table = finished(&mut writer);
    made
}

/// The text of the file `tree` describes: the letters `a` to `g` over and over, so the tokens hold seven different
/// texts of two letters and a group's fourteen bytes hold no space.
pub(crate) fn bench_text(elements: usize) -> Vec<u8> {
    let bytes = (elements / 8).max(1).saturating_mul(14);
    b"abcdefg".iter().copied().cycle().take(bytes).collect()
}

/// A call stage.
fn call<const N: usize>(
    meaning: CallMeaning,
    arguments: [StepOperand; N],
    outputs: &[u32],
) -> FillStage {
    FillStage::Call {
        meaning,
        arguments: operands(arguments),
        outputs: RegisterList(outputs.iter().map(|each| Register(*each)).collect()),
    }
}

/// A rows fill over the nodes of one kind: a column read, the call, the write of the node and the call's outputs.
fn called_rows<const N: usize>(
    kind: u64,
    column: NodeColumn,
    stage: FillStage,
    row: [StepOperand; N],
) -> MadeTable {
    let tree = Relation::SemanticSyntax;
    let mut all = Vec::from(scan_nodes(tree));
    all.push(keep_equal(1, kind));
    all.push(node_read(tree, column, 0, 2));
    all.push(stage);
    rows(N, pipeline(all, row))
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
pub(crate) fn bench_table(bench: FillBench) -> MadeTable {
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
        FillBench::CallSpanBounds => called_rows(
            13,
            NodeColumn::Span,
            call(CallMeaning::SpanBounds, [of(2)], &[3, 4]),
            [of(0), of(3), of(4)],
        ),
        FillBench::CallReplace => called_rows(
            13,
            NodeColumn::Text,
            call(CallMeaning::Replace(RegexId(1)), [of(2), word(4)], &[3]),
            [of(2), of(3)],
        ),
        FillBench::CallCapture => called_rows(
            13,
            NodeColumn::Text,
            call(CallMeaning::Capture, [of(2), word(6)], &[3, 4]),
            [of(2), of(3), of(4)],
        ),
        FillBench::CallSourceMatches => called_rows(
            12,
            NodeColumn::Span,
            call(
                CallMeaning::SourceMatches(
                    RegexId(3),
                    SourceSearch {
                        lines: SearchLines::Unnumbered,
                        group: SearchGroup::Whole,
                    },
                ),
                [of(2)],
                &[3],
            ),
            [of(0), of(3)],
        ),
        FillBench::OrderedMatch => table(
            1,
            4,
            MadeFill::Ordered(OrderedFill::Match(
                grouped(Some(MadeColumn(1))),
                grouped(Some(MadeColumn(1))),
            )),
        ),
        FillBench::GroupEnds => kept(MadeFill::Group(
            GroupFill::Ends(GroupEnd::Greatest),
            grouped(Some(MadeColumn(1))),
        )),
        FillBench::Product => rows(
            2,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    FillStage::Keep {
                        test: TestOperation::Less,
                        left: of(1),
                        right: word(32),
                    },
                    FillStage::Product {
                        source: StepRows::Filled(TOLD),
                        out: Register(3),
                    },
                    read(FIRST, 3, 4),
                    FillStage::Keep {
                        test: TestOperation::Equal,
                        left: of(4),
                        right: of(1),
                    },
                ],
                [of(1), of(4)],
            ),
        ),
    }
}

/// A definition of two columns with one pattern in call form, bound by its first column and collecting its second.
pub(crate) fn called() -> PackDefinition {
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
