//! Combine the two checked source-dependent graphs while preserving every original public port.

use super::types::{FileProgramRoutes, RoutedBuild, RuleRoute};
use crate::domain::executable_program::service::logical::service::held;
use crate::domain::executable_program::types::logical_build::value::LogicalBuild;
use crate::domain::executable_program::types::program_inputs::value::ProgramInputs;
use crate::domain::executable_program::types::{RuleInput, RuleNodes};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, PlanInput, PlanInputList, PlanNodeId,
};

/// One original physical declaration which was actually written at the same authored step in the alternate graph.
fn origins(
    rules: &[RuleInput<'_>],
    original: &LogicalBuild<'_>,
    alternate: &LogicalBuild<'_>,
) -> Vec<Option<PlanNodeId>> {
    let alt_nodes = alternate.program.nodes();
    let mut origins = vec![None; alt_nodes.len()];
    for ((defined, ordinary), variant) in rules.iter().zip(&original.rules).zip(&alternate.rules) {
        for ((step, normal), other) in defined
            .plan
            .steps
            .iter()
            .zip(&ordinary.steps)
            .zip(&variant.steps)
        {
            let (Ok(normal), Ok(other)) = (normal, other) else {
                continue;
            };
            let at = other.0 as usize;
            if origins[at].is_none() && std::ptr::eq(alt_nodes[at].operation, &step.operation) {
                origins[at] = Some(*normal);
            }
        }
    }
    origins
}

/// Resolve an alternate node or retain its original refusal.
fn routed(
    node: &Result<PlanNodeId, OperatingError>,
    slots: &[PlanNodeId],
) -> Result<PlanNodeId, OperatingError> {
    let node = node.clone()?;
    slots
        .get(node.0 as usize)
        .copied()
        .ok_or(OperatingError::ShapeMismatch)
}

/// Compile both paths into a single physical graph, with all inputs carrying their original bindings.
///
/// The normal representation owns the public identities. Alternative branches hold only additional
/// declarations when their exact bound inputs or authored operation differ.
///
/// # Errors
///
/// Returns the ordinary shape refusal when either route has an invalid node, or the authored
/// visible ports cannot be paired without changing their failure.
pub fn joined<'a, 'p: 'a>(
    mut normal: LogicalBuild<'p>,
    alternate: LogicalBuild<'p>,
    rules: &'p [RuleInput<'p>],
    storage: &'a mut ProgramInputs,
) -> Result<RoutedBuild<'a>, OperatingError> {
    let origin = origins(rules, &normal, &alternate);
    let old = normal.program.nodes();
    let alt = alternate.program.nodes();
    storage.lists = old.iter().map(|node| node.inputs.0.to_vec()).collect();
    let mut headers: Vec<_> = old
        .iter()
        .map(|node| (node.operation, node.bindings, node.scope))
        .collect();
    let mut slots = Vec::<PlanNodeId>::with_capacity(alt.len());
    for (k, source) in alt.iter().enumerate() {
        let inputs: Vec<_> = source
            .inputs
            .0
            .iter()
            .map(|input| match input {
                PlanInput::Node(id) => slots
                    .get(id.0 as usize)
                    .copied()
                    .map(PlanInput::Node)
                    .ok_or(OperatingError::ShapeMismatch),
                other => Ok(*other),
            })
            .collect::<Result<_, _>>()?;
        let existing = origin[k].filter(|id| {
            old.get(id.0 as usize).is_some_and(|entry| {
                std::ptr::eq(entry.operation, source.operation)
                    && entry.bindings == source.bindings
                    && entry.scope == source.scope
                    && entry.inputs.0 == inputs
            })
        });
        let node = if let Some(original) = existing {
            original
        } else {
            let id = PlanNodeId(
                u32::try_from(headers.len()).map_err(|_| OperatingError::ShapeMismatch)?,
            );
            headers.push((source.operation, source.bindings, source.scope));
            normal.reads.push(inputs.clone());
            storage.lists.push(inputs);
            normal.layouts.push(alternate.layouts[k].clone());
            normal.deferred.push(alternate.deferred[k].clone());
            normal.reach.push(alternate.reach[k]);
            id
        };
        slots.push(node);
    }
    for (kind, numbers) in &alternate.roots {
        let roots = normal.roots.entry(kind.clone()).or_default();
        for number in numbers {
            let chosen = slots[number.0 as usize];
            if !roots.contains(&chosen) {
                roots.push(chosen);
            }
        }
    }
    let mut graph = LogicalProgram::new();
    for ((operation, bindings, scope), inputs) in headers.into_iter().zip(&storage.lists) {
        let _id = graph.node(LogicalNode {
            operation,
            bindings,
            scope,
            inputs: PlanInputList(inputs),
        });
    }
    let mut consumers = held(
        &mut graph,
        rules,
        &normal.rules,
        &storage.lists,
        &mut normal.engine,
    )?;
    let mut routes = FileProgramRoutes::default();
    for (i, expected) in normal.rules.iter().enumerate() {
        let other = &alternate.rules[i];
        let nodes = RuleNodes {
            steps: other.steps.iter().map(|v| routed(v, &slots)).collect(),
            emit: routed(&other.emit, &slots),
            refused: other.refused.clone(),
        };
        if expected.refused != nodes.refused
            || expected.steps.len() != nodes.steps.len()
            || expected
                .steps
                .iter()
                .zip(&nodes.steps)
                .any(|(a, b)| a.as_ref().err() != b.as_ref().err())
        {
            return Err(OperatingError::ShapeMismatch);
        }
        let uses: Vec<_> = alternate.engine[i]
            .iter()
            .map(|id| slots[id.0 as usize])
            .collect();
        if uses.len() != normal.engine[i].len() {
            return Err(OperatingError::ShapeMismatch);
        }
        for (&ordinary, &special) in normal.engine[i].iter().zip(&uses) {
            if ordinary != special {
                graph.hold(special);
                consumers[special.0 as usize].0 = consumers[special.0 as usize].0.saturating_add(1);
                routes.unused_when_proven.push(ordinary);
                routes.unused_when_ordinary.push(special);
            }
        }
        routes.rules.push(if nodes != *expected {
            Some(RuleRoute { nodes, uses })
        } else {
            None
        });
    }
    let result = LogicalBuild {
        program: graph,
        rules: normal.rules,
        consumers,
        engine: normal.engine,
        roots: normal.roots,
        layouts: normal.layouts,
        reads: normal.reads,
        deferred: normal.deferred,
        scopes: normal.scopes,
        reach: normal.reach,
    };
    Ok(RoutedBuild {
        program: result,
        routes,
    })
}
