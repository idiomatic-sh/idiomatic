//! The runs of the streaming cases' programs at seam C, each as planned and as its held twin (the same plan with every node
//! held, so none streams), and their answers with every name as its text, their feeds and a warm worker's second file.
#![cfg(test)]
use super::demands::Demand;
use super::stream_programs::join;
use super::{TableInputs, TextResources, planned, ran};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, defines_rows};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeFlow, NodeRows, PlanInput, PlanInputList, PlanNodeId, Resources,
};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{
    FileId, Integer, NodeId, Offset, Position, Relation, Span, Value as Cell,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// A program over Defines rows: its steps, the one it emits, and the rows and texts of the file.
pub(crate) struct Program {
    /// The steps, in node order.
    pub(crate) steps: Vec<Step>,
    /// The node the program emits.
    pub(crate) emit: PlanNodeId,
    /// The Defines rows: each name, its kind and refinement, and the row's span.
    pub(crate) rows: Vec<(String, Span)>,
    /// The file's texts, each a name of it.
    pub(crate) texts: Vec<String>,
}

/// The run of a program: its answer, the nodes holding rows after it, and the buffers it ran with.
pub(crate) struct Ran {
    /// The emit's rows, each name as its text, or the first fault.
    pub(crate) answer: Result<Vec<Vec<String>>, PortableOperatorFault>,
    /// The rows each node was fed, by node, for the emit and every node that holds rows after the run; none for another.
    pub(crate) fed: Vec<Option<[usize; 3]>>,
    /// Whether the same worker, warm from the run, answers the next file of the same rows alike, rows and feeds.
    pub(crate) warm: bool,
    /// What the first file asked of the inputs, in order.
    pub(crate) demands: Vec<Demand>,
    /// The file's outcome as the engine reports it: the first relation the inputs refused, which the engine records and
    /// gives precedence, or else the emit's rows or first fault.
    pub(crate) outcome: Result<Vec<Vec<String>>, Outcome>,
    /// How many node tables the run formed.
    pub(crate) formed: u32,
    /// How many times the first file's chains called a stage.
    pub(crate) stage_calls: usize,
    /// The rows the emit holds after the first file: its rows before its fault, when it faulted.
    pub(crate) held: usize,
    /// The worker's buffers after the run.
    pub(crate) buffers: FileBuffers,
}

/// A cell as its text: a name by its text, anything else as it is.
pub(crate) fn shown(value: Cell, resources: &TextResources) -> String {
    let texts = resources.texts();
    match value {
        Cell::Symbol(symbol) => text_of(texts.symbols, ByteSlice(texts.source.0), symbol)
            .map_or_else(|| format!("{value:?}"), |text| text.0.to_owned()),
        Cell::Node(_)
        | Cell::Name(_)
        | Cell::File(_)
        | Cell::Span(_)
        | Cell::Number(_)
        | Cell::Flag(_) => format!("{value:?}"),
    }
}

/// Runs `program`, as planned or, with `twin`, as its held twin.
pub(crate) fn ran_program(program: &Program, twin: bool) -> Result<Ran, PortableOperatorFault> {
    ran_after(program, twin, &[])
}

/// Runs `program` as `ran_program` does, the engine running each of `first` before the emit.
pub(crate) fn ran_after(
    program: &Program,
    twin: bool,
    first: &[PlanNodeId],
) -> Result<Ran, PortableOperatorFault> {
    let texts: Vec<&str> = program.texts.iter().map(String::as_str).collect();
    let tables = |resources: &TextResources| {
        let named: Vec<(Cell, Span)> = program
            .rows
            .iter()
            .map(|(name, span)| (resources.symbol(name), *span))
            .collect();
        vec![(Relation::Defines, defines_rows(&named))]
    };
    ran_steps(
        &Steps {
            steps: &program.steps,
            emit: program.emit,
            first,
        },
        twin,
        &texts,
        &tables,
    )
}

/// What a run runs: the steps, the node it emits, and the nodes the engine runs before it.
pub(crate) struct Steps<'a> {
    /// The steps, in node order.
    pub(crate) steps: &'a [Step],
    /// The node the program emits.
    pub(crate) emit: PlanNodeId,
    /// The nodes the engine runs first, in order.
    pub(crate) first: &'a [PlanNodeId],
}

/// Runs `run.steps` over the relations `tables` makes from the file of `texts`, emitting `run.emit` after running
/// `run.first`; with `twin`, the same plan with every node held, so none streams.
pub(crate) fn ran_steps(
    run: &Steps<'_>,
    twin: bool,
    texts: &[&str],
    tables: &dyn Fn(&TextResources) -> Vec<(Relation, Table)>,
) -> Result<Ran, PortableOperatorFault> {
    ran_in(run, twin, TextResources::new(texts), tables, Vec::new())
}

/// Runs `run.steps` as `ran_steps` does, over the file of `resources` (its texts and its tree) and the relations `tables`
/// makes from it, the engine naming `roots` as every node's tree nodes.
pub(crate) fn ran_in(
    run: &Steps<'_>,
    twin: bool,
    mut resources: TextResources,
    tables: &dyn Fn(&TextResources) -> Vec<(Relation, Table)>,
    roots: Vec<NodeId>,
) -> Result<Ran, PortableOperatorFault> {
    let held = tables(&resources);
    let lent: Vec<(Relation, &Table)> = held
        .iter()
        .map(|(relation, table)| (*relation, table))
        .collect();
    let inputs = TableInputs::new(&lent).with_roots(roots);
    let mut logical = LogicalProgram::new();
    for step in run.steps {
        let _id = logical.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    logical.emit(RuleOrdinal(0), run.emit);
    let mut plan = planned(&logical)?;
    // The held twin is the same plan, the same rewrites and indexes, with no node streamed: what the engine ran before
    // the chain of streamed stages. Holding the nodes as engine uses instead would change what the rewrites may do.
    if twin {
        for node in &mut plan.nodes {
            node.flow = NodeFlow::Held;
        }
    }
    let mut buffers = FileBuffers::default();
    let file = |buffers: &mut FileBuffers,
                resources: &mut TextResources|
     -> Result<_, PortableOperatorFault> {
        for node in run.first {
            ran(&plan, *node, buffers, &inputs, resources)?;
        }
        let answer = ran(&plan, run.emit, buffers, &inputs, resources).and_then(|()| match buffers
            .rows(run.emit)
        {
            NodeRows::Materialized(table) => Ok(table
                .cells
                .chunks(table.kinds.len().max(1))
                .take(table.count.0)
                .map(|row| row.iter().map(|cell| shown(*cell, resources)).collect())
                .collect()),
            NodeRows::Absent => Err(PortableOperatorFault::NotRun),
        });
        let fed: Vec<Option<[usize; 3]>> = (0..u32::try_from(run.steps.len()).unwrap_or(u32::MAX))
            .map(PlanNodeId)
            .map(|node| {
                (node == run.emit || buffers.rows(node) != NodeRows::Absent)
                    .then(|| buffers.fed(node).0.map(|rows| rows.0))
            })
            .collect();
        Ok((answer, fed))
    };
    let (answer, fed) = file(&mut buffers, &mut resources)?;
    let demands = inputs.after.demands.trace();
    let outcome = match demands.iter().find_map(|demand| match demand {
        Demand::Refused(relation) => Some(*relation),
        Demand::Lent(_) | Demand::Frontier | Demand::Observation => None,
    }) {
        Some(relation) => Err(Outcome::Refused(relation)),
        None => answer.clone().map_err(Outcome::Fault),
    };
    let formed = buffers.meter.formed.0;
    let stage_calls = buffers.chains.calls.0;
    let held = match buffers.rows(run.emit) {
        NodeRows::Materialized(table) => table.count.0,
        NodeRows::Absent => 0,
    };
    // The file ends: the engine releases every node, and the warm worker runs the next file of the same rows.
    for node in (0..u32::try_from(run.steps.len()).unwrap_or(u32::MAX)).map(PlanNodeId) {
        buffers.release(node);
    }
    let (again, fed_again) = file(&mut buffers, &mut resources)?;
    Ok(Ran {
        warm: again == answer && fed_again == fed,
        demands,
        outcome,
        answer,
        fed,
        formed,
        stage_calls,
        held,
        buffers,
    })
}

/// A file's outcome when it is no rows: the relation the inputs refused, or the operators' first fault.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Outcome {
    /// The inputs refused this relation.
    Refused(Relation),
    /// The operators' first fault.
    Fault(PortableOperatorFault),
}

/// Whether a planned run agrees with its held twin: the same inputs asked in the same order, the same engine outcome, the
/// same rows in order or the same first fault, the same rows fed to
/// every node both hold (the emit always), and each warm worker answering its next file alike.
pub(crate) fn agrees(planned: &Ran, twin: &Ran) -> bool {
    planned.answer == twin.answer
        && planned.demands == twin.demands
        && planned.outcome == twin.outcome
        && planned.warm
        && twin.warm
        && planned.fed.iter().zip(&twin.fed).all(|pair| match pair {
            (Some(planned), Some(twin)) => planned == twin,
            (None, _) | (_, None) => true,
        })
}

/// The lines of the wide tree the tree cases read: 1,700 lines of a node and two tokens, 5,101 nodes with the root.
pub(crate) const TREE_LINES: u32 = 1_700;

/// Runs `steps`, emitting `emit`, over the wide tree of `TREE_LINES` lines and the relations `tables` makes from it (given the
/// tree's nodes), the engine naming every node of the tree as each node's tree nodes; with `twin`, as its held twin.
pub(crate) fn ran_over_tree(
    steps: &[Step],
    emit: PlanNodeId,
    twin: bool,
    tables: &dyn Fn(&TextResources, &[NodeId]) -> Vec<(Relation, Table)>,
) -> Result<Ran, PortableOperatorFault> {
    let nodes: Vec<NodeId> = (0..1 + TREE_LINES * 3)
        .map(|at| node_id(FileId(0), Position(at)))
        .collect();
    ran_in(
        &Steps {
            steps,
            emit,
            first: &[],
        },
        twin,
        TextResources::wide_tree(TREE_LINES),
        &|resources| tables(resources, &nodes),
        nodes.clone(),
    )
}

/// Runs `steps`, emitting `emit`, over one Defines row and, in place of `short`'s rows, a view of `short` that lends one
/// column fewer than its kinds: malformed facts, which the executor refuses where it reads them.
pub(crate) fn ran_over_short_view(
    steps: &[Step],
    emit: PlanNodeId,
    short: Relation,
) -> Result<Ran, PortableOperatorFault> {
    let tables = |resources: &TextResources| {
        let full = defines_rows(&[(
            resources.symbol("a"),
            Span {
                start: Offset(0),
                end: Offset(1),
            },
        )]);
        let width = kinds(short).len().saturating_sub(1);
        let short_view = Table {
            kinds: kinds(short).iter().copied().take(width).collect(),
            cells: (0..width).map(|_| Cell::Number(Integer(0))).collect(),
            count: RowCount(1),
        };
        if short == Relation::Defines {
            Vec::from([(short, short_view)])
        } else {
            Vec::from([(Relation::Defines, full), (short, short_view)])
        }
    };
    ran_steps(
        &Steps {
            steps,
            emit,
            first: &[],
        },
        false,
        &["a"],
        &tables,
    )
}

/// An inner join of node 0's rows with `right` on their node (column 0), answering the left's node.
pub(crate) fn node_probe(right: Relation) -> Step {
    let right = (StepInput::Relation(right), PlanInput::Relation(right));
    let left = (StepInput::Step(StepId(0)), PlanInput::Node(PlanNodeId(0)));
    let output = vec![JoinColumn {
        side: Side::Left,
        column: ColumnIndex(0),
    }];
    let mut probe = join(left, right, output, Vec::new());
    if let Operation::Join {
        left_on, right_on, ..
    } = &mut probe.operation
    {
        (*left_on, *right_on) = (vec![ColumnIndex(0)], vec![ColumnIndex(0)]);
    }
    probe
}
