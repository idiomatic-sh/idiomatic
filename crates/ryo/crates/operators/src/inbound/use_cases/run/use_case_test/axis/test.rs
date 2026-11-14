//! The axis kind through `plan` and `run` only, over keyed edges: what each direction answers from a start, in main's walk
//! order, and what the domain admits.
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, Input as StepInput};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the cases read edges from: parent, child, ordinal, field, file.
const EDGES: Relation = Relation::SemanticChild;

/// A walk: its direction, the start node, and whether the domain holds only the even nodes.
type Input = (Direction, u32, bool);
/// The start and the reached node of each answer row, in order.
type Value = std::borrow::Cow<'static, [u32]>;
type Failure = PortableOperatorFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_axis_answers_the_parents_in_edge_order"),
        word: Word::Succeeds,
        input: || (Direction::Parent, 4, false),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[4, 2, 4, 1])),
    },
    Case {
        name: CaseName("an_axis_answers_the_children_in_edge_order"),
        word: Word::Succeeds,
        input: || (Direction::Child, 1, false),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1, 3, 1, 2, 1, 4])),
    },
    Case {
        name: CaseName("an_axis_answers_the_ancestors_breadth_first"),
        word: Word::Succeeds,
        input: || (Direction::Ancestor, 6, false),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[6, 4, 6, 5, 6, 2, 6, 1, 6, 3])),
    },
    Case {
        name: CaseName("an_axis_answers_the_descendants_breadth_first"),
        word: Word::Succeeds,
        input: || (Direction::Descendant, 1, false),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1, 3, 1, 2, 1, 4, 1, 5, 1, 6])),
    },
    Case {
        name: CaseName("the_nearest_ancestor_is_the_first_the_domain_admits"),
        word: Word::Succeeds,
        input: || (Direction::NearestAncestor, 6, true),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[6, 4])),
    },
    Case {
        name: CaseName("an_axis_admits_only_what_its_domain_holds"),
        word: Word::Succeeds,
        input: || (Direction::Descendant, 1, true),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1, 2, 1, 4, 1, 6])),
    },
];

/// The node at position `at` of file 0.
fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

/// Rows of `EDGES`, parent to child, with the field `f`.
fn edges(pairs: &[(u32, u32)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(EDGES).len());
    for (parent, child) in pairs {
        cells.extend([
            node(*parent),
            node(*child),
            Cell::Number(Integer(0)),
            texts.symbol("f"),
            Cell::File(FileId(0)),
        ]);
    }
    Table {
        kinds: kinds(EDGES).to_vec(),
        cells,
        count: RowCount(pairs.len()),
    }
}

/// Rows of `relation`, each its node in every node column and a cell of each column's kind elsewhere.
fn rows_of(relation: Relation, nodes: &[(u32, u32)], texts: &TextResources) -> Table {
    use ryo_relations::domain::relations::types::{CellKind, Flag, Offset, Span};
    let columns = kinds(relation);
    let mut cells = Vec::with_capacity(nodes.len() * columns.len());
    for (at, _) in nodes {
        cells.extend(columns.iter().map(|kind| match kind {
            CellKind::Node => node(*at),
            CellKind::Symbol => texts.symbol("f"),
            CellKind::File => Cell::File(FileId(0)),
            CellKind::Span => Cell::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            CellKind::Number => Cell::Number(Integer(0)),
            CellKind::Flag => Cell::Flag(Flag::Lowered),
        }));
    }
    Table {
        kinds: columns.to_vec(),
        cells,
        count: RowCount(nodes.len()),
    }
}

/// Plans and runs the axis from `start` over a graph with two parents for node 4 (1 and 2), and answers each row's start
/// and reached node.
fn subject((direction, start, even): Input) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["f"]);
    // 1 -> 3, 1 -> 2, 2 -> 4, 1 -> 4, 3 -> 5, 4 -> 6, 5 -> 6.
    let graph = edges(
        &[(1, 3), (1, 2), (2, 4), (1, 4), (3, 5), (4, 6), (5, 6)],
        &resources,
    );
    let domain_nodes: Vec<(u32, u32)> = (1..=6)
        .filter(|at| !even || at % 2 == 0)
        .map(|at| (at, at))
        .collect();
    let domain = rows_of(Relation::Function, &domain_nodes, &resources);
    let starts = rows_of(Relation::Defines, &[(start, start)], &resources);
    let inputs = TableInputs::new(&[
        (EDGES, &graph),
        (Relation::Defines, &starts),
        (Relation::Function, &domain),
    ]);
    let step = Step {
        operation: Operation::Axis {
            from: StepInput::Relation(Relation::Defines),
            edges: StepInput::Relation(EDGES),
            domain: StepInput::Relation(Relation::Function),
            direction,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output: Vec::new(),
        },
        inputs: vec![
            PlanInput::Relation(Relation::Defines),
            PlanInput::Relation(EDGES),
            PlanInput::Relation(Relation::Function),
        ],
    };
    let rows = emitted(
        &[step],
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    let position = |cell: &Cell| match cell {
        Cell::Node(id) => ryo_relations::domain::relations::service::node_position(*id).0,
        _ => u32::MAX,
    };
    Ok(rows
        .iter()
        .flat_map(|table| {
            let width = table.kinds.len().max(1);
            table
                .cells
                .chunks(width)
                .flat_map(move |row| [position(&row[0]), position(&row[width - 1])])
        })
        .collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
