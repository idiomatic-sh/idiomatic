//! The `queued_walk` item's cases: which lowerings run the kernel, and how the kernel compares a text state: by the symbol its
//! text interns to, so a symbol and a name of the same text are one state.
use super::item::{QueuedWalkItem, WorklistInputs, worklisted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{DistinctNode, Lowering, WorklistNode};
use crate::domain::physical_plan::types::{InputRows, PlanNodeId, RowSource};
use crate::domain::table_texts::types::Texts;
use crate::domain::worklist::service::lower;
use crate::domain::worklist::types::WorklistShape;
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
    /// A root whose state is a made symbol, a lattice and an edge naming that state by its name.
    TextStateAndItsName,
}

/// What the part answered.
#[derive(Debug, PartialEq, Eq)]
enum Answered {
    /// Whether the lowering runs the kernel.
    Runs(Reach),
    /// How many rows the worklist answered.
    Rows(usize),
}

type Input = Given;
type Value = Answered;
type Failure = PortableOperatorFault;

/// The columns every case's worklist reads: roots (root, state), edges (source, destination, order, before, after), lattice
/// (lower, upper).
fn shape() -> WorklistShape<'static> {
    WorklistShape {
        root: ColumnIndex(0),
        state: ColumnIndex(1),
        edge_columns: [
            ColumnIndex(0),
            ColumnIndex(1),
            ColumnIndex(2),
            ColumnIndex(3),
            ColumnIndex(4),
        ],
        lattice_columns: [ColumnIndex(0), ColumnIndex(1)],
        root_keys: &[],
        edge_keys: &[],
    }
}

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Given::Lowered(Lowering::Distinct(DistinctNode {
                sources: [
                    RowSource::Node(PlanNodeId(0)),
                    RowSource::Node(PlanNodeId(1)),
                ],
            }))
        },
        expect: Expect::Value(Answered::Runs(Reach::Skips)),
    },
    Case {
        name: CaseName("only_a_worklist_lowering_runs_the_queued_walk_kernel"),
        word: Word::Succeeds,
        input: || {
            Given::Lowered(Lowering::Worklist(WorklistNode {
                roots: RowSource::Node(PlanNodeId(0)),
                edges: RowSource::Node(PlanNodeId(1)),
                lattice: RowSource::Node(PlanNodeId(2)),
                worklist: lower(shape()),
            }))
        },
        expect: Expect::Value(Answered::Runs(Reach::Runs)),
    },
    Case {
        name: CaseName("a_text_state_and_its_name_are_one_state"),
        word: Word::Succeeds,
        input: || Given::TextStateAndItsName,
        expect: Expect::Value(Answered::Rows(1)),
    },
];

/// A root of state `made`, a made symbol; the lattice row `least < made` naming it by its name; and an edge of the root to
/// itself in state `made`, by its name, joining the least state.
fn text_state_and_its_name() -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let least = intern_derived(&mut symbols, ByteSlice(b""), NameText("least"));
    let made = intern_derived(&mut symbols, ByteSlice(b""), NameText("made"));
    let name = name_made_across(&symbols, made).ok_or(PortableOperatorFault::WrongOperation)?;
    let texts = Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(b""),
        paths: &[],
        arenas: &[],
    };
    let number = |value| Cell::Number(Integer(value));
    let roots = Table {
        kinds: vec![CellKind::Number, CellKind::Symbol],
        cells: vec![number(1), Cell::Symbol(made)],
        count: RowCount(1),
    };
    let edges = Table {
        kinds: vec![
            CellKind::Number,
            CellKind::Number,
            CellKind::Number,
            CellKind::Symbol,
            CellKind::Symbol,
        ],
        cells: vec![
            number(1),
            number(1),
            number(0),
            Cell::Name(name),
            Cell::Symbol(least),
        ],
        count: RowCount(1),
    };
    let lattice = Table {
        kinds: vec![CellKind::Symbol, CellKind::Symbol],
        cells: vec![Cell::Symbol(least), Cell::Name(name)],
        count: RowCount(1),
    };
    let mut output = Table::default();
    let fault = worklisted(
        &lower(shape()),
        WorklistInputs {
            roots: InputRows::Table(&roots),
            edges: InputRows::Table(&edges),
            lattice: InputRows::Table(&lattice),
        },
        &texts,
        &mut Default::default(),
        &mut Default::default(),
        &mut output,
    )?;
    // A row's fault is the kernel's answer too: the case fails on one, as on a fault of the node.
    if let Some(fault) = fault {
        return Err(fault);
    }
    Ok(Answered::Rows(output.count.0))
}

/// Calls the part on one input; the author states it.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Lowered(lowering) => Ok(Answered::Runs(QueuedWalkItem.reached_by(&lowering))),
        Given::TextStateAndItsName => text_state_and_its_name(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for WorklistNode {
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        vec![Witnessed {
            node: Self {
                roots: RowSource::Node(PlanNodeId(0)),
                edges: RowSource::Node(PlanNodeId(1)),
                lattice: RowSource::Node(PlanNodeId(2)),
                worklist: lower(WorklistShape {
                    root: columns.id,
                    state: columns.state,
                    edge_columns: [
                        columns.parent,
                        columns.child,
                        columns.order,
                        columns.state,
                        columns.state,
                    ],
                    lattice_columns: [columns.parent, columns.child],
                    root_keys: &[],
                    edge_keys: &[],
                }),
            },
            roles: vec![InputRole::Rows, InputRole::Edges, InputRole::Domain],
        }]
    }
}
