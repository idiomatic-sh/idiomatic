//! The file pass's service: one file's rules run through the request's planned program, the operators running each node over
//! the file's facts and tree in place; and the plan's root kinds, written as texts, mapped to the open file's own kind numbers,
//! so each node reading the tree starts from the tree nodes of its kinds, and a node with no root kind from every tree node.

use crate::application::file_dag::file_pass::types::{FileRoots, NodeReads, PassSource, PassInputs, PassProgram, PassResources, RootKindNodes, TreeStart};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::fused_program::service::{assemble, logical, root_kinds, serving};
use ryo_execution_domain::domain::fused_program::types::fused_program::value::FusedProgram;
use ryo_execution_domain::domain::fused_program::types::program_inputs::value::ProgramInputs;
use ryo_execution_domain::domain::fused_program::types::{RuleBindings, RuleInput, RuleObservations, Serving};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::trust::service::{needs_of, physical_reads};
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_language::domain::elements::service::{kind_names, positions};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::of_kind;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::{Candidates, FactRows, OpenFileBytes, PlanInputs, PlanNodeId, RenderOutcomes, RenderRequests, Resources};
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
use std::sync::Arc;

pub(crate) mod captures;
pub(crate) mod nodes;
pub(crate) mod rules;

/// The roots of `program`, before any file.
pub(crate) fn file_roots(program: &FusedProgram) -> FileRoots {
    let kinds: Vec<RootKindNodes> = root_kinds(program)
        .map(|root| RootKindNodes {
            kind: root.kind.clone(),
            nodes: root.nodes.to_vec(),
        })
        .collect();
    let count = kinds
        .iter()
        .flat_map(|root| &root.nodes)
        .map(|node| usize::try_from(node.0).unwrap_or(usize::MAX).saturating_add(1))
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
pub(crate) fn fill_roots(roots: &mut FileRoots, elements: &Elements, texts: &Texts<'_>, file: FileId) {
    roots.all.clear();
    roots.all.extend(positions(elements).map(|at| node_id(file, at)));
    for list in &mut roots.lists {
        list.clear();
    }
    let names = kind_names(elements);
    for root in &roots.kinds {
        let numbered = names
            .iter()
            .position(|name| symbol_text(texts, *name).is_some_and(|found| found.0 == &*root.kind.0));
        // A kind the file does not name, or a number past the kinds a file can number, has no tree node in it.
        let Some(Ok(kind)) = numbered.map(u16::try_from) else {
            continue;
        };
        for node in &root.nodes {
            if let Some(list) = roots.lists.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) {
                list.extend(of_kind(elements, KindBits(kind)).iter().map(|at| node_id(file, *at)));
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
    })
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
        absorb(&mut each.needs, trusted.get(at).map_or(&[][..], Vec::as_slice));
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = input
                && let Some(earlier) = reads.get(id.0 as usize)
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
        Value::Symbol(intern_derived(self.symbols, ByteSlice(self.source.0), NameText(text.0)))
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

/// Refuses the project phase and the observation stage of a request whose file pass has a rule no stage serves yet, with the
/// latest stage among them; a pass that could not be made refuses with its own fault. Branch only: until the fourth stage
/// serves them.
///
/// # Errors
///
/// `NotServedUntil` with the latest unserved stage, or the fault the pass's program was refused with.
pub(crate) fn served(pass: Option<&Result<PassProgram, OperatingError>>) -> Result<(), OperatingError> {
    let Some(pass) = pass else {
        return Ok(());
    };
    let pass = pass.as_ref().map_err(Clone::clone)?;
    let mut latest = None;
    let mut rule = RuleIndex(0);
    while let Ok(served) = serving(&pass.program, rule) {
        if let Serving::NotServedUntil(stage) = served {
            latest = latest.max(Some(stage));
        }
        rule = RuleIndex(rule.0.saturating_add(1));
    }
    latest.map_or(Ok(()), |stage| Err(OperatingError::NotServedUntil(stage)))
}

