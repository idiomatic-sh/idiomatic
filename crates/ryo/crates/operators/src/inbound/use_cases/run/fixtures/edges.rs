//! The edge rows the graph kinds' run cases read, and their answers as the cases state them; and the graphs the worklist's
//! run cases read: root, edge and lattice rows the host lends, entities and partitions as numbers, states as texts.
use super::{TableInputs, TextResources, emitted};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use crate::inbound::use_cases::run::fixtures::cases::self_matching;
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Position, Relation, Value as Cell,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, HostedNumber, Input as StepInput, StepId,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The relation the cases read edges from: parent, child, ordinal, field, file.
pub(crate) const EDGES: Relation = Relation::SemanticChild;

/// The node at position `at` of file 0.
pub(crate) fn node(at: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(at)))
}

/// The edge rows `parent -> child`, each with its ordinal, the field `f` and file 0.
pub(crate) fn edges(pairs: &[(u32, u32, i64)], texts: &TextResources) -> Table {
    let mut cells = Vec::with_capacity(pairs.len() * kinds(EDGES).len());
    for (parent, child, ordinal) in pairs {
        cells.extend([
            node(*parent),
            node(*child),
            Cell::Number(Integer(*ordinal)),
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

/// A cell as the case states it: a node by its position, a number as itself.
pub(crate) fn stated(cell: Cell) -> i64 {
    if let Cell::Node(id) = cell {
        i64::from(ryo_relations::domain::relations::service::node_position(id).0)
    } else if let Cell::Number(Integer(number)) = cell {
        number
    } else {
        panic!("a case states nodes and numbers only, not {cell:?}")
    }
}

/// The rows `step` answers over `pairs` as the edges, each cell as the case states it.
///
/// # Errors
///
/// The fault `plan` or `run` answers.
pub(crate) fn edge_rows(
    step: Step,
    pairs: &[(u32, u32, i64)],
) -> Result<Vec<i64>, PortableOperatorFault> {
    let mut resources = TextResources::new(&["f"]);
    let table = edges(pairs, &resources);
    let inputs = TableInputs::new(&[(EDGES, &table)]);
    let rows = emitted(
        &[step],
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    Ok(rows
        .iter()
        .flat_map(|table| table.cells.iter().copied().map(stated))
        .collect())
}

/// A root row: its partition, the root, and a state it starts in.
pub(crate) type RootRow = (i64, i64, &'static str);

/// An edge row: its partition, source, destination, order, before-state and after-state.
pub(crate) type EdgeRow = (i64, i64, i64, i64, &'static str, &'static str);

/// A lattice row: a lower state and an upper state above it.
pub(crate) type LatticeRow = (&'static str, &'static str);

/// constructor-super's five lattice rows: `unreached` least, `never` and `once` above it, `mixed` above both, `multiple` top.
pub(crate) const CALLS: &[LatticeRow] = &[
    ("unreached", "never"),
    ("unreached", "once"),
    ("never", "mixed"),
    ("once", "mixed"),
    ("mixed", "multiple"),
];

/// A chain of three states: `u` least, then `a`, then `b`.
pub(crate) const CHAIN: &[LatticeRow] = &[("u", "a"), ("a", "b")];

/// The rows a worklist runs over, and whether it keys its runs by the partition column.
#[derive(Clone, Debug, Default)]
pub(crate) struct Graph {
    /// The lattice rows.
    pub(crate) lattice: Vec<LatticeRow>,
    /// The root rows.
    pub(crate) roots: Vec<RootRow>,
    /// The edge rows.
    pub(crate) edges: Vec<EdgeRow>,
    /// Whether the runs are keyed by the partition column.
    pub(crate) keyed: bool,
}

/// The edges of a source of constructor-super's transfer: one to each of `successors` in order for every state but
/// `unreached`, the state passed on raised by the source's call when it has one.
pub(crate) fn passed(source: i64, successors: &[i64], call: bool) -> Vec<EdgeRow> {
    let mut rows = Vec::new();
    for (order, destination) in (0_i64..).zip(successors) {
        for before in ["never", "once", "mixed", "multiple"] {
            let after = match (call, before) {
                (false, _) => before,
                (true, "never") => "once",
                (true, _) => "multiple",
            };
            rows.push((0, source, *destination, order, before, after));
        }
    }
    rows
}

/// An edge of `entity` to itself in each of `states`, joining the least state: it changes nothing, and records each pop.
pub(crate) fn observed(entity: i64, states: &[&'static str], least: &'static str) -> Vec<EdgeRow> {
    states
        .iter()
        .map(|state| (0, entity, entity, 0, *state, least))
        .collect()
}

/// A graph over the chain lattice of `roots` and `edges`, unkeyed.
pub(crate) fn chain(
    roots: &[(i64, &'static str)],
    edges: &[(i64, i64, i64, &'static str, &'static str)],
) -> Graph {
    Graph {
        lattice: CHAIN.to_vec(),
        roots: roots
            .iter()
            .map(|(root, state)| (0, *root, *state))
            .collect(),
        edges: edges
            .iter()
            .map(|(source, destination, order, before, after)| {
                (0, *source, *destination, *order, *before, *after)
            })
            .collect(),
        keyed: false,
    }
}

/// Every text the graph names, each once.
fn texts(graph: &Graph) -> Vec<&'static str> {
    let mut texts: Vec<&'static str> = graph
        .lattice
        .iter()
        .flat_map(|(lower, upper)| [*lower, *upper])
        .chain(graph.roots.iter().map(|root| root.2))
        .chain(graph.edges.iter().flat_map(|edge| [edge.4, edge.5]))
        .collect();
    texts.sort_unstable();
    texts.dedup();
    texts
}

/// A table of `kinds` holding `rows`.
fn table(kinds: &[CellKind], rows: impl Iterator<Item = Vec<Cell>>) -> Table {
    let mut table = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for row in rows {
        table.cells.extend(row);
        table.count = RowCount(table.count.0 + 1);
    }
    table
}

/// A number cell.
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

/// The host's root, edge and lattice tables of `graph`, in that order.
fn hosted(graph: &Graph, resources: &TextResources) -> Vec<Table> {
    use CellKind::{Number, Symbol};
    vec![
        table(
            &[Number, Number, Symbol],
            graph.roots.iter().map(|(key, root, state)| {
                vec![number(*key), number(*root), resources.symbol(state)]
            }),
        ),
        table(
            &[Number, Number, Number, Number, Symbol, Symbol],
            graph
                .edges
                .iter()
                .map(|(key, source, destination, order, before, after)| {
                    vec![
                        number(*key),
                        number(*source),
                        number(*destination),
                        number(*order),
                        resources.symbol(before),
                        resources.symbol(after),
                    ]
                }),
        ),
        table(
            &[Symbol, Symbol],
            graph
                .lattice
                .iter()
                .map(|(lower, upper)| vec![resources.symbol(lower), resources.symbol(upper)]),
        ),
    ]
}

/// The step reading the host's relation `place`.
fn host(place: u32) -> Step {
    Step {
        operation: Operation::Hosted {
            relation: HostedNumber(place),
        },
        inputs: Vec::new(),
    }
}

/// A select of node `node`'s rows where column `column` matches itself as a pattern.
fn guarded(node: u32, column: u32) -> Step {
    Step {
        operation: Operation::Select {
            from: StepInput::Step(StepId(node)),
            comparisons: vec![self_matching(column)],
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Node(PlanNodeId(node))],
    }
}

/// The worklist over nodes `roots`, `edges` and `lattice`, keyed by the partition column when `keyed`.
fn worklist([roots, edges, lattice]: [u32; 3], keyed: bool) -> Step {
    let keys = if keyed {
        vec![ColumnIndex(0)]
    } else {
        Vec::new()
    };
    Step {
        operation: Operation::Worklist {
            from: StepInput::Step(StepId(roots)),
            edges: StepInput::Step(StepId(edges)),
            lattice: StepInput::Step(StepId(lattice)),
            root: ColumnIndex(1),
            state: ColumnIndex(2),
            source: ColumnIndex(1),
            destination: ColumnIndex(2),
            order: ColumnIndex(3),
            before: ColumnIndex(4),
            after: ColumnIndex(5),
            lower: ColumnIndex(0),
            upper: ColumnIndex(1),
            root_keys: keys.clone(),
            edge_keys: keys,
        },
        inputs: [roots, edges, lattice]
            .map(|node| PlanInput::Node(PlanNodeId(node)))
            .to_vec(),
    }
}

/// A cell as the case states it: a number as itself, a symbol as its text.
fn stated_text(cell: Cell, texts: &[&str], resources: &TextResources) -> String {
    if let Cell::Number(Integer(value)) = cell {
        return value.to_string();
    }
    texts
        .iter()
        .find(|text| resources.symbol(text) == cell)
        .map_or_else(|| format!("{cell:?}"), |text| (*text).to_owned())
}

/// The trace a worklist answers over `graph`, each row `root:source>destination#order before=after`, its partition and a `/`
/// first when keyed, rows apart by a space.
///
/// # Errors
///
/// The fault `plan` or `run` answers.
pub(crate) fn traced(graph: &Graph) -> Result<String, PortableOperatorFault> {
    let texts = texts(graph);
    let mut resources = TextResources::new(&texts);
    let inputs = TableInputs::new(&[]).with_hosted(hosted(graph, &resources));
    let steps = [host(0), host(1), host(2), worklist([0, 1, 2], graph.keyed)];
    let rows = emitted(
        &steps,
        &[PlanNodeId(3)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )?;
    let width = if graph.keyed { 7 } else { 6 };
    let mut trace = Vec::new();
    for table in &rows {
        for row in table.cells.chunks(width) {
            let cells: Vec<String> = row
                .iter()
                .map(|cell| stated_text(*cell, &texts, &resources))
                .collect();
            let [.., root, source, destination, order, before, after] = cells.as_slice() else {
                return Err(PortableOperatorFault::ShapeMismatch);
            };
            let key = if graph.keyed {
                cells
                    .first()
                    .map_or_else(String::new, |key| format!("{key}/"))
            } else {
                String::new()
            };
            trace.push(format!(
                "{key}{root}:{source}>{destination}#{order} {before}={after}"
            ));
        }
    }
    Ok(trace.join(" "))
}

/// The fault a worklist answers when each of its inputs named in `faulty` (of `roots`, `edges` and `lattice`) holds a row
/// whose state is a text no pattern is: the roots `[`, the edges `(`, the lattice `*`.
fn input_fault(faulty: [bool; 3]) -> Result<(), PortableOperatorFault> {
    let graph = Graph {
        lattice: vec![("u", "a"), if faulty[2] { ("*", "*") } else { ("a", "a") }],
        roots: vec![(0, 1, if faulty[0] { "[" } else { "a" })],
        edges: vec![(0, 1, 1, 0, if faulty[1] { "(" } else { "a" }, "a")],
        keyed: false,
    };
    let texts = texts(&graph);
    let mut resources = TextResources::new(&texts);
    let inputs = TableInputs::new(&[]).with_hosted(hosted(&graph, &resources));
    let steps = [
        host(0),
        host(1),
        host(2),
        guarded(0, 2),
        guarded(1, 4),
        guarded(2, 0),
        worklist([3, 4, 5], false),
    ];
    emitted(
        &steps,
        &[PlanNodeId(6)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    )
    .map(|_| ())
}

/// Which input's fault a worklist answers: first with all three inputs faulty, then with the lattice and the roots; each
/// named by the one input whose fault alone it equals, `none` when it answered no fault, `alike` when two faults alone are
/// one fault.
pub(crate) fn first_faults() -> String {
    let alone = [
        ("roots", input_fault([true, false, false])),
        ("edges", input_fault([false, true, false])),
        ("lattice", input_fault([false, false, true])),
    ];
    let distinct = alone.iter().all(|(name, fault)| {
        fault.is_err()
            && alone
                .iter()
                .all(|(other, against)| name == other || fault != against)
    });
    if !distinct {
        return "alike".to_owned();
    }
    let named = |answer: Result<(), PortableOperatorFault>| match answer {
        Ok(()) => "none",
        Err(fault) => alone
            .iter()
            .find(|(_, held)| held.as_ref().err() == Some(&fault))
            .map_or("another", |(name, _)| *name),
    };
    format!(
        "{} {}",
        named(input_fault([true, true, true])),
        named(input_fault([true, false, true]))
    )
}
