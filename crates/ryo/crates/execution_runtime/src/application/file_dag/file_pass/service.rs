//! The file pass's service: one file's rules run through the request's planned program, the operators running each node over
//! the file's facts and tree in place; and the plan's root kinds, written as texts, mapped to the open file's own kind numbers,
//! so each node reading the tree starts from the tree nodes of its kinds, and a node with no root kind from every tree node.

use crate::application::file_dag::file_pass::types::{
    CountersListened, FileRoots, PassInputs, PassProgram, PassResources, PassScratch, PassSource,
    RootKindNodes, TreeStart,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::publications::service::bound::service::held_observation;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::{assemble, logical, root_kinds};
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
use ryo_execution_domain::domain::trust::service::step_read_sets;
use ryo_execution_domain::domain::trust::types::read_sets::value::{ReadSets, StepReadSets};
use ryo_language::domain::elements::service::{kind_at, kind_names, positions};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::of_kind;
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::types::{PortableOperatorFault, RenderRefusal, WithheldInput};
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
use ryo_values::domain::plan_parts::types::{HostedNumber, StepId};
use ryo_values::domain::row_texts::types::CellText;
use std::sync::{Arc, Mutex, PoisonError};

pub(crate) mod answering;
pub(crate) mod captures;
pub(crate) mod counters;
pub(crate) mod inputs;
pub(crate) mod nodes;
pub(crate) mod rules;
pub(crate) mod selection;

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
    // Spike (ryo-fmx3, host3): a node of one root kind reads that kind's list, shared with every node of the kind.
    let mut single = vec![u32::MAX; count];
    let mut seen = vec![0_u8; count];
    for (place, root) in kinds.iter().enumerate() {
        for node in &root.nodes {
            let at = usize::try_from(node.0).unwrap_or(usize::MAX);
            if let Some(start) = starts.get_mut(at) {
                *start = TreeStart::FromKinds;
            }
            if let (Some(times), Some(one)) = (seen.get_mut(at), single.get_mut(at)) {
                *times = times.saturating_add(1);
                *one = if *times == 1 {
                    u32::try_from(place).unwrap_or(u32::MAX)
                } else {
                    u32::MAX
                };
            }
        }
    }
    FileRoots {
        kind_lists: vec![Vec::new(); kinds.len()],
        numbers: Vec::new(),
        kinds,
        starts,
        lists: vec![Vec::new(); count],
        all: Vec::new(),
        single,
        kinds_filled: Vec::new(),
        nodes_filled: Vec::new(),
        names_seen: Vec::new(),
    }
}

/// Spike (ryo-fmx3, host3): [`fill_roots`] proportional to the file: each root kind's tree nodes listed once per file for the
/// kinds the file holds, a node of one kind reading its kind's list, and only what the last file filled cleared; the kind
/// numbers found once for every file that names the same kinds.
fn fill_roots_by_kind(roots: &mut FileRoots, elements: &Elements, texts: &Texts<'_>, file: FileId) {
    roots.all.clear();
    roots
        .all
        .extend(positions(elements).map(|at| node_id(file, at)));
    for kind in roots.kinds_filled.drain(..) {
        if let Some(list) = roots.kind_lists.get_mut(usize::try_from(kind).unwrap_or(usize::MAX)) {
            list.clear();
        }
    }
    for node in roots.nodes_filled.drain(..) {
        if let Some(list) = roots.lists.get_mut(usize::try_from(node).unwrap_or(usize::MAX)) {
            list.clear();
        }
    }
    let names = kind_names(elements);
    let same = names.len() == roots.names_seen.len()
        && names.iter().zip(&roots.names_seen).all(|(name, seen)| {
            symbol_text(texts, *name).is_some_and(|found| found.0 == &**seen)
        });
    if !same {
        roots.names_seen.clear();
        roots.names_seen.extend(names.iter().map(|name| {
            symbol_text(texts, *name).map_or_else(|| Box::from("\u{0}unnamed"), |found| Box::from(found.0))
        }));
        // A name with no text never matches a root kind, as `symbol_text` answering none never did.
        let unnamed: Vec<bool> = names.iter().map(|name| symbol_text(texts, *name).is_none()).collect();
        roots.numbers.clear();
        let numbers: Vec<Option<u16>> = roots
            .kinds
            .iter()
            .map(|root| {
                roots
                    .names_seen
                    .iter()
                    .zip(&unnamed)
                    .position(|(seen, none)| !*none && **seen == *root.kind.0)
                    .and_then(|kind| u16::try_from(kind).ok())
            })
            .collect();
        roots.numbers = numbers;
    }
    let indexed = (0..names.len()).any(|kind| {
        u16::try_from(kind).is_ok_and(|kind| !of_kind(elements, KindBits(kind)).is_empty())
    });
    for (place, root) in roots.kinds.iter().enumerate() {
        let Some(Some(kind)) = roots.numbers.get(place).copied() else {
            continue;
        };
        let Some(list) = roots.kind_lists.get_mut(place) else {
            continue;
        };
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
        if list.is_empty() {
            continue;
        }
        list.sort_unstable();
        list.dedup();
        roots
            .kinds_filled
            .push(u32::try_from(place).unwrap_or(u32::MAX));
        // A node of several kinds holds every one of its kinds' tree nodes: within one file, tree order is node order.
        for node in &root.nodes {
            let at = usize::try_from(node.0).unwrap_or(usize::MAX);
            if roots.single.get(at).is_some_and(|one| *one != u32::MAX) {
                continue;
            }
            if let Some(own) = roots.lists.get_mut(at) {
                if own.is_empty() {
                    roots.nodes_filled.push(node.0);
                }
                own.extend_from_slice(list);
            }
        }
    }
    for node in &roots.nodes_filled {
        if let Some(list) = roots.lists.get_mut(usize::try_from(*node).unwrap_or(usize::MAX)) {
            list.sort_unstable();
            list.dedup();
        }
    }
}

/// Makes the candidates of `file`, whose tree is `elements` and whose kind names read through `texts`.
pub(crate) fn fill_roots(
    roots: &mut FileRoots,
    elements: &Elements,
    texts: &Texts<'_>,
    file: FileId,
) {
    if ryo_operators::spike::host_lever(ryo_operators::spike::HostLever::Roots) {
        return fill_roots_by_kind(roots, elements, texts, file);
    }
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
    // Spike (ryo-fmx3, host3): a node of one root kind reads its kind's list (empty when the file holds none of the kind).
    if let Some(one) = roots.single.get(at)
        && *one != u32::MAX
        && ryo_operators::spike::host_lever(ryo_operators::spike::HostLever::Roots)
    {
        return Candidates(
            roots
                .kind_lists
                .get(usize::try_from(*one).unwrap_or(usize::MAX))
                .map_or(&[][..], Vec::as_slice),
        );
    }
    match (roots.starts.get(at), roots.lists.get(at)) {
        (Some(TreeStart::FromKinds), Some(list)) => Candidates(list),
        _ => Candidates(&roots.all),
    }
}

/// The program the file pass runs for a request: its rules merged into one logical program, planned by its operators and
/// assembled, with what each step of each rule reads, each read set stored once in `sets`, the program's one store.
///
/// # Errors
///
/// The logical program's refusal, or the shape fault the operators plan with.
pub(crate) fn pass_program(
    source: &PassSource,
    sets: &mut ReadSets,
) -> Result<PassProgram, OperatingError> {
    conditioned_pass(source, sets, None, None)
}

/// Prepare the ordinary file program together with a variant for one typed source condition.
pub(crate) fn conditioned_pass(
    source: &PassSource,
    sets: &mut ReadSets,
    condition: Option<ryo_execution_domain::domain::executable_program::types::SourceAbsence>,
    extended: Option<Relation>,
) -> Result<PassProgram, OperatingError> {
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
    inputs.assuming(None, extended);
    let normal = logical(&rules, &mut inputs)?;
    let (program, conditional) = if let Some(absence) = condition {
        let mut alternative = ProgramInputs::hosting(hosted.clone());
        alternative.assuming(Some(absence.clone()), extended);
        let mut storage = ProgramInputs::default();
        let selected = (|| {
            let other = logical(&rules, &mut alternative)?;
            let joined = ryo_execution_domain::domain::executable_program::routes::service::joined(
                normal,
                other,
                &rules,
                &mut storage,
            )?;
            let combined = joined.program;
            let routes = joined.routes;
            let physical = operators.plan(combined.program()).map_err(portable_fault)?;
            Ok::<_, OperatingError>((assemble(combined, physical), routes))
        })();
        match selected {
            Ok((program, routes)) => (
                program,
                Some(Box::new(
                    crate::application::file_dag::file_pass::types::ConditionalFileProgram {
                        absence,
                        routes,
                    },
                )),
            ),
            Err(_) => return conditioned_pass(source, sets, None, extended),
        }
    } else {
        let physical = operators.plan(normal.program()).map_err(portable_fault)?;
        (assemble(normal, physical), None)
    };
    let reads = plans
        .iter()
        .map(|plan| step_read_sets(plan, sets))
        .collect();
    Ok(PassProgram {
        conditional,
        program,
        operators: Arc::clone(operators),
        plans: plans.to_vec().into_boxed_slice(),
        reads,
        scratch: Mutex::new(Vec::new()),
        ids: ids.clone().into_boxed_slice(),
        turns: std::sync::OnceLock::new(),
        hosted_steps: plans
            .iter()
            .map(|plan| {
                plan.steps
                    .iter()
                    .filter(|step| step.scope == ryo_values::domain::plan_parts::types::StepScope::PerFile)
                    .filter_map(|step| match step.operation {
                        ryo_values::domain::plans::types::Operation::Hosted { relation } => Some(relation),
                        _ => None,
                    })
                    .collect()
            })
            .collect(),
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
        if counters::service::listened() == CountersListened::Listening {
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

/// What step `step` of rule `rule` of `pass` reads, when the program holds that step.
pub(crate) fn step_sets(
    pass: &PassProgram,
    rule: RuleIndex,
    step: StepId,
) -> Option<&StepReadSets> {
    pass.reads
        .get(rule.0)
        .and_then(|reads| reads.0.get(usize::try_from(step.0).unwrap_or(usize::MAX)))
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
