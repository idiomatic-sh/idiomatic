//! The cases of `built::service::runs`, each over a pack the contract's encoding wrote: every stage word of the pack in
//! the code's order with what the fill side says of it, then each table's fill. What it says here is what a pipeline
//! and a table's making ask before they run, so a case of a stage or a fill that runs is beside the case that runs it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::FillRuns;
use crate::domain::lowered::types::{
    FillInput, GraphFill, GroupFill, MadeFill, MadeTable, OrderedFill,
};
use crate::domain::made_tables::built::service::runs;
use crate::domain::made_tables::built::types::FillPart;
use crate::domain::made_tables::carrying::service::stage_kind;
use crate::domain::made_tables::fills::service::fill_code_at;
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, coded, constants, keep_equal, node_read, of, operands, pipeline, probe, read,
    rows, scan, scan_nodes, table, walk, word,
};
use crate::domain::steps::stages::types::{FillStage, ProbeRange, TreeWalk, UseMode};
use crate::domain::steps::types::{
    BoundKind, CallMeaning, ColumnList, MadeColumn, MadeTableId, MatchColumns, RangeBound,
    ReadColumn, Register, RegisterList, StepRows, TestOperation,
};
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use ryo_values::domain::plan_orders::types::GroupEnd;
use std::borrow::Cow;

/// The pack a case asks about.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A pipeline of the stages the first rule holds: a scan, reads, a keep of two cells, a probe, a walk, the write.
    Stages,
    /// Keeps of texts: a prefix, which runs, and the numeric order of two texts, which does not yet.
    Keeps,
    /// Calls: the bounds of a span, which runs, and the lower case of a text, which does not yet.
    Calls,
    /// A probe of the nodes by their parent held to a range of their start: an index over the nodes that sorts each
    /// key's rows, which is not built yet.
    OrderedNodes,
    /// Every fill the lowering writes for a table: constant rows, rows, a count, a sorted table, a group's ends, an
    /// ordered match, a group's places, and a graph's cycles, which does not run yet.
    Fills,
}

type Input = Asked;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_stage_of_the_first_rules_kinds_runs"),
        word: Word::Succeeds,
        input: || Asked::Stages,
        expect: Expect::Value(Cow::Borrowed(
            "StageScan runs, StageRead runs, StageKeep runs, StageProbe runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWalk runs, StageWrite runs | fills: runs, runs | a word that is no stage: not yet",
        )),
    },
    Case {
        name: CaseName("a_keep_of_a_prefix_runs_and_one_of_a_numeric_order_does_not_yet"),
        word: Word::Succeeds,
        input: || Asked::Keeps,
        expect: Expect::Value(Cow::Borrowed(
            "StageScan runs, StageRead runs, StageRead runs, StageKeep runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageKeep not yet, StageWrite runs | fills: runs, runs | a word that is no stage: not yet",
        )),
    },
    Case {
        name: CaseName("a_call_of_span_bounds_runs_and_one_of_words_does_not"),
        word: Word::Succeeds,
        input: || Asked::Calls,
        expect: Expect::Value(Cow::Borrowed(
            "StageScan runs, StageRead runs, StageRead runs, StageCall runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageCall not yet, StageWrite runs | fills: runs, runs | a word that is no stage: not yet",
        )),
    },
    Case {
        name: CaseName("every_fill_built_runs_and_the_cycles_fill_not_yet"),
        word: Word::Succeeds,
        input: || Asked::Fills,
        expect: Expect::Value(Cow::Borrowed(
            "StageScan runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs, StageScan runs, StageRead runs, StageRead runs, StageWrite runs | fills: runs, runs, runs, runs, runs, runs, runs, not yet | a word that is no stage: not yet",
        )),
    },
    Case {
        name: CaseName("a_probe_of_the_nodes_with_a_range_does_not_run_yet"),
        word: Word::Succeeds,
        input: || Asked::OrderedNodes,
        expect: Expect::Value(Cow::Borrowed(
            "StageScan runs, StageRead runs, StageRead runs, StageProbe not yet, StageWrite runs | fills: runs | a word that is no stage: not yet",
        )),
    },
];

/// The input of a grouping fill: the relation's two columns, keyed by the first and ordered by the second.
fn input() -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline(
            vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)],
            [of(1), of(2)],
        )]),
        keys: ColumnList(Box::new([MadeColumn(0)])),
        order: Some(MadeColumn(1)),
    }
}

/// The tables of a case.
fn tables(asked: Asked) -> Vec<MadeTable> {
    let tree = Relation::SemanticSyntax;
    let keep = |test| FillStage::Keep {
        test,
        left: of(2),
        right: word(3),
    };
    let call = |meaning, outputs: &[u32]| FillStage::Call {
        meaning,
        arguments: operands([of(2)]),
        outputs: RegisterList(outputs.iter().map(|each| Register(*each)).collect()),
    };
    let over_nodes = |column, stages: Vec<FillStage>| {
        let mut all = Vec::from(scan_nodes(tree));
        all.push(node_read(tree, column, 0, 2));
        all.extend(stages);
        rows(1, pipeline(all, [of(0)]))
    };
    match asked {
        Asked::Stages => vec![
            rows(
                2,
                pipeline(
                    vec![
                        scan(0),
                        read(FIRST, 0, 1),
                        keep_equal(1, 1),
                        probe(1, None, UseMode::Rows, 2),
                        read(SECOND, 2, 3),
                    ],
                    [of(1), of(3)],
                ),
            ),
            over_nodes(
                NodeColumn::Start,
                vec![walk(tree, TreeWalk::Children(None), 0, 3)],
            ),
        ],
        Asked::Keeps => vec![
            over_nodes(NodeColumn::Text, vec![keep(TestOperation::StartsWith)]),
            over_nodes(NodeColumn::Text, vec![keep(TestOperation::NumericAtLeast(crate::domain::steps::types::TEXTS))]),
        ],
        Asked::Calls => vec![
            over_nodes(
                NodeColumn::Span,
                vec![call(CallMeaning::SpanBounds, &[3, 4])],
            ),
            over_nodes(NodeColumn::Text, vec![call(CallMeaning::Words, &[3])]),
        ],
        Asked::OrderedNodes => {
            let bound = |cell: u64, end: BoundKind| RangeBound {
                value: word(cell),
                end,
            };
            vec![over_nodes(
                NodeColumn::Start,
                vec![FillStage::Probe {
                    source: StepRows::Node(tree),
                    columns: MatchColumns(Box::new([ReadColumn::Node(NodeColumn::Parent)])),
                    values: operands([of(0)]),
                    range: Some(ProbeRange {
                        column: ReadColumn::Node(NodeColumn::Start),
                        low: bound(0, BoundKind::Strict),
                        high: bound(6, BoundKind::Inclusive),
                    }),
                    mode: UseMode::Rows,
                    out: Register(3),
                }],
            )]
        }
        Asked::Fills => vec![
            constants(&[1, 2]),
            rows(1, pipeline(vec![scan(0), read(FIRST, 0, 1)], [of(1)])),
            table(
                1,
                2,
                MadeFill::Group(GroupFill::Count { distinct: None }, input()),
            ),
            table(2, 2, MadeFill::Ordered(OrderedFill::Sorted(input()))),
            table(
                3,
                2,
                MadeFill::Group(GroupFill::Ends(GroupEnd::Least), input()),
            ),
            table(
                4,
                4,
                MadeFill::Ordered(OrderedFill::Match(input(), input())),
            ),
            table(5, 3, MadeFill::Group(GroupFill::Places, input())),
            table(
                6,
                2,
                MadeFill::Graph(
                    GraphFill::Cycles(crate::domain::code::records::types::EdgeColumns {
                        source: MadeColumn(0),
                        destination: MadeColumn(1),
                    }),
                    // A cycles fill's edges have no key and no order: the encoding holds the table to that.
                    Box::new([FillInput {
                        keys: ColumnList(Box::new([])),
                        order: None,
                        ..input()
                    }]),
                ),
            ),
        ],
    }
}

/// What the fill side says, as a word.
const fn said(answer: FillRuns) -> &'static str {
    match answer {
        FillRuns::Runs => "runs",
        FillRuns::NotYet => "not yet",
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let made_tables = tables(asked);
    let count = u32::try_from(made_tables.len()).unwrap_or(0);
    let code = coded(made_tables, Vec::new())?;
    let stages: Vec<String> = code
        .words
        .iter()
        .filter(|each| stage_kind(each.op).is_some())
        .map(|each| {
            format!(
                "{:?} {}",
                each.op,
                said(runs(&code, FillPart::Stage(*each)))
            )
        })
        .collect();
    let mut fills = Vec::new();
    for table in 0..count {
        let fill = fill_code_at(&code, MadeTableId(table))?;
        fills.push(said(runs(&code, FillPart::Fill(fill))));
    }
    // A word of a step program is no stage of a pipeline.
    let other = code
        .words
        .iter()
        .find(|each| stage_kind(each.op).is_none())
        .map_or("none", |each| said(runs(&code, FillPart::Stage(*each))));
    Ok(Cow::Owned(format!(
        "{} | fills: {} | a word that is no stage: {other}",
        stages.join(", "),
        fills.join(", ")
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
