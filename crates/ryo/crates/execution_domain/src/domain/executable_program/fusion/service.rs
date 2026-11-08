//! Classify every node of the analysis graph; the compiler fuses only what is `Fusable`.
use super::types::*;
use crate::domain::executable_program::types::Fusibilities;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::ReaderMultiplicity,
    file_program::types::{
        FileProgramNode, GraphObservationRoot, NodeInput, ProgramNodeId, RequestGraph,
        SchemaValidity,
    },
    operator::contract::{
        resources::ResourceProgram,
        types::{Effects, InputMode, ReuseScope},
    },
};
use ryo_values::domain::{
    plan_parts::types::{Input, Operand, RepeatedRows, StepId},
    plans::types::{Operation, Plan},
};

/// The route of every operation kind, chosen with no wildcard: a new kind cannot compile
/// until it is classified here.
pub(crate) fn shape(operation: &Operation) -> Result<(), Ineligible> {
    match operation {
        Operation::Select { kept, .. } => {
            if kept.is_empty() {
                Ok(())
            } else {
                Err(Ineligible::Reshapes)
            }
        }
        Operation::Project { distinct, .. } => {
            if *distinct == RepeatedRows::Kept {
                Ok(())
            } else {
                Err(Ineligible::Reshapes)
            }
        }
        Operation::Rows { .. } | Operation::Hosted { .. } => Err(Ineligible::Source),
        Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Axis { .. }
        | Operation::OrderedDfs { .. } => Err(Ineligible::MultiInput),
        Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. } => Err(Ineligible::Whole),
        Operation::Extend { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => Err(Ineligible::RowFault),
    }
}

/// Literal and column operands mean the same in every rule; a parameter or function binds per rule.
pub(super) fn rule_independent(values: &[Operand]) -> RegionSharing {
    if values
        .iter()
        .all(|value| matches!(value, Operand::Literal(_) | Operand::Column(_)))
    {
        RegionSharing::Shareable
    } else {
        RegionSharing::RuleScoped
    }
}

/// Whether the node's answer depends only on its operation and inputs, so equal nodes of
/// different rules may share one region. Rule-scoped work stays rule-scoped.
pub(crate) fn region_sharing(node: &FileProgramNode, held: Option<Barrier>) -> RegionSharing {
    if node.schema_valid == SchemaValidity::Invalid || !matches!(held, None | Some(Barrier::Handover)) {
        return RegionSharing::RuleScoped;
    }
    match &node.operation {
        Operation::Extend { values, .. } => rule_independent(values),
        _ if node.contract.reuse == ReuseScope::Request => RegionSharing::Shareable,
        _ => RegionSharing::RuleScoped,
    }
}

fn hold(held: &mut [Option<Barrier>], node: ProgramNodeId, barrier: Barrier) {
    if let Some(slot) = held.get_mut(node.0) {
        slot.get_or_insert(barrier);
    }
}

fn root(graph: &RequestGraph, rule: RuleIndex, step: StepId) -> Option<ProgramNodeId> {
    match graph.program_nodes.get(rule.0)?.get(step.0 as usize)? {
        Ok(id) => Some(*id),
        Err(_) => None,
    }
}

const SOLE_READER: ReaderMultiplicity = ReaderMultiplicity(1);

fn fusibility_of(
    node: &FileProgramNode,
    graph: &RequestGraph,
    held: &[Option<Barrier>],
    readers: &[ReaderMultiplicity],
) -> Fusibility {
    if let Err(why) = shape(&node.operation) {
        return Fusibility::Excluded(why);
    }
    let [NodeInput::Node(child)] = node.inputs.as_slice() else {
        return Fusibility::Excluded(Ineligible::Source);
    };
    if node.schema_valid == SchemaValidity::Invalid {
        return Fusibility::Barrier(Barrier::Fault);
    }
    if node.contract.effects != Effects::Pure {
        return Fusibility::Excluded(Ineligible::RowFault);
    }
    if !matches!(node.contract.semantics.shape, InputMode::Row { .. }) {
        return Fusibility::Excluded(Ineligible::Whole);
    }
    if !matches!(node.contract.resources, ResourceProgram::None)
        || node.contract.derived.slots.iter().any(Option::is_some)
    {
        return Fusibility::Barrier(Barrier::Derived);
    }
    let Some(producer) = graph.nodes.get(child.0) else {
        return Fusibility::Barrier(Barrier::Fault);
    };
    if producer.phase != node.phase {
        return Fusibility::Barrier(Barrier::Partition);
    }
    if let Some(barrier) = held[child.0] {
        return Fusibility::Barrier(barrier);
    }
    if readers[child.0] != SOLE_READER {
        return Fusibility::Barrier(Barrier::SharedConsumer);
    }
    // Publish absence-sensitive answers before a continuation can project away the file
    // key needed for completeness.
    if producer.contract.absence_inputs.0 != 0 {
        return Fusibility::Barrier(Barrier::Fault);
    }
    Fusibility::Fusable
}

/// Classify the graph from its operations and the owner-issued identities of the request.
pub(crate) fn classify(plans: &[Plan], graph: &RequestGraph) -> Fusibilities {
    let mut held = vec![None; graph.nodes.len()];
    let mut readers = vec![ReaderMultiplicity::default(); graph.nodes.len()];
    for node in &graph.nodes {
        for input in &node.inputs {
            if let NodeInput::Node(child) = input {
                readers[child.0].0 += 1;
            }
        }
    }
    // Every named publication is a true owner-issued identity, even without an Emit reader.
    if let Some(observations) = &graph.observations {
        for output in observations.outputs.iter().flatten() {
            if let GraphObservationRoot::PerFile(id) = output.root {
                hold(&mut held, id, Barrier::Capture);
            }
        }
    }
    for (rule, plan) in plans.iter().enumerate() {
        let roots = std::iter::once((Input::Step(plan.emit.step), Barrier::Handover))
            .chain(
                plan.frontier
                    .iter()
                    .map(|input| (*input, Barrier::Frontier)),
            )
            .chain(
                plan.observations
                    .iter()
                    .map(|output| (output.source, Barrier::Observation)),
            );
        for (input, barrier) in roots {
            let Input::Step(step) = input else { continue };
            if let Some(id) = root(graph, RuleIndex(rule), step) {
                hold(&mut held, id, barrier);
                readers[id.0].0 += 1;
            }
        }
    }
    for (node, derived) in graph.derived_sources.0.iter().enumerate() {
        if *derived {
            hold(&mut held, ProgramNodeId(node), Barrier::Derived);
        }
    }
    for alternative in graph.ordered_existence.iter().flatten() {
        if let crate::domain::file_program::types::ResolvedAlternative::Eligible(resolved) =
            alternative
        {
            hold(&mut held, resolved.join, Barrier::Lifetime);
            hold(&mut held, resolved.select, Barrier::Lifetime);
        }
    }
    let fusibility = graph
        .nodes
        .iter()
        .map(|node| fusibility_of(node, graph, &held, &readers))
        .collect();
    Fusibilities {
        fusibility,
        readers,
        held,
    }
}
