//! Test support for the plan and run use cases: the rows and resources an engine lends a run, held in tables and counted, and
//! the programs a case plans, built from operations over relations.

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{
    Candidates, FactRows, LogicalNode, NodeBindings, NodeRows, OpenFileBytes, PlanInput,
    PlanInputList, PlanInputs, PlanNodeId, RenderOutcomes, RenderRequests, Resources, RuleOrdinal,
    TextLookup,
};
use crate::inbound::use_cases::plan::interface::PlanUseCase as _;
use crate::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use crate::inbound::use_cases::run::interface::RunUseCase as _;
use crate::inbound::use_cases::run::use_case::RunUseCaseImpl;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn, Relation, Value};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::{ObservationBindingId, ParameterId, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::Table;
use std::cell::Cell;

/// The rows of relations, held as a worker's columns, lent to a run and counted per read.
#[derive(Debug, Default)]
pub struct TableInputs {
    /// Each relation's columns.
    held: Vec<(Relation, Vec<FactColumn>)>,
    /// How many times each relation was read, in `held`'s order.
    reads: Vec<Cell<usize>>,
    /// The rows of a binding no case holds.
    empty: Table,
}

impl TableInputs {
    /// Inputs holding each table's rows as its relation's columns.
    #[must_use]
    pub fn new(tables: &[(Relation, &Table)]) -> Self {
        let held: Vec<(Relation, Vec<FactColumn>)> = tables
            .iter()
            .map(|(relation, table)| (*relation, columns_of(table)))
            .collect();
        let reads = held.iter().map(|_| Cell::new(0)).collect();
        Self {
            held,
            reads,
            empty: Table::default(),
        }
    }

    /// How many times `relation`'s rows were read.
    #[must_use]
    pub fn reads(&self, relation: Relation) -> usize {
        self.held
            .iter()
            .zip(&self.reads)
            .filter(|((held, _), _)| *held == relation)
            .map(|(_, count)| count.get())
            .sum()
    }
}

impl PlanInputs for TableInputs {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        match self
            .held
            .iter()
            .zip(&self.reads)
            .find(|((held, _), _)| *held == relation)
        {
            Some(((_, columns), count)) => {
                count.set(count.get() + 1);
                FactRows::Lent(ColumnsView {
                    kinds: ryo_relations::domain::relations::fixtures::kinds(relation),
                    columns,
                })
            }
            None => FactRows::Refused,
        }
    }
    fn roots(&self, _root: PlanNodeId) -> Candidates<'_> {
        Candidates(&[])
    }
    fn bound(&self, _parameter: ParameterId) -> &Table {
        &self.empty
    }
    fn observation(&self, _input: ObservationBindingId) -> &Table {
        &self.empty
    }
}

/// The columns of `table`, as a reader writes a relation of its kinds.
#[must_use]
pub fn columns_of(table: &Table) -> Vec<FactColumn> {
    let width = table.kinds.len();
    (0..width)
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .iter()
                    .skip(column)
                    .step_by(width.max(1))
                    .map(|cell| ryo_relations::domain::relations::fixtures::to_bits(*cell).0)
                    .collect(),
            )
        })
        .collect()
}

/// The resources of a file whose symbols are `texts`, in symbol order: a symbol's text is its place in them.
#[derive(Debug, Default)]
pub struct TextResources {
    /// The file's symbols' texts, in symbol order.
    texts: Vec<Box<str>>,
    /// The file's tree: no node.
    tree: Elements,
}

impl TextResources {
    /// Resources whose symbols are `texts`.
    #[must_use]
    pub fn new(texts: &[&str]) -> Self {
        Self {
            texts: texts.iter().map(|text| Box::from(*text)).collect(),
            tree: Elements::default(),
        }
    }

    /// The cell of the symbol whose text is `text`.
    ///
    /// # Panics
    ///
    /// When `text` is none of the symbols.
    #[must_use]
    pub fn symbol(&self, text: &str) -> Value {
        let at = self
            .texts
            .iter()
            .position(|held| &**held == text)
            .and_then(|at| u32::try_from(at).ok());
        #[expect(clippy::expect_used, reason = "a case names only the texts it gave")]
        Value::Symbol(Symbol(at.expect("a text of the file")))
    }
}

impl Resources for TextResources {
    fn text(&self, value: Value) -> TextLookup<'_> {
        match value {
            Value::Symbol(Symbol(at)) => usize::try_from(at)
                .ok()
                .and_then(|at| self.texts.get(at))
                .map_or(TextLookup::NoText, |text| TextLookup::Found(CellText(text))),
            Value::Node(_)
            | Value::Name(_)
            | Value::File(_)
            | Value::Span(_)
            | Value::Number(_)
            | Value::Flag(_) => TextLookup::NoText,
        }
    }
    fn intern(&mut self, text: CellText<'_>) -> Value {
        let at = self
            .texts
            .iter()
            .position(|held| &**held == text.0)
            .unwrap_or_else(|| {
                self.texts.push(Box::from(text.0));
                self.texts.len() - 1
            });
        Value::Symbol(Symbol(u32::try_from(at).unwrap_or(u32::MAX)))
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        self.intern(text)
    }
    fn source(&self) -> OpenFileBytes<'_> {
        OpenFileBytes(&[])
    }
    fn tree(&self) -> &Elements {
        &self.tree
    }
    fn render(
        &mut self,
        _requests: RenderRequests<'_>,
    ) -> Result<RenderOutcomes, PortableOperatorFault> {
        Err(PortableOperatorFault::WrongBinding)
    }
}

/// The kinds of no language: a case's kind tests name none.
#[must_use]
pub fn vocabulary() -> KindVocabulary {
    KindVocabulary {
        language: LanguageName("cases".into()),
        kinds: Vec::new(),
    }
}

/// One step of a case's program: its operation and what each of its inputs reads.
#[derive(Clone, Debug)]
pub struct Step {
    /// The operation.
    pub operation: Operation,
    /// What its inputs read, in the order its kind reads them.
    pub inputs: Vec<PlanInput>,
}

/// The rows each emitted step answers, in emit order, or the first fault: the steps planned as one program, each the emit of
/// a rule of its own when `emits` names it, and run over `inputs` and `resources` with `buffers`.
///
/// # Errors
///
/// The fault `plan` or `run` answers.
pub fn emitted(
    steps: &[Step],
    emits: &[PlanNodeId],
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
    buffers: &mut FileBuffers,
) -> Result<Vec<Table>, PortableOperatorFault> {
    let vocabulary = vocabulary();
    let mut program = LogicalProgram::new(&vocabulary);
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for (rule, node) in emits.iter().enumerate() {
        program.emit(RuleOrdinal(u32::try_from(rule).unwrap_or(u32::MAX)), *node);
    }
    let plan = PlanUseCaseImpl.plan(&program)?;
    let run = RunUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::matching::factory::use_case(),
    ));
    let mut answered = Vec::with_capacity(emits.len());
    for node in emits {
        run.run(&plan, *node, buffers, inputs, resources)?;
        match buffers.rows(*node) {
            NodeRows::Materialized(table) => answered.push(table.clone()),
            NodeRows::Absent => return Err(PortableOperatorFault::NotRun),
        }
    }
    Ok(answered)
}

/// The plan the real planner makes of `program`: what a port double that plans as the operators do answers.
///
/// # Errors
///
/// The planner's shape fault.
pub fn planned(
    program: &LogicalProgram<'_>,
) -> Result<
    crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
    PortableOperatorFault,
> {
    PlanUseCaseImpl::new().plan(program)
}

/// What the real runner answers for `node`: what a port double that runs as the operators do answers.
///
/// # Errors
///
/// The runner's first fault.
pub fn ran(
    plan: &crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
) -> Result<(), PortableOperatorFault> {
    RunUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::matching::factory::use_case(),
    ))
    .run(plan, node, buffers, inputs, resources)
}
