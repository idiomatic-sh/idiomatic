//! The `ordered_walk` item's cases: which lowerings run the kernel, and how the kernel compares a text cell: by the symbol
//! main's engine interns its text to, so a symbol and a name of the same text are one entity.
use super::item::{OrderedWalkItem, visited};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{InputRows, PlanNodeId, RowSource};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::service::{intern_derived, name_made_across};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

/// What a case gives the part: a lowering to reach, or the kernel's text case.
#[derive(Clone, Debug)]
enum Given {
    /// A lowering, asked whether it runs the kernel.
    Lowered(Lowering),
    /// A root that is a made symbol, and one edge from the name of that symbol to itself.
    TextAndItsName,
}

/// What the part answered.
#[derive(Debug, PartialEq, Eq)]
enum Answered {
    /// Whether the lowering runs the kernel.
    Runs(Reach),
    /// How many visits the walk answered.
    Visits(usize),
}

/// A lowering, or the kernel's text case.
type Input = Given;
/// Whether the node runs the kernel, or how many visits it answered.
type Value = Answered;
type Failure = PortableOperatorFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Given::Lowered(Lowering::Distinct {
                sources: [
                    RowSource::Node(PlanNodeId(0)),
                    RowSource::Node(PlanNodeId(1)),
                ],
            })
        },
        expect: Expect::Value(Answered::Runs(Reach::Skips)),
    },
    Case {
        name: CaseName("a_symbol_and_a_name_of_its_text_are_one_entity"),
        word: Word::Succeeds,
        input: || Given::TextAndItsName,
        expect: Expect::Value(Answered::Visits(1)),
    },
];

/// The walk from a root that is a made symbol over one edge whose source and destination are the name of that symbol.
fn text_and_its_name() -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("made"));
    let name = name_made_across(&symbols, symbol).ok_or(PortableOperatorFault::WrongOperation)?;
    let texts = Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(b""),
        paths: &[],
        arenas: &[],
    };
    let roots = Table {
        kinds: vec![CellKind::Symbol],
        cells: vec![Cell::Symbol(symbol)],
        count: RowCount(1),
    };
    let edges = Table {
        kinds: vec![
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number,
            CellKind::Number,
        ],
        cells: vec![
            Cell::Name(name),
            Cell::Name(name),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(7)),
        ],
        count: RowCount(1),
    };
    let dfs =
        crate::domain::ordered_dfs::service::lower(crate::domain::ordered_dfs::types::DfsShape {
            root: ColumnIndex(0),
            edge_columns: [
                ColumnIndex(0),
                ColumnIndex(1),
                ColumnIndex(2),
                ColumnIndex(3),
            ],
            root_keys: &[],
            edge_keys: &[],
        });
    let mut output = Table::default();
    let _fault = visited(
        &dfs,
        InputRows::Table(&roots),
        InputRows::Table(&edges),
        &texts,
        &mut Default::default(),
        &mut output,
    )?;
    Ok(Answered::Visits(output.count.0))
}

/// Calls the part on one input; the author states it.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Lowered(lowering) => Ok(Answered::Runs(OrderedWalkItem.reached_by(&lowering))),
        Given::TextAndItsName => text_and_its_name(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
