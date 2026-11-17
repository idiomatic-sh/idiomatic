//! Test support for the plan and run use cases: the rows and resources an engine lends a run, held in tables and counted, and
//! the programs a case plans, built from operations over relations.

use crate::domain::contract::types::{PortableOperatorFault, WithheldInput};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use crate::domain::physical_plan::types::candidates::value::Candidates;
use crate::domain::physical_plan::types::fact_rows::value::FactRows;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use crate::domain::physical_plan::types::held_rows::value::HeldRows;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, OpenFileBytes, PlanInputList, PlanInputs, PlanNodeId,
    RenderOutcomes, RenderRequests, Resources,
};
use crate::domain::table_texts::types::Texts;
use crate::inbound::use_cases::plan::interface::PlanUseCase as _;
use crate::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use crate::inbound::use_cases::run::interface::RunUseCase as _;
use crate::inbound::use_cases::run::use_case::RunUseCaseImpl;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::{intern, intern_derived, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn, NodeId, Relation, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{HostedNumber, StepScope};
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::Table;
use std::cell::Cell;

pub mod allocating;
#[cfg(test)]
pub(crate) mod cases;
pub mod counting;
pub mod demands;
#[cfg(test)]
pub(crate) mod edges;
pub(crate) mod graph_workloads;
pub mod held;
mod kernel_programs;
#[cfg(test)]
pub(crate) mod relational;
pub(crate) mod stream_programs;
pub(crate) mod streams;
pub mod texts;
pub mod trees;
pub mod unshared;
use held::HeldInputs;

/// The rows of relations, held as a worker's columns, lent to a run and counted per read.
#[derive(Debug, Default)]
pub struct TableInputs {
    /// Each relation's columns.
    held: Vec<(Relation, Vec<FactColumn>)>,
    /// How many times each relation was read, in `held`'s order.
    reads: Vec<Cell<usize>>,
    /// The tree nodes every node reading the tree starts from, in tree order.
    roots: Vec<NodeId>,
    /// The rows held for each frontier entry, bound observation and host relation.
    after: HeldInputs,
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
            roots: Vec::new(),
            after: HeldInputs::default(),
        }
    }

    /// These inputs, with every node reading the tree starting from `roots`.
    #[must_use]
    pub fn with_roots(self, roots: Vec<NodeId>) -> Self {
        Self { roots, ..self }
    }

    /// These inputs, with the host's rows of each relation, by its place.
    #[must_use]
    pub fn with_hosted(mut self, tables: Vec<Table>) -> Self {
        self.after.hosted = tables;
        self
    }

    /// These inputs, withholding every frontier entry, bound observation and host relation with `token`.
    #[must_use]
    pub fn withholding(mut self, token: WithheldInput) -> Self {
        self.after.withheld = Some(token);
        self
    }

    /// How many times the host's rows were read.
    #[must_use]
    pub fn hosted_reads(&self) -> usize {
        self.after.hosted_reads.get()
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
        self.after
            .demands
            .asked(relation, self.held.iter().map(|(held, _)| *held));
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
        Candidates(&self.roots)
    }
    fn frontier(&self, _place: FrontierPlace) -> HeldRows<'_> {
        self.after.held(demands::Demand::Frontier)
    }
    fn observation(&self, _place: BoundObservationPlace) -> HeldRows<'_> {
        self.after.held(demands::Demand::Observation)
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        self.after.hosted(relation)
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

/// The resources of a file whose bytes are `texts`, one after the other, each text one of its symbols.
#[derive(Debug, Default)]
pub struct TextResources {
    /// The file's bytes: every text, in order.
    source: Vec<u8>,
    /// The file's symbols: one for each distinct text.
    symbols: Symbols,
    /// The file's tree: no node, but in the small tree's resources.
    tree: Elements,
    /// What each rendering of a batch answers, in request order.
    renderings: Vec<ryo_language::domain::semantics::types::RenderOutcome>,
    /// What the run asked of the resources, in order.
    calls: Vec<ResourceCall>,
}

/// One request a run made of the resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceCall {
    /// A text interned in the file's symbols.
    Interned(String),
    /// A text minted as a derived symbol.
    Minted(String),
    /// A batch of renderings.
    Rendered(Vec<ryo_language::domain::semantics::types::RenderRequest>),
}

impl TextResources {
    /// Resources whose file is `texts`, one after the other, each a symbol of it.
    #[must_use]
    pub fn new(texts: &[&str]) -> Self {
        let source: Vec<u8> = texts.iter().flat_map(|text| text.bytes()).collect();
        let mut symbols = Symbols::default();
        let mut start = 0_u32;
        for text in texts {
            let end = start.saturating_add(u32::try_from(text.len()).unwrap_or(u32::MAX));
            let _symbol = intern(&mut symbols, ByteSlice(&source), Offset(start), Offset(end));
            start = end;
        }
        Self {
            source,
            symbols,
            tree: Elements::default(),
            renderings: Vec::new(),
            calls: Vec::new(),
        }
    }

    /// These resources, each rendering of a batch answering `renderings` in request order.
    #[must_use]
    pub fn rendering(
        mut self,
        renderings: Vec<ryo_language::domain::semantics::types::RenderOutcome>,
    ) -> Self {
        self.renderings = renderings;
        self
    }

    /// What the run asked of the resources, in order.
    #[must_use]
    pub fn calls(&self) -> &[ResourceCall] {
        &self.calls
    }

    /// The symbol a derived text is.
    fn derived(&mut self, text: CellText<'_>) -> Value {
        Value::Symbol(intern_derived(
            &mut self.symbols,
            ByteSlice(&self.source),
            NameText(text.0),
        ))
    }

    /// The cell of the symbol whose text is `text`.
    ///
    /// # Panics
    ///
    /// When `text` is none of the symbols.
    #[must_use]
    pub fn symbol(&self, text: &str) -> Value {
        let found = (0..u32::MAX)
            .map(Symbol)
            .map_while(|symbol| {
                text_of(&self.symbols, ByteSlice(&self.source), symbol).map(|held| (symbol, held))
            })
            .find(|(_, held)| held.0 == text);
        #[expect(clippy::expect_used, reason = "a case names only the texts it gave")]
        Value::Symbol(found.expect("a text of the file").0)
    }
}

impl Resources for TextResources {
    fn texts(&self) -> Texts<'_> {
        Texts {
            symbols: &self.symbols,
            source: SourceView(&self.source),
            paths: &[],
            arenas: &[],
        }
    }
    fn intern(&mut self, text: CellText<'_>) -> Value {
        self.calls.push(ResourceCall::Interned(text.0.to_owned()));
        self.derived(text)
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        self.calls.push(ResourceCall::Minted(text.0.to_owned()));
        self.derived(text)
    }
    fn source(&self) -> OpenFileBytes<'_> {
        OpenFileBytes(&self.source)
    }
    fn tree(&self) -> &Elements {
        &self.tree
    }
    fn render(
        &mut self,
        requests: RenderRequests<'_>,
    ) -> Result<RenderOutcomes, PortableOperatorFault> {
        if self.renderings.is_empty() {
            return Err(PortableOperatorFault::WrongBinding);
        }
        self.calls.push(ResourceCall::Rendered(requests.0.to_vec()));
        Ok(RenderOutcomes(
            self.renderings
                .iter()
                .take(requests.0.len())
                .cloned()
                .collect(),
        ))
    }
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
    let matching = std::sync::Arc::new(crate::outbound::matching::factory::use_case());
    emitted_through(steps, emits, (&[], matching), inputs, resources, buffers)
}

/// What a program reads besides rows: the bindings every node reads, and the matching port the runner is composed with.
pub type Composed<'a> = (
    &'a [ryo_values::domain::query_bindings::types::ParameterBinding],
    std::sync::Arc<dyn crate::outbound::matching::interface::MatchingPort>,
);

/// [`emitted`], each node reading `composed`'s bindings, run through `composed`'s matching port.
///
/// # Errors
///
/// The fault `plan` or `run` answers.
pub fn emitted_through(
    steps: &[Step],
    emits: &[PlanNodeId],
    (bindings, matching): Composed<'_>,
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
    buffers: &mut FileBuffers,
) -> Result<Vec<Table>, PortableOperatorFault> {
    let mut program = LogicalProgram::new();
    for step in steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(bindings),
            scope: StepScope::PerFile,
        });
    }
    for (rule, node) in emits.iter().enumerate() {
        program.emit(RuleOrdinal(u32::try_from(rule).unwrap_or(u32::MAX)), *node);
    }
    let plan = PlanUseCaseImpl.plan(&program)?;
    let run = RunUseCaseImpl::new(matching);
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
