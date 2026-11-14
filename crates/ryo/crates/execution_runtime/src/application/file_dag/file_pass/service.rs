//! The file pass's service: one file's rules run through the request's planned program, the operators running each node over
//! the file's facts and tree in place; and the plan's root kinds, written as texts, mapped to the open file's own kind numbers,
//! so each node reading the tree starts from the tree nodes of its kinds, and a node with no root kind from every tree node.

use crate::application::file_dag::file_pass::types::{
    FileRoots, NodeReads, PassInputs, PassProgram, PassResources, PassScratch, PassSource,
    RootKindNodes, RulesHeld, Stages, TreeStart,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::constants::BOUND_STAGE;
use ryo_execution_domain::domain::executable_program::service::{
    assemble, logical, root_kinds, serving,
};
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
use ryo_execution_domain::domain::executable_program::types::{
    RuleBindings, RuleInput, RuleObservations, Serving,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::trust::service::{needs_of, physical_reads};
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_language::domain::elements::service::{kind_at, kind_names, positions};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::of_kind;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::{
    Candidates, FactRows, OpenFileBytes, PlanInputs, PlanNodeId, RenderOutcomes, RenderRequests,
    Resources,
};
use ryo_operators::domain::table_texts::service::symbol_text;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{FileId, KindBits, Relation, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{Input, ObservationBindingId, ParameterId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::Table;
use std::sync::{Arc, Mutex, PoisonError};

pub(crate) mod captures;
pub(crate) mod nodes;
pub(crate) mod rules;

/// The relations `plans` read, each once, in the order they are first read: what each file's sources are asked for, so a file
/// reads exactly what its request's rules can read.
pub(crate) fn demanded<P: std::borrow::Borrow<Plan>>(plans: &[P]) -> Box<[Relation]> {
    let mut demanded: Vec<Relation> = Vec::new();
    for step in plans.iter().flat_map(|plan| &plan.borrow().steps) {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Relation(relation) = *input
                && !demanded.contains(&relation)
            {
                demanded.push(relation);
            }
        }
    }
    demanded.into_boxed_slice()
}

/// The roots of `program`, before any file.
pub(crate) fn file_roots(program: &ExecutableProgram) -> FileRoots {
    let kinds: Vec<RootKindNodes> = root_kinds(program)
        .map(|root| RootKindNodes {
            kind: root.kind.clone(),
            nodes: root.nodes.to_vec(),
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
    let mut inputs = ProgramInputs::default();
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
    given.unwrap_or_else(|| PassScratch {
        roots: file_roots(&pass.program),
        ..PassScratch::default()
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
        match self.lookup.columns(relation) {
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
    fn bound(&self, _parameter: ParameterId) -> &Table {
        &self.unserved
    }
    fn observation(&self, _input: ObservationBindingId) -> &Table {
        &self.unserved
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
        _requests: RenderRequests<'_>,
    ) -> Result<RenderOutcomes, PortableOperatorFault> {
        // Rendering is the second stage's: a rule that renders is refused before any file.
        Err(PortableOperatorFault::UnsupportedRender)
    }
}

/// Refuses opening the project phase or the observation stage of a request holding a rule no stage serves yet, with the latest
/// stage among them.
///
/// # Errors
///
/// `NotServedUntil` the latest unserved stage.
pub(crate) fn served(pass: &PassProgram) -> Result<(), OperatingError> {
    stages(pass)
        .latest
        .map_or(Ok(()), |stage| Err(OperatingError::NotServedUntil(stage)))
}

/// Refuses evaluating a rule after the files: until the fourth stage no rule runs there. The refusal names the latest stage
/// among the request's rules that no stage serves yet, or the fourth when every rule is served in the files.
///
/// # Errors
///
/// `NotServedUntil` that stage when the request holds a rule.
pub(crate) fn served_after_the_files(pass: &PassProgram) -> Result<(), OperatingError> {
    match stages(pass) {
        Stages {
            latest: Some(stage),
            ..
        } => Err(OperatingError::NotServedUntil(stage)),
        Stages {
            latest: None,
            held: RulesHeld::Some,
        } => Err(OperatingError::NotServedUntil(BOUND_STAGE)),
        Stages {
            latest: None,
            held: RulesHeld::None,
        } => Ok(()),
    }
}

/// The latest stage among the request's rules that no stage serves yet, and whether it holds a rule at all.
fn stages(pass: &PassProgram) -> Stages {
    let mut found = Stages {
        latest: None,
        held: RulesHeld::None,
    };
    let mut rule = RuleIndex(0);
    while let Ok(served) = serving(&pass.program, rule) {
        found.held = RulesHeld::Some;
        if let Serving::NotServedUntil(stage) = served {
            found.latest = found.latest.max(Some(stage));
        }
        rule = RuleIndex(rule.0.saturating_add(1));
    }
    found
}
