//! The graph tables the encoding writes. A graph fill the contract states is written with the record of the columns its
//! kernel is told, at the place the table's one parameter names, and those columns read back through the record's own
//! reader. A graph table that is not the width its row states, or whose fill is not one the contract states, is the
//! encoding's fault of the rule that reaches it: every read of it by place would be of the wrong column.

use super::super::fixtures::{of, reg, rule, scan, stages, table_pack};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::service::{
    edge_columns_at, visit_columns_at, worklist_columns_at,
};
use crate::domain::code::records::types::{EdgeColumns, VisitColumns, WorklistColumns};
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
    /// An `ordered-dfs` of roots and ordered edges, each keyed on its first column, in a table of six columns.
    Walk,
    /// The same `ordered-dfs` in a table of the five columns its row has beside the key.
    WalkNarrow,
    /// The same `ordered-dfs` whose edges name no order column.
    WalkUnordered,
    /// A `worklist` of roots, ordered edges and a lattice, the first two keyed on their first column, in a table of seven
    /// columns.
    Worklist,
    /// The same `worklist` whose lattice is keyed.
    WorklistKeyedLattice,
    /// The same `worklist` with no lattice.
    WorklistTwoInputs,
    /// A `cycles` of edges with no key, in a table of two columns.
    Cycles,
    /// The same `cycles` whose edges are keyed, in a table of three columns.
    CyclesKeyed,
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
            "width 3; the parameter 9 names Some(EdgeColumns { source: MadeColumn(1), destination: MadeColumn(2) }); inputs [\"1 pipeline keyed [0] ordered 4294967295\"]",
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
        name: CaseName(
            "an_ordered_walk_five_wider_than_its_keys_is_written_and_its_four_columns_read_back",
        ),
        word: Word::Succeeds,
        input: || Scenario::Walk,
        expect: Expect::Value(Cow::Borrowed(
            "width 6; the parameter 9 names Some(VisitColumns { root: MadeColumn(1), source: MadeColumn(1), destination: MadeColumn(2), edge: MadeColumn(4) }); inputs [\"1 pipeline keyed [0] ordered 4294967295\", \"1 pipeline keyed [0] ordered 3\"]",
        )),
    },
    Case {
        name: CaseName("an_ordered_walk_as_wide_as_its_row_without_its_key_is_refused"),
        word: Word::Refuses,
        input: || Scenario::WalkNarrow,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("an_ordered_walk_whose_edges_name_no_order_is_refused"),
        word: Word::Refuses,
        input: || Scenario::WalkUnordered,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName(
            "a_worklist_six_wider_than_its_keys_is_written_and_its_eight_columns_read_back",
        ),
        word: Word::Succeeds,
        input: || Scenario::Worklist,
        expect: Expect::Value(Cow::Borrowed(
            "width 7; the parameter 9 names Some(WorklistColumns { root: MadeColumn(1), state: MadeColumn(2), source: MadeColumn(1), destination: MadeColumn(2), before: MadeColumn(4), after: MadeColumn(5), lower: MadeColumn(0), upper: MadeColumn(1) }); inputs [\"1 pipeline keyed [0] ordered 4294967295\", \"1 pipeline keyed [0] ordered 3\", \"1 pipeline keyed [] ordered 4294967295\"]",
        )),
    },
    Case {
        name: CaseName("a_worklist_whose_lattice_is_keyed_is_refused"),
        word: Word::Refuses,
        input: || Scenario::WorklistKeyedLattice,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_worklist_with_no_lattice_is_refused"),
        word: Word::Refuses,
        input: || Scenario::WorklistTwoInputs,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_cycles_of_two_columns_is_written_and_its_edge_columns_read_back"),
        word: Word::Succeeds,
        input: || Scenario::Cycles,
        expect: Expect::Value(Cow::Borrowed(
            "width 2; the parameter 9 names Some(EdgeColumns { source: MadeColumn(0), destination: MadeColumn(1) }); inputs [\"1 pipeline keyed [] ordered 4294967295\"]",
        )),
    },
    Case {
        name: CaseName("a_cycles_whose_edges_are_keyed_is_refused"),
        word: Word::Refuses,
        input: || Scenario::CyclesKeyed,
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

/// The edge columns of a `follow`'s fill: an edge row is its key, its source, its destination.
const fn edges(destination: u16) -> EdgeColumns {
    EdgeColumns {
        source: MadeColumn(1),
        destination: MadeColumn(destination),
    }
}

/// The columns of the ordered walk: a root row is its key and its root; an edge row is its key, its source, its
/// destination, its order and its identity.
const VISITS: VisitColumns = VisitColumns {
    root: MadeColumn(1),
    source: MadeColumn(1),
    destination: MadeColumn(2),
    edge: MadeColumn(4),
};

/// The columns of the worklist: a root row is its key, its root and its state; an edge row is its key, its source, its
/// destination, its order and its two states; a lattice row is its two states.
const STATES: WorklistColumns = WorklistColumns {
    root: MadeColumn(1),
    state: MadeColumn(2),
    source: MadeColumn(1),
    destination: MadeColumn(2),
    before: MadeColumn(4),
    after: MadeColumn(5),
    lower: MadeColumn(0),
    upper: MadeColumn(1),
};

/// The edge columns of the `cycles`: an edge row is its source and its destination.
const RING: EdgeColumns = EdgeColumns {
    source: MadeColumn(0),
    destination: MadeColumn(1),
};

fn lowered(scenario: Scenario) -> LoweredPack {
    let follow = GraphFill::FixedPoints(edges(2));
    let (walk, worklist) = (GraphFill::OrderedDfs(VISITS), GraphFill::Worklist(STATES));
    let roots = |cells| input(cells, &[0], None);
    let ordered = |cells| input(cells, &[0], Some(3));
    let lattice = |keys: &[u16]| input(2, keys, None);
    match scenario {
        Scenario::Follow => graph_pack(follow, vec![input(3, &[0], None)], 3),
        Scenario::FollowNarrow => graph_pack(follow, vec![input(3, &[0], None)], 2),
        Scenario::FollowOrdered => graph_pack(follow, vec![input(3, &[0], Some(1))], 3),
        Scenario::FollowPast => graph_pack(
            GraphFill::FixedPoints(edges(3)),
            vec![input(3, &[0], None)],
            3,
        ),
        Scenario::Walk => graph_pack(walk, vec![roots(2), ordered(5)], 6),
        Scenario::WalkNarrow => graph_pack(walk, vec![roots(2), ordered(5)], 5),
        Scenario::WalkUnordered => graph_pack(walk, vec![roots(2), input(5, &[0], None)], 6),
        Scenario::Worklist => graph_pack(worklist, vec![roots(3), ordered(6), lattice(&[])], 7),
        Scenario::WorklistKeyedLattice => {
            graph_pack(worklist, vec![roots(3), ordered(6), lattice(&[0])], 7)
        }
        Scenario::WorklistTwoInputs => graph_pack(worklist, vec![roots(3), ordered(6)], 7),
        Scenario::Cycles => graph_pack(GraphFill::Cycles(RING), vec![input(2, &[], None)], 2),
        Scenario::CyclesKeyed => graph_pack(GraphFill::Cycles(RING), vec![input(3, &[0], None)], 3),
    }
}

/// The slot `ahead` places after `at` in the pool.
fn slot(code: &ProgramCode, at: u32, ahead: usize) -> Option<WordSlot> {
    let place = usize::try_from(at).ok()?.checked_add(ahead)?;
    code.pool.get(place).copied()
}

/// The head of the graph table's record as 2.8 lays it out: its width, then the list of its index sites, then its one
/// parameter; and the place its inputs start at.
fn head(code: &ProgramCode) -> Option<(WordSlot, WordSlot, u32)> {
    let table = code.tables.get(1)?;
    let sites = list(&code.pool, WordSlot(table.operands.0.checked_add(1)?)).len();
    let parameter = slot(code, table.operands.0, sites.checked_add(2)?)?;
    let inputs = usize::try_from(table.operands.0)
        .ok()?
        .checked_add(sites)?
        .checked_add(3)?;
    Some((
        slot(code, table.operands.0, 0)?,
        parameter,
        u32::try_from(inputs).ok()?,
    ))
}

/// The `count` inputs of a record that start at `at`, each as its pipelines, its key columns and its order column.
fn inputs(code: &ProgramCode, at: u32, count: usize) -> Option<Vec<String>> {
    let mut at = at;
    let mut read = Vec::new();
    for _ in 0..count {
        let entries = list(&code.pool, WordSlot(at)).len();
        let keys_at = at
            .checked_add(u32::try_from(entries).ok()?)?
            .checked_add(1)?;
        let keys = list(&code.pool, WordSlot(keys_at));
        let order_at = keys_at
            .checked_add(u32::try_from(keys.len()).ok()?)?
            .checked_add(1)?;
        let keys: Vec<u32> = keys.iter().map(|key| key.0).collect();
        read.push(format!(
            "{entries} pipeline keyed {keys:?} ordered {:?}",
            slot(code, order_at, 0)?.0
        ));
        at = order_at.checked_add(1)?;
    }
    Some(read)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let code = encoded(&lowered(scenario))?;
    let Some((width, parameter, first)) = head(&code) else {
        return Ok(Cow::Borrowed("no record"));
    };
    let at = PoolAt(parameter.0);
    let (told, count) = match scenario {
        Scenario::Follow
        | Scenario::FollowNarrow
        | Scenario::FollowOrdered
        | Scenario::FollowPast => (format!("{:?}", edge_columns_at(&code.pool, at)), 1),
        Scenario::Walk | Scenario::WalkNarrow | Scenario::WalkUnordered => {
            (format!("{:?}", visit_columns_at(&code.pool, at)), 2)
        }
        Scenario::Worklist | Scenario::WorklistKeyedLattice | Scenario::WorklistTwoInputs => {
            (format!("{:?}", worklist_columns_at(&code.pool, at)), 3)
        }
        Scenario::Cycles | Scenario::CyclesKeyed => {
            (format!("{:?}", edge_columns_at(&code.pool, at)), 1)
        }
    };
    Ok(Cow::Owned(format!(
        "width {}; the parameter {} names {told}; inputs {:?}",
        width.0,
        parameter.0,
        inputs(&code, first, count).unwrap_or_default(),
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
