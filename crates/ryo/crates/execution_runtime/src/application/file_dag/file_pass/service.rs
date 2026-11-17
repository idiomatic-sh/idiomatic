//! The file pass's service: one file's rules run through the request's planned program, the operators running each node over
//! the file's facts and tree in place; and the plan's root kinds, written as texts, mapped to the open file's own kind numbers,
//! so each node reading the tree starts from the tree nodes of its kinds, and a node with no root kind from every tree node.

use crate::application::file_dag::file_pass::constants::NODE_COUNTERS_TARGET;
use crate::application::file_dag::file_pass::service::reads::service::read;
use crate::application::file_dag::file_pass::service::triggers::service::node_activity;
use crate::application::file_dag::file_pass::types::{
    FileRoots, NodeReads, PassInputs, PassProgram, PassResources, PassScratch, PassSource,
    RootKindNodes, TreeStart,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::publications::service::bound::service::held_observation;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::activation::service::trigger_kinds;
use ryo_execution_domain::domain::executable_program::activation::types::NodeActivity;
use ryo_execution_domain::domain::executable_program::service::refusals::service::layout;
use ryo_execution_domain::domain::executable_program::service::{
    activation, assemble, logical, roots,
};
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
use ryo_execution_domain::domain::executable_program::types::{
    RuleBindings, RuleInput, RuleObservations,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::{
    portable_fault, rendered_batch, unsupported_render,
};
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::trust::service::{needs_of, physical_reads};
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_language::domain::elements::service::{kind_at, kind_names, positions};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::of_kind;
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::types::{PortableOperatorFault, RenderRefusal, WithheldInput};
use ryo_operators::domain::physical_plan::types::activity::value::{Activity, NodeLayout};
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::candidates::value::Candidates;
use ryo_operators::domain::physical_plan::types::fact_rows::value::FactRows;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::{
    OpenFileBytes, PlanInputs, PlanNodeId, RenderOutcomes, RenderRequests, Resources,
};
use ryo_operators::domain::table_texts::service::symbol_text;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{FileId, KindBits, Relation, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{HostedNumber, Input};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::row_texts::types::CellText;
use std::sync::{Arc, Mutex, PoisonError};

pub(crate) mod answering;
pub(crate) mod captures;
pub(crate) mod counters;
pub(crate) mod inputs;
pub(crate) mod nodes;
pub(crate) mod reads;
pub(crate) mod rules;
pub(crate) mod triggers;

/// Writes into `into` the relations the rules that run on a file read, each once, in rule order: the union of `reads` over the
/// rules whose intent for the file is Run. A rule that does not run on the file evaluates none of its nodes there, so nothing
/// only it reads is asked of the file's sources.
pub(crate) fn file_demand(
    program: &PreparedProgram,
    intents: &[FileRuleIntent],
    into: &mut Vec<Relation>,
) {
    into.clear();
    for (rule, intent) in intents.iter().enumerate() {
        if !matches!(intent, FileRuleIntent::Run { .. }) {
            continue;
        }
        // A rule the program does not hold runs nothing, so it reads nothing.
        let Ok(reads) = program.reads(RuleIndex(rule)) else {
            continue;
        };
        for relation in &reads.0 {
            if !into.contains(relation) {
                into.push(*relation);
            }
        }
    }
}

/// The roots of `program`, before any file.
pub(crate) fn file_roots(program: &ExecutableProgram) -> FileRoots {
    // In the activation plan's kind order, so a root kind's place is its trigger's slot.
    let kinds: Vec<RootKindNodes> = trigger_kinds(activation(program))
        .iter()
        .map(|kind| RootKindNodes {
            kind: kind.clone(),
            nodes: roots(program, kind).to_vec(),
        })
        .collect();
    let count = kinds
        .iter()
        .flat_map(|root| &root.nodes)
        .map(|node| {
            usize::try_from(node.0)
                .unwrap_or(usize::MAX)
                .saturating_add(1)
        })
        .max()
        .unwrap_or(0);
    let mut starts = vec![TreeStart::FromEveryNode; count];
    for node in kinds.iter().flat_map(|root| &root.nodes) {
        if let Some(start) = starts.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) {
            *start = TreeStart::FromKinds;
        }
    }
    FileRoots {
        kinds,
        starts,
        lists: vec![Vec::new(); count],
        all: Vec::new(),
    }
}

/// Makes the candidates of `file`, whose tree is `elements` and whose kind names read through `texts`.
pub(crate) fn fill_roots(
    roots: &mut FileRoots,
    elements: &Elements,
    texts: &Texts<'_>,
    file: FileId,
) {
    roots.all.clear();
    roots
        .all
        .extend(positions(elements).map(|at| node_id(file, at)));
    for list in &mut roots.lists {
        list.clear();
    }
    let names = kind_names(elements);
    // A tree built without its kind index lists no element under any kind: its roots are found by reading each element's kind.
    let indexed = (0..names.len()).any(|kind| {
        u16::try_from(kind).is_ok_and(|kind| !of_kind(elements, KindBits(kind)).is_empty())
    });
    for root in &roots.kinds {
        let numbered = names.iter().position(|name| {
            symbol_text(texts, *name).is_some_and(|found| found.0 == &*root.kind.0)
        });
        // A kind the file does not name, or a number past the kinds a file can number, has no tree node in it.
        let Some(Ok(kind)) = numbered.map(u16::try_from) else {
            continue;
        };
        for node in &root.nodes {
            if let Some(list) = roots
                .lists
                .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                if indexed {
                    list.extend(
                        of_kind(elements, KindBits(kind))
                            .iter()
                            .map(|at| node_id(file, *at)),
                    );
                } else {
                    list.extend(
                        positions(elements)
                            .filter(|at| kind_at(elements, *at) == KindBits(kind))
                            .map(|at| node_id(file, at)),
                    );
                }
            }
        }
    }
    // A node of two kinds holds both kinds' tree nodes: within one file, tree order is node order.
    for list in &mut roots.lists {
        list.sort_unstable();
        list.dedup();
    }
}

/// The tree nodes `node` starts from in the open file, in tree order.
pub(crate) fn roots_of(roots: &FileRoots, node: PlanNodeId) -> Candidates<'_> {
    let at = usize::try_from(node.0).unwrap_or(usize::MAX);
    match (roots.starts.get(at), roots.lists.get(at)) {
        (Some(TreeStart::FromKinds), Some(list)) => Candidates(list),
        _ => Candidates(&roots.all),
    }
}

/// The program the file pass runs for a request: its rules merged into one logical program, planned by its operators and
/// assembled, with what each step of each rule reads.
///
/// # Errors
///
/// The logical program's refusal, or the shape fault the operators plan with.
pub(crate) fn pass_program(source: &PassSource) -> Result<PassProgram, OperatingError> {
    let PassSource {
        plans,
        bindings,
        operators,
        hosted,
        ids,
    } = source;
    let rules: Vec<RuleInput<'_>> = plans
        .iter()
        .zip(bindings)
        .map(|(plan, bound)| RuleInput {
            plan,
            bindings: RuleBindings(bound),
            observations: RuleObservations(&plan.observation_inputs),
        })
        .collect();
    let mut inputs = ProgramInputs::hosting(hosted.clone());
    let build = logical(&rules, &mut inputs)?;
    let plan = operators.plan(build.program()).map_err(portable_fault)?;
    let program = assemble(build, plan);
    let reads = plans.iter().map(|plan| step_reads(plan)).collect();
    Ok(PassProgram {
        program,
        operators: Arc::clone(operators),
        plans: plans.to_vec().into_boxed_slice(),
        reads,
        scratch: Mutex::new(Vec::new()),
        ids: ids.clone().into_boxed_slice(),
    })
}

/// What a file of `pass` runs in: what an earlier file gave back, or one made for the program, its roots' kinds found.
pub(crate) fn taken(pass: &PassProgram) -> PassScratch {
    // A file that panicked held its own scratch, never the pool's: what a poisoned pool holds is whole.
    let given = pass
        .scratch
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .pop();
    given.unwrap_or_else(|| {
        let mut made = PassScratch {
            roots: file_roots(&pass.program),
            ..PassScratch::default()
        };
        // The node counters are on only for a run whose subscriber listens to them, so a run that does not pays nothing.
        if tracing::enabled!(target: NODE_COUNTERS_TARGET, tracing::Level::INFO) {
            made.buffers.count_nodes();
        }
        made
    })
}

/// Gives back what a file of `pass` ran in, for the next file to refill without growing it.
pub(crate) fn given_back(pass: &PassProgram, scratch: PassScratch) {
    pass.scratch
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(scratch);
}

/// What each step of `plan` reads: every relation read on the way to its rows, and the trusted reads of it and the steps it
/// reads.
fn step_reads(plan: &Plan) -> Box<[NodeReads]> {
    let uses = physical_reads(plan);
    let trusted = needs_of(plan);
    let mut reads: Vec<NodeReads> = Vec::with_capacity(plan.steps.len());
    for (at, step) in plan.steps.iter().enumerate() {
        let mut each = NodeReads {
            // A step refused for an authoring error reads nothing here: it never runs, and its rule answers its fault.
            uses: match uses.get(at) {
                Some(Ok(read)) => read.clone(),
                Some(Err(_)) | None => Vec::new(),
            },
            needs: Vec::new(),
        };
        absorb(
            &mut each.needs,
            trusted.get(at).map_or(&[][..], Vec::as_slice),
        );
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = input
                && let Some(earlier) = reads.get(usize::try_from(id.0).unwrap_or(usize::MAX))
            {
                let needs = earlier.needs.clone();
                absorb(&mut each.needs, &needs);
            }
        }
        reads.push(each);
    }
    reads.into_boxed_slice()
}

/// Adds each read of `more` that `into` does not hold yet.
fn absorb(into: &mut Vec<RelationRead>, more: &[RelationRead]) {
    for read in more {
        if !into.contains(read) {
            into.push(read.clone());
        }
    }
}

impl PlanInputs for PassInputs<'_, '_, '_> {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        // The file's one read of the relation: every reader answers it.
        let read = read(&self.reads, self.lookup, relation);
        match read {
            Ok(view) => FactRows::Lent(view),
            Err(error) => {
                let mut refused = self.refused.borrow_mut();
                if refused.is_none() {
                    *refused = Some(error);
                }
                FactRows::Refused
            }
        }
    }
    fn roots(&self, root: PlanNodeId) -> Candidates<'_> {
        roots_of(self.roots, root)
    }
    fn frontier(&self, _place: FrontierPlace) -> HeldRows<'_> {
        // A step in a file reads no handover: only a step after the files does.
        HeldRows::Withheld(WithheldInput(u32::MAX))
    }
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_> {
        held_observation(&self.bound, place)
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        match self
            .hosted
            .get(usize::try_from(relation.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
        {
            Some(table) => HeldRows::Held(table),
            None => HeldRows::Withheld(WithheldInput(u32::MAX)),
        }
    }
    fn activity(&self, node: PlanNodeId) -> Activity<'_> {
        match (node_activity(self, node), layout(self.program, node)) {
            (NodeActivity::Dormant, Ok(kinds)) => Activity::Dormant(NodeLayout(kinds)),
            // A node whose layout is a refusal answers it by running.
            (NodeActivity::Dormant, Err(_)) | (NodeActivity::Active, _) => Activity::Active,
        }
    }
}

impl Resources for PassResources<'_> {
    fn texts(&self) -> Texts<'_> {
        Texts {
            symbols: self.symbols,
            source: self.source,
            paths: self.paths,
            arenas: &[],
        }
    }
    fn intern(&mut self, text: CellText<'_>) -> Value {
        self.mint(text)
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        // Within a file a made text is the file's own derived symbol.
        Value::Symbol(intern_derived(
            self.symbols,
            ByteSlice(self.source.0),
            NameText(text.0),
        ))
    }
    fn source(&self) -> OpenFileBytes<'_> {
        OpenFileBytes(self.source.0)
    }
    fn tree(&self) -> &Elements {
        self.elements
    }
    fn render(
        &mut self,
        requests: RenderRequests<'_>,
    ) -> Result<RenderOutcomes, PortableOperatorFault> {
        // The engine's reason is kept for the run's error: no session, a refused batch, a batch of another length, or an
        // answer the language cannot render.
        let refused = match rendered_batch(self.semantic, requests.0, self.source) {
            Ok(answers)
                if !answers
                    .iter()
                    .any(|answer| matches!(answer, RenderOutcome::Unsupported(_))) =>
            {
                return Ok(RenderOutcomes(answers));
            }
            Ok(answers) => unsupported_render(&answers),
            Err(error) => error,
        };
        // The refusal is recorded once, and its place travels with the node's fault to every reader.
        let refusal = RenderRefusal(u32::try_from(self.refusals.len()).unwrap_or(u32::MAX));
        self.refusals.push(refused);
        Err(PortableOperatorFault::RenderRefused { refusal })
    }
}
