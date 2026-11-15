//! The programs the streaming cases run at seam C, each as planned and as its held twin (the same plan with every node held,
//! so none streams), and their answers with every name as its text, their feeds and a warm worker's second file.
use super::cases::compared;
use super::demands::Demand;
use super::{TableInputs, TextResources, planned, ran};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, defines_rows, over_defines, over_nodes};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeFlow, NodeRows, PlanInput, PlanInputList, PlanNodeId, Resources,
};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Offset, Relation, Span, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

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
    let mut resources = TextResources::new(texts);
    let held = tables(&resources);
    let lent: Vec<(Relation, &Table)> = held
        .iter()
        .map(|(relation, table)| (*relation, table))
        .collect();
    let inputs = TableInputs::new(&lent);
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

/// A capture of each name into the two slots of `{a}{b}`.
pub(crate) fn split_names() -> Step {
    over_defines(Operation::Capture {
        from: StepInput::Relation(Relation::Defines),
        path: ColumnIndex(3),
        pattern: CapturePattern("{a}{b}".into()),
    })
}

/// A select of every column of node `from`, kept where `comparisons` hold.
pub(crate) fn select_of(from: u32, comparisons: Vec<PlanComparison>) -> Step {
    over_nodes(
        Operation::Select {
            from: StepInput::Step(StepId(from)),
            comparisons,
            kept: Vec::new(),
        },
        &[from],
    )
}

/// Whether column `column` compares by `test` with the written text `text`.
pub(crate) fn against(test: Test, column: u32, text: &str) -> PlanComparison {
    compared(
        test,
        Operand::Column(ColumnIndex(column)),
        Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    )
}

/// The span `start..end`.
pub(crate) fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

/// Rows of the span-between stage over a filter, `rows` of them, the stage failing on row `map_fault` (its span runs
/// backwards) and the filter on row `filter_fault` (its name is no pattern).
pub(crate) fn faulting(rows: usize, map_fault: usize, filter_fault: usize) -> Program {
    Program {
        steps: vec![
            over_defines(Operation::SpanBetween {
                from: StepInput::Relation(Relation::Defines),
                start: ColumnIndex(6),
                end: ColumnIndex(6),
            }),
            select_of(0, vec![super::cases::self_matching(3)]),
        ],
        emit: PlanNodeId(1),
        rows: (0..rows)
            .map(|at| {
                (
                    if at == filter_fault { "[" } else { "a" }.to_owned(),
                    if at == map_fault {
                        span(2, 1)
                    } else {
                        span(0, 1)
                    },
                )
            })
            .collect(),
        texts: vec!["a".into(), "[".into()],
    }
}

/// An inner join of `left` with `right` on their names, answering `output`, its right rows kept where `filter` holds.
pub(crate) fn join(
    left: (StepInput, PlanInput),
    right: (StepInput, PlanInput),
    output: Vec<JoinColumn>,
    filter: Vec<PlanComparison>,
) -> Step {
    Step {
        operation: Operation::Join {
            left: left.0,
            right: right.0,
            left_on: vec![ColumnIndex(3)],
            right_on: vec![ColumnIndex(3)],
            output,
            filter,
            view: Vec::new(),
            reach: None,
        },
        inputs: vec![left.1, right.1],
    }
}

/// Every column of a join's left side, a row of Defines.
pub(crate) fn left_columns() -> Vec<JoinColumn> {
    (0..7)
        .map(|column| JoinColumn {
            side: Side::Left,
            column: ColumnIndex(column),
        })
        .collect()
}

/// The left's node and the right's node.
pub(crate) fn both_nodes() -> Vec<JoinColumn> {
    vec![
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        },
    ]
}
