//! The graph tables the encoding writes. A graph fill the contract states is written with the record of the columns its
//! kernel is told, at the place the table's one parameter names, and those columns read back through the record's own
//! reader. A graph table that is not the width its row states, or whose fill is not one the contract states, is the
//! encoding's fault of the rule being written: every read of it by place would be of the wrong column.

use super::super::fixtures::{of, reg, rule, scan, stages, table_pack};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::service::edge_columns_at;
use crate::domain::code::records::types::EdgeColumns;
use crate::domain::code::service::list;
use crate::domain::code::types::{PoolAt, ProgramCode, WordSlot};
use crate::domain::encoding::constants::LAST_STEP;
use crate::domain::encoding::service::encoded;
use crate::domain::lowered::types::{
    Emptiness, FillInput, GraphFill, LoweredPack, MadeFill, MadeTable,
};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::LoadRefusal;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::types::{
    ColumnList, MadeColumn, MadeTableId, ProgramStep, StepOperands, StepRows,
};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The pack a row encodes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A `follow` of edges keyed on their first column, in a table of three columns.
    Follow,
    /// The same `follow` in a table of the two columns of a source and a fixed point.
    FollowNarrow,
    /// The same `follow` whose edges name an order column.
    FollowOrdered,
    /// The same `follow` told a destination past its edges' row.
    FollowPast,
    /// A graph fill whose row the contract does not state.
    Unstated,
}

type Input = Scenario;
/// What a row reads of the code written.
type Value = Cow<'static, str>;
type Failure = LoweringError;

/// The encoding's own fault of the second rule, the one that reaches the graph table.
const FAULT: LoweringError = LoweringError::Refused {
    rule: SelectedRuleOrdinal(1),
    refusal: LoadRefusal::Unlowerable(LAST_STEP),
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_follow_two_wider_than_its_keys_is_written_and_its_edge_columns_read_back",
        ),
        word: Word::Succeeds,
        input: || Scenario::Follow,
        expect: Expect::Value(Cow::Borrowed(
            "width WordSlot(3); the parameter WordSlot(9) names Some(EdgeColumns { source: MadeColumn(1), destination: MadeColumn(2) }); 1 pipeline, keys [WordSlot(0)], order WordSlot(4294967295)",
        )),
    },
    Case {
        name: CaseName("a_follow_as_wide_as_a_source_and_a_fixed_point_alone_is_refused"),
        word: Word::Refuses,
        input: || Scenario::FollowNarrow,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_follow_whose_edges_are_ordered_is_refused"),
        word: Word::Refuses,
        input: || Scenario::FollowOrdered,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_follow_told_a_column_past_its_edges_row_is_refused"),
        word: Word::Refuses,
        input: || Scenario::FollowPast,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_graph_fill_whose_row_is_not_stated_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Unstated,
        expect: Expect::Error(FAULT),
    },
];

/// An input of one pipeline whose rows are `cells` wide, keyed on `keys`, ordered by `order`.
fn input(cells: u32, keys: &[u16], order: Option<u16>) -> FillInput {
    FillInput {
        pipelines: Box::new([Pipeline {
            stages: stages(vec![scan(1)]),
            row: StepOperands((0..cells).map(|_| of(1)).collect()),
        }]),
        keys: ColumnList(keys.iter().copied().map(MadeColumn).collect()),
        order: order.map(MadeColumn),
    }
}

/// A pack of two rules: the first walks table 0, a table of rows, and the second walks table 1, a graph table of `width`
/// columns made by `fill` over `inputs`. Table 0 is written first, so the graph table's records stand past the pool's start.
fn graph_pack(fill: GraphFill, inputs: Vec<FillInput>, width: usize) -> LoweredPack {
    let mut made = table_pack(vec![scan(1)], 1, Vec::new());
    made.rules.push(rule(
        1,
        vec![ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(1)),
            out: reg(0),
        }],
    ));
    made.tables.push(MadeTable {
        node: NodeDigest([7; 16]),
        fill: MadeFill::Graph(fill, inputs.into()),
        width: ColumnCount(width),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    });
    made
}

/// The edge columns of the rows' fills: an edge row is its key, its source, its destination.
const fn edges(destination: u16) -> EdgeColumns {
    EdgeColumns {
        source: MadeColumn(1),
        destination: MadeColumn(destination),
    }
}

fn lowered(scenario: Scenario) -> LoweredPack {
    let follow = GraphFill::FixedPoints(edges(2));
    match scenario {
        Scenario::Follow => graph_pack(follow, vec![input(3, &[0], None)], 3),
        Scenario::FollowNarrow => graph_pack(follow, vec![input(3, &[0], None)], 2),
        Scenario::FollowOrdered => graph_pack(follow, vec![input(3, &[0], Some(1))], 3),
        Scenario::FollowPast => graph_pack(
            GraphFill::FixedPoints(edges(3)),
            vec![input(3, &[0], None)],
            3,
        ),
        Scenario::Unstated => graph_pack(GraphFill::Cycles, vec![input(2, &[], None)], 2),
    }
}

/// The parts of the graph table's record as 2.8 lays them out: its width, then the list of its index sites, then its one
/// parameter; and after them its first input's lists of pipeline entries and of key columns, and its order column.
fn record(code: &ProgramCode) -> Option<String> {
    let table = code.tables.get(1)?;
    let slot = |at: u32| code.pool.get(usize::try_from(at).ok()?).copied();
    let sites = u32::try_from(list(&code.pool, WordSlot(table.operands.0.checked_add(1)?)).len());
    let parameter = table.operands.0.checked_add(2)?.checked_add(sites.ok()?)?;
    let entries = u32::try_from(list(&code.pool, WordSlot(parameter.checked_add(1)?)).len());
    let keys_at = parameter.checked_add(2)?.checked_add(entries.ok()?)?;
    let keys = list(&code.pool, WordSlot(keys_at));
    let order = keys_at
        .checked_add(1)?
        .checked_add(u32::try_from(keys.len()).ok()?)?;
    Some(format!(
        "width {:?}; the parameter {:?} names {:?}; {} pipeline, keys {keys:?}, order {:?}",
        slot(table.operands.0)?,
        slot(parameter)?,
        edge_columns_at(&code.pool, PoolAt(slot(parameter)?.0)),
        entries.ok()?,
        slot(order)?,
    ))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let code = encoded(&lowered(scenario))?;
    Ok(Cow::Owned(record(&code).unwrap_or_default()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
