//! Test support for the plan and run use cases: the rows and resources an engine lends a run, held in tables and counted, and
//! the programs a case plans, built from operations over relations.

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{
    Candidates, FactRows, LogicalNode, NodeBindings, NodeRows, OpenFileBytes, PlanInputList,
    PlanInputs, PlanNodeId, RenderOutcomes, RenderRequests, Resources, RuleOrdinal,
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
use ryo_values::domain::plan_parts::types::{ObservationBindingId, ParameterId, StepScope};
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::Table;
use std::cell::Cell;

pub mod allocating;
#[cfg(test)]
pub(crate) mod cases;
pub mod counting;
pub mod texts;
pub mod unshared;

/// The rows of relations, held as a worker's columns, lent to a run and counted per read.
#[derive(Debug, Default)]
pub struct TableInputs {
    /// Each relation's columns.
    held: Vec<(Relation, Vec<FactColumn>)>,
    /// How many times each relation was read, in `held`'s order.
    reads: Vec<Cell<usize>>,
    /// The rows of a binding no case holds.
    empty: Table,
    /// The tree nodes every node reading the tree starts from, in tree order.
    roots: Vec<NodeId>,
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
            roots: Vec::new(),
        }
    }

    /// These inputs, with every node reading the tree starting from `roots`.
    #[must_use]
    pub fn with_roots(self, roots: Vec<NodeId>) -> Self {
        Self { roots, ..self }
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
        Candidates(&self.roots)
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

    /// The resources of the file `abc` in file 0: a `file` node over the tokens `a` (a `word`), `b` (an `other`) and `c` (a
    /// `word`), at positions 1 to 3.
    #[must_use]
    pub fn small_tree() -> Self {
        use ryo_language::domain::builders::service::{
            enter_node, leave_node, table_builder, token,
        };
        use ryo_language::domain::completion::service::{completion, name_kind};
        use ryo_relations::domain::relations::types::{FileId, KindBits};
        let source = b"abc".to_vec();
        let (mut tree, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder = table_builder(&mut tree, &mut symbols, ByteSlice(&source));
        enter_node(&mut builder, KindBits(1), Offset(0));
        for (bits, start) in [(2, 0), (3, 1), (2, 2)] {
            token(
                &mut builder,
                KindBits(bits),
                Offset(start),
                Offset(start + 1),
            );
        }
        leave_node(&mut builder, Offset(3));
        for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
            let mut names = completion(&mut tree, &mut symbols, ByteSlice(&source), FileId(0));
            name_kind(&mut names, KindBits(bits), NameText(name));
        }
        Self {
            source,
            symbols,
            tree,
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

/// The kinds of no language: a case's kind tests name none.
#[must_use]

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

/// The rows every node of `steps` answers, each the emit of a rule of its own, planned with `rewrites` alone and run over
/// `inputs` and `resources`: what a program answers with and without a rewrite.
///
/// # Errors
///
/// The fault the planner or the runner answers.
#[cfg(test)]
pub(crate) fn rewritten(
    steps: &[Step],
    rewrites: &[&dyn crate::domain::physical_plan::collections::rewrites::interface::RewritesItem],
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
) -> Result<Vec<Table>, PortableOperatorFault> {
    let mut program = LogicalProgram::new();
    let mut emits = Vec::with_capacity(steps.len());
    for (rule, step) in (0..).map(RuleOrdinal).zip(steps) {
        let id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
        program.emit(rule, id);
        emits.push(id);
    }
    let plan = crate::domain::physical_plan::service::plan_with(&program, rewrites)?;
    let mut buffers = FileBuffers::default();
    emits
        .into_iter()
        .map(|node| {
            ran(&plan, node, &mut buffers, inputs, resources)?;
            match buffers.rows(node) {
                NodeRows::Materialized(table) => Ok(table.clone()),
                NodeRows::Absent => Err(PortableOperatorFault::NotRun),
            }
        })
        .collect()
}
