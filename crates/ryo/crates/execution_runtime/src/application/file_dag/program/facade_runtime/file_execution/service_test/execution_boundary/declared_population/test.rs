//! Declared populations are refused or kept exactly, never silently shortened.
use super::super::super::scratch_owner::fixtures::{execute, hosted};
use super::super::fixtures::source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{NodeId, Relation, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::{Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
    rows::types::{RowCount, Table},
};
/// A declared population and the path it is read through.
#[derive(Clone, Copy, Debug)]
enum Population {
    /// A syntax-tree table of `nodes` node cells declaring `count` rows, read as a source.
    SyntaxTree { nodes: u64, count: usize },
    /// `rows` zero-width hosted rows, projected to no columns.
    ZeroWidth { rows: usize },
}

/// The table the read answers: its row count and its cell count.
#[derive(Debug, PartialEq)]
struct Kept {
    count: usize,
    cells: usize,
}

type Input = Population;
type Value = Kept;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("malformed_declared_population_is_refused_not_silently_shortened"),
        word: Word::Refuses,
        input: || Population::SyntaxTree { nodes: 1, count: 2 },
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("zero_width_bags_keep_their_declared_population"),
        word: Word::Succeeds,
        input: || Population::ZeroWidth { rows: 3 },
        expect: Expect::Value(Kept { count: 3, cells: 0 }),
    },
];

/// Reads the population through its path and answers the table's shape.
fn subject(population: Input) -> Result<Value, Failure> {
    let output = match population {
        Population::SyntaxTree { nodes, count } => {
            let table = Table {
                kinds: kinds(Relation::SyntaxTree).to_vec(),
                cells: (0..nodes).map(|node| Cell::Node(NodeId(node))).collect(),
                count: RowCount(count),
            };
            source(Rows::Computed(&table))?
        }
        Population::ZeroWidth { rows } => {
            execute(
                Operation::Project {
                    from: PlanInput::Step(StepId(0)),
                    columns: vec![],
                    distinct: RepeatedRows::Kept,
                },
                hosted(&[], vec![vec![]; rows]),
                &mut Symbols::default(),
            )
            .0?
        }
    };
    Ok(Kept {
        count: output.count.0,
        cells: output.cells.len(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
