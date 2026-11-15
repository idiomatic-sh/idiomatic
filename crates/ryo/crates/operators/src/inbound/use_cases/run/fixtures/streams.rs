//! The programs the streaming cases run at seam C, each as planned and as its held twin (the same program with every node
//! before its emit held by the engine, so none streams), and their answers with every name as its text.
use super::cases::compared;
use super::{TableInputs, TextResources, planned, ran};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, defines_rows, over_defines, over_nodes};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInputList, PlanNodeId, Resources,
};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Offset, Relation, Span, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as StepInput, LiteralText, Operand, PlanComparison,
    QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;

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

/// Runs `program`, every node before its emit held by the engine as well when `twin` is set.
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
    let resources = TextResources::new(&texts);
    let named: Vec<(Cell, Span)> = program
        .rows
        .iter()
        .map(|(name, span)| (resources.symbol(name), *span))
        .collect();
    let table = defines_rows(&named);
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let mut resources = resources;
    let mut logical = LogicalProgram::new();
    for step in &program.steps {
        let _id = logical.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    logical.emit(RuleOrdinal(0), program.emit);
    if twin {
        for node in (0..program.emit.0).map(PlanNodeId) {
            logical.hold(node);
        }
    }
    let plan = planned(&logical)?;
    let mut buffers = FileBuffers::default();
    for node in first {
        ran(&plan, *node, &mut buffers, &inputs, &mut resources)?;
    }
    let answer =
        ran(&plan, program.emit, &mut buffers, &inputs, &mut resources).and_then(
            |()| match buffers.rows(program.emit) {
                NodeRows::Materialized(table) => Ok(table
                    .cells
                    .chunks(table.kinds.len().max(1))
                    .take(table.count.0)
                    .map(|row| row.iter().map(|cell| shown(*cell, &resources)).collect())
                    .collect()),
                NodeRows::Absent => Err(PortableOperatorFault::NotRun),
            },
        );
    Ok(Ran {
        answer,
        formed: buffers.meter.formed.0,
        buffers,
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
