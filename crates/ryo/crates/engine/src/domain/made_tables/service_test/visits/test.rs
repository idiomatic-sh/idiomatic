//! The cases of the `Graph(OrderedDfs)` fill (`ordered-dfs`), each run through `made` on a pack the contract's encoding
//! wrote: each distinct key and root walked once, depth first, a source's edges in their order; a row is the root's key,
//! the root, the edge's source, destination and identity, and the ordinal of the visit among its root's.

use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::code::records::types::VisitColumns;
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{ConstantRows, FillInput, GraphFill, MadeFill, MadeTable};
use crate::domain::made_tables::fixtures::packs::{of, operands, pipeline_of, table};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{
    ColumnList, MadeColumn, MadeTableId, MatchColumns, ReadColumn, Register, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use std::borrow::Cow;

/// The walk a case makes.
#[derive(Clone, Copy, Debug)]
enum Walked {
    /// The contract's stated edges (source, destination, order) (1, 3, 2), (1, 2, 1), (2, 3, 0), (3, 1, 0) of key 7,
    /// root 1 given twice; root 9 of key 7 leaves no edge; root 1 of key 8 has no edge of its key.
    Stated,
    /// Two edges of one key and source with one order.
    Duplicate,
}

type Input = Walked;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_distinct_root_is_walked_once_depth_first_in_its_edges_order"),
        word: Word::Succeeds,
        input: || Walked::Stated,
        expect: Expect::Value(Cow::Borrowed(
            "7 1 1 2 1 0; 7 1 1 3 2 3; 7 1 2 3 0 1; 7 1 3 1 0 2 | read 48 given 36 made 4",
        )),
    },
    Case {
        name: CaseName("two_edges_of_one_source_and_order_are_the_meanings_fault"),
        word: Word::Refuses,
        input: || Walked::Duplicate,
        expect: Expect::Refusal(|fault| {
            if matches!(fault, FilledError::Meaning { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// A constant table of the rows, and the pipeline that reads every column of it, keyed by the first column.
fn constant_input(
    number: u32,
    node: u8,
    rows: &[&[u64]],
    order: Option<MadeColumn>,
) -> (MadeTable, FillInput) {
    let width = rows.first().map_or(0, |row| row.len());
    let held = table(
        node,
        width,
        MadeFill::Constant(ConstantRows(
            rows.iter()
                .map(|row| row.iter().map(|cell| CellWord(*cell)).collect())
                .collect(),
        )),
    );
    let base = number.saturating_mul(10).saturating_add(20);
    let mut stages = vec![FillStage::Scan {
        source: StepRows::Made(MadeTableId(number)),
        columns: MatchColumns::default(),
        values: operands([]),
        out: Register(base),
    }];
    let mut row = Vec::new();
    for column in 0..u16::try_from(width).unwrap_or(0) {
        let out = base.saturating_add(1).saturating_add(u32::from(column));
        stages.push(FillStage::Read {
            source: StepRows::Made(MadeTableId(number)),
            column: ReadColumn::Made(MadeColumn(column)),
            row: Register(base),
            out: Register(out),
        });
        row.push(of(out));
    }
    (
        held,
        FillInput {
            pipelines: Box::new([pipeline_of(stages, row)]),
            keys: ColumnList(Box::new([MadeColumn(0)])),
            order,
        },
    )
}

/// The roots (key, root) and the edges (key, source, destination, order) of a case, and their walk: the edge's
/// identity is its order.
fn tables(walked: Walked) -> Vec<MadeTable> {
    let roots: &[&[u64]] = &[&[7, 1], &[7, 9], &[8, 1], &[7, 1]];
    let edges: &[&[u64]] = match walked {
        Walked::Stated => &[&[7, 1, 3, 2], &[7, 1, 2, 1], &[7, 2, 3, 0], &[7, 3, 1, 0]],
        Walked::Duplicate => &[&[7, 1, 3, 2], &[7, 1, 2, 2]],
    };
    let (root_table, root_input) = constant_input(0, 2, roots, None);
    let (edge_table, edge_input) = constant_input(1, 3, edges, Some(MadeColumn(3)));
    let columns = VisitColumns {
        root: MadeColumn(1),
        source: MadeColumn(1),
        destination: MadeColumn(2),
        edge: MadeColumn(3),
    };
    vec![
        root_table,
        edge_table,
        table(
            1,
            6,
            MadeFill::Graph(
                GraphFill::OrderedDfs(columns),
                Box::new([root_input, edge_input]),
            ),
        ),
    ]
}

fn subject(walked: Input) -> Result<Value, Failure> {
    let made_tables = tables(walked);
    let last = MadeTableId(u32::try_from(made_tables.len().saturating_sub(1)).unwrap_or(0));
    let mut world = world_with(made_tables, Vec::new(), &[], Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _now = made(&mut frame, looped, last)?;
    let seen = shown(frame.store, last);
    let mut rows: Vec<&str> = seen.split("; ").filter(|row| !row.is_empty()).collect();
    rows.sort_unstable();
    let work = table_of(frame.store, last)
        .map(|slot| slot.work)
        .unwrap_or_default();
    Ok(Cow::Owned(format!(
        "{} | read {} given {} made {}",
        rows.join("; "),
        work.read.0,
        work.given.0,
        work.made.0
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
