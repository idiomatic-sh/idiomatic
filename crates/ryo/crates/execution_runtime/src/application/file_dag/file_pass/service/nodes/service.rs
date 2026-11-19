//! A rule's node in the open file: run by the operators, its rows answered with their coverage, each engine use of a node done
//! when its rule's turn ends, and every node's rows released when the file ends.

use crate::application::file_dag::file_pass::service::answering::service::{
    held, ran_answering, ran_rows,
};
use crate::application::file_dag::file_pass::service::step_sets;
use crate::application::file_dag::file_pass::types::{
    NodeAnswer, NodeInFile, NodeMove, PassFile, PassProgram, PassResources,
};
use crate::application::file_dag::frontiers::service::reason;
use crate::application::file_dag::program::rule_outcome::service::classify;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::completion::service::partial_any;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::completion::types::RegionAdmission;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::refusals::service::layout;
use ryo_execution_domain::domain::executable_program::service::{
    consumers, physical, rule as fused_rule,
};
use ryo_execution_domain::domain::frontier_coverage::service::affected_files;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::output_coverage::service::output_coverage;
use ryo_execution_domain::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::fact_rows::value::FactRows;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanInputs, PlanNodeId, Resources};
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_relations::domain::relations::service::kinds as relation_kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::lineage::service::{of_step, one_file};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::{Input, StepId, StepScope};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::rows::service::reset;
use ryo_values::domain::rows::types::Table;

/// What a rule's node, the node of its plan's step `step`, answers in the open file: its rows, complete or partial as its
/// reads are; empty and partial when a read it trusts the absence of is partial; or the fault it raised.
pub(crate) fn node_outcome(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    node: Result<PlanNodeId, OperatingError>,
    step: StepId,
) -> RootOutcome {
    match node_rows(at, symbols, rule, node, step) {
        Ok(NodeAnswer { table, coverage }) => match affected_files(&coverage) {
            None => RootOutcome::Complete(table),
            Some(affected) => RootOutcome::Partial { table, affected },
        },
        Err(error) => classify(error),
    }
}

/// A rule's node, the node of its plan's step `step`, its rows in the open file and their coverage.
pub(crate) fn node_rows(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    rule: RuleIndex,
    node: Result<PlanNodeId, OperatingError>,
    step: StepId,
) -> Result<NodeAnswer, OperatingError> {
    let pass = at.pass;
    // A step the program does not hold reads nothing.
    let reads = step_sets(pass, rule, step);
    let uses = reads.map_or(&[][..], |reads| &*reads.uses);
    let needs = reads.map_or(&[][..], |reads| &*reads.needs);
    let node = match node {
        Ok(node) => node,
        Err(error) => {
            // A step's schema fault is answered once its sources are admitted: a relation the facts refuse stops the file first.
            for read in uses {
                if native_form(read.relation()).is_none()
                    && matches!(at.inputs.facts(read.relation()), FactRows::Refused)
                {
                    break;
                }
            }
            return Err(error);
        }
    };
    let node = crate::application::file_dag::file_pass::service::selection::service::selected_step(
        at.pass,
        at.condition,
        rule,
        step,
        node,
    )?;
    let completion = CompletionPolicy::file_inputs(rule, at.file, at.evaluation, at.publication);
    if completion.region_admission(needs) == RegionAdmission::Blocked {
        let mut table = Table::default();
        reset(&mut table, layout(&at.pass.program, node)?);
        return Ok(NodeAnswer {
            table,
            coverage: completion.blocked_coverage(),
        });
    }
    let pass_program = at.pass;
    ran_answering(&pass_program.program, node, &mut |each| {
        file_step(at, symbols, each)
    })?;
    let table = ran_rows(&pass_program.program, &at.buffers, node)?;
    // What no read of the step lacks, no operator on the way can lack.
    let coverage = match completion.coverage(uses) {
        CoverageState::Complete => CoverageState::Complete,
        CoverageState::Partial { .. } | CoverageState::Inapplicable { .. } => {
            step_coverage(pass, &completion, rule, step)?
        }
    };
    Ok(NodeAnswer { table, coverage })
}

/// What one step's rows lack, with their cell kinds and where they come from.
pub(crate) type StepCoverage = (Vec<CellKind>, CoverageState, Lineage);

/// The coverage of step `step` of `rule`'s plan in the open file: each relation it reads covered as the file's partiality says
/// of it, and each step on the way passing on what its inputs lack as its operator does (`output_coverage`), so a missing row
/// that could change another file's answer makes the repository partial.
fn step_coverage(
    pass: &PassProgram,
    completion: &CompletionPolicy,
    rule: RuleIndex,
    step: StepId,
) -> Result<CoverageState, OperatingError> {
    step_coverage_with(pass, rule, step, |input, _| match input {
        Input::Relation(relation) => {
            let kinds = relation_kinds(relation);
            Some(Ok((
                kinds.to_vec(),
                completion.relation_coverage(relation),
                one_file(kinds),
            )))
        }
        Input::Step(_) => None,
        Input::BoundObservation(_) => Some(Err(OperatingError::NotRun)),
    })
    .map(|(_, coverage, _)| coverage)
}

/// `rule`'s step `step` as the steps after the files read it once the files handed it over: its cell kinds, `coverage`, and
/// each row from the one file it was made in.
pub(crate) fn handed_step(
    pass: &PassProgram,
    rule: RuleIndex,
    step: StepId,
    coverage: CoverageState,
) -> StepCoverage {
    let node = match fused_rule(&pass.program, rule) {
        Ok(nodes) => nodes
            .steps
            .get(usize::try_from(step.0).unwrap_or(usize::MAX))
            .cloned(),
        Err(_) => None,
    };
    let kinds = match node {
        Some(Ok(node)) => {
            layout(&pass.program, node).map_or_else(|_| Vec::new(), <[CellKind]>::to_vec)
        }
        Some(Err(_)) | None => Vec::new(),
    };
    let lineage = one_file(&kinds);
    (kinds, coverage, lineage)
}

/// The coverage of step `step` of `rule`'s plan with its cell kinds and where its rows come from, each input `leaf` answers
/// covered as it says, given the scope of the step an input names or of the step reading it, and every other step passing on
/// what its inputs lack as its operator does.
pub(crate) fn step_coverage_with(
    pass: &PassProgram,
    rule: RuleIndex,
    step: StepId,
    leaf: impl FnMut(Input, StepScope) -> Option<Result<StepCoverage, OperatingError>>,
) -> Result<StepCoverage, OperatingError> {
    step_coverage_seen(pass, rule, step, leaf, |_, _| {})
}

/// [`step_coverage_with`], each step on the way shown to `seen` with its operands' coverage, in plan order.
pub(crate) fn step_coverage_seen(
    pass: &PassProgram,
    rule: RuleIndex,
    step: StepId,
    mut leaf: impl FnMut(Input, StepScope) -> Option<Result<StepCoverage, OperatingError>>,
    mut seen: impl FnMut(StepId, &[StepCoverage]),
) -> Result<StepCoverage, OperatingError> {
    let plan = pass.plans.get(rule.0).ok_or(OperatingError::NotRun)?;
    let nodes = fused_rule(&pass.program, rule)?;
    let last = usize::try_from(step.0).unwrap_or(usize::MAX);
    let mut merged: Vec<StepCoverage> = Vec::with_capacity(last.saturating_add(1));
    for (place, each) in plan.steps.iter().enumerate().take(last.saturating_add(1)) {
        let operands = named(&inputs_of(&each.operation))
            .into_iter()
            .map(|input| {
                let scope = match *input {
                    Input::Step(id) => plan
                        .steps
                        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                        .map_or(each.scope, |read| read.scope),
                    Input::Relation(_) | Input::BoundObservation(_) => each.scope,
                };
                match leaf(*input, scope) {
                    Some(answer) => answer,
                    None => match *input {
                        Input::Step(id) => merged
                            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                            .cloned()
                            .ok_or(OperatingError::NotRun),
                        Input::Relation(_) | Input::BoundObservation(_) => {
                            Err(OperatingError::NotRun)
                        }
                    },
                }
            })
            .collect::<Result<Vec<StepCoverage>, OperatingError>>()?;
        seen(StepId(u32::try_from(place).unwrap_or(u32::MAX)), &operands);
        let output = match nodes.steps.get(place) {
            Some(Ok(node)) => layout(&pass.program, *node).map(<[CellKind]>::to_vec),
            Some(Err(error)) => Err(error.clone()),
            None => Err(OperatingError::NotRun),
        };
        let (kinds, coverage) = match output {
            Ok(kinds) => {
                let inputs: Vec<InputCoverage<'_>> = operands
                    .iter()
                    .enumerate()
                    .map(|(ordinal, (kinds, coverage, lineage))| InputCoverage {
                        ordinal: OperandOrdinal(ordinal),
                        kinds,
                        coverage: coverage.clone(),
                        lineage: lineage.clone(),
                    })
                    .collect();
                let coverage = output_coverage(&each.operation, &kinds, &inputs)?;
                (kinds, coverage)
            }
            // No lineage is certified from a refused schema.
            Err(_) => (Vec::new(), partial_any(vec![None])),
        };
        let lineages: Vec<Lineage> = operands
            .into_iter()
            .map(|(_, _, lineage)| lineage)
            .collect();
        let lineage = of_step(&each.operation, &lineages, &kinds);
        merged.push((kinds, coverage, lineage));
    }
    merged.into_iter().nth(last).ok_or(OperatingError::NotRun)
}

/// Runs `node` for the open file through the operators.
pub(crate) fn ran_node(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    node: PlanNodeId,
) -> Result<(), OperatingError> {
    let mut resources = PassResources {
        symbols,
        source: at.source,
        paths: at.paths,
        elements: at.inputs.lookup.tree(),
        semantic: at.semantic,
        refusals: &mut at.refusals,
    };
    if at.request.observer.is_none() && at.inputs.lookup.partials().is_empty() {
        ryo_operators::domain::physical_plan::service::bind_requested_entity_sources(&mut at.buffers, physical(&at.pass.program), &resources.texts(), |relation| {
            match at.inputs.lookup.columns(relation) {
                Ok(view) => Some(view),
                Err(_) => None,
            }
        });
    }
    run_node(at.pass, node, &mut at.buffers, &at.inputs, &mut resources).map_err(|fault| {
        if let PortableOperatorFault::RenderRefused { refusal } = fault {
            // A refused rendering answers the engine's reason for the node it came from, whichever rule ran that node.
            at.refusals
                .get(usize::try_from(refusal.0).unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_else(|| portable_fault(fault))
        } else if let PortableOperatorFault::Withheld(token) = fault {
            // A bound observation the file withheld answers the reason it was withheld, as after the files.
            reason(&at.inputs.withholdings, token)
        } else {
            portable_fault(fault)
        }
    })
}

/// The one node step of the request's program: `node` run by the operators over `inputs` and `resources`, in a file or after
/// the files, its rows held in `buffers`.
///
/// # Errors
///
/// The operators' fault, raised where the node's first fault is in the interpreter's order.
pub(crate) fn run_node(
    pass: &PassProgram,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
    inputs: &dyn PlanInputs,
    resources: &mut dyn Resources,
) -> Result<(), PortableOperatorFault> {
    pass.operators
        .run(physical(&pass.program), node, buffers, inputs, resources)
}

/// Starts the open file: a node no rule that runs on it can run (a step of a rule that does not run on it, or a step after the
/// files) runs nothing in it, so each read it makes of an earlier node is done before any rule runs, and that node goes at
/// its last consumer that does run. `live` is the pass's scratch, refilled per file.
pub(crate) fn unread_unrun(
    at: &mut PassFile<'_, '_, '_, '_>,
    intents: &[FileRuleIntent],
    live: &mut Vec<NodeInFile>,
) {
    let pass = at.pass;
    live.clear();
    let mut node = PlanNodeId(0);
    while consumers(&pass.program, node).is_ok() {
        live.push(NodeInFile::Unrunnable);
        node = PlanNodeId(node.0.saturating_add(1));
    }
    for (rule, intent) in intents.iter().enumerate() {
        if !matches!(intent, FileRuleIntent::Run { .. }) {
            continue;
        }
        let (Ok(nodes), Some(plan)) = (
            crate::application::file_dag::file_pass::service::selection::service::file_rule(
                pass,
                at.condition,
                RuleIndex(rule),
            ),
            pass.plans.get(rule),
        ) else {
            continue;
        };
        // A rule's step reads only its earlier steps, and a step in each file never reads one after the files: the nodes
        // marked here read only nodes marked here.
        for (step, node) in plan.steps.iter().zip(&nodes.steps) {
            if step.scope == StepScope::PerFile
                && let Ok(node) = node
                && let Some(alive) = live.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                *alive = NodeInFile::Runnable;
            }
        }
    }
    let plan = physical(&pass.program);
    for (place, alive) in live.iter().enumerate() {
        if *alive == NodeInFile::Unrunnable {
            at.buffers
                .unread(plan, PlanNodeId(u32::try_from(place).unwrap_or(u32::MAX)));
        }
    }
}

/// Ends `rule`'s turn in the open file, whether it ran or not: each of its engine uses, the node it emits and each node it holds
/// for a frontier or a named output, is done, so a node no reader still needs gives its rows back now, not at the file's end.
pub(crate) fn rule_done(at: &mut PassFile<'_, '_, '_, '_>, rule: RuleIndex) {
    crate::application::file_dag::file_pass::service::selection::service::finish_rule(at, rule);
}

/// Ends the file: every node's rows go back to the buffers, those of a rule the file stopped at among them.
pub(crate) fn released(at: &mut PassFile<'_, '_, '_, '_>) {
    at.buffers.ended();
}

/// One move of [`ran_answering`] in the open file: a node run; a relation's facts admitted, a refused one stopping the file (the
/// lookup keeps the refusal, which the file answers first); a frontier entry or a bound observation held, a withheld one
/// answering why.
pub(crate) fn file_step(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    step: NodeMove,
) -> Result<(), OperatingError> {
    match step {
        NodeMove::Run(node) => ran_node(at, symbols, node),
        NodeMove::Empty(node) => {
            at.buffers.empty_as(node, &[]);
            Ok(())
        }
        NodeMove::Read(PlanInput::Relation(relation)) => {
            if native_form(relation).is_none()
                && matches!(at.inputs.facts(relation), FactRows::Refused)
            {
                return Err(OperatingError::NotRead);
            }
            Ok(())
        }
        NodeMove::Read(PlanInput::Frontier(place)) => {
            held(&at.inputs.withholdings, at.inputs.frontier(place))
        }
        NodeMove::Read(PlanInput::Observation(place)) => {
            held(&at.inputs.withholdings, at.inputs.observation(place))
        }
        NodeMove::Read(PlanInput::Node(_)) => Ok(()),
    }
}
