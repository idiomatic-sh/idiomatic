//! Finite request descriptors; no executable Plan or mutable worker pool survives.
use crate::application::file_dag::derived::constants as activation;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{executable_program::value::ExecutableProgram, InputSource},
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::Relation;

use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
pub(in crate::application::file_dag) struct RequestDerived {
    relations: Box<[Box<[Relation]>]>,
    graph_rules: std::collections::HashSet<RuleIndex>,
}
impl RequestDerived {
    pub(in crate::application::file_dag) fn compile(
        program: &ExecutableProgram,
    ) -> Result<Self, OperatingError> {
        let mut relations = Vec::with_capacity(program.rules().len());
        let mut graph_rules = std::collections::HashSet::new();
        for (rule, mapping) in program.rules().iter().enumerate() {
            let mut reads = Vec::new();
            let mut graph = false;
            for origin in mapping.original_steps.iter() {
                let Ok(origin) = origin else {
                    continue;
                };
                let region_id = *program
                    .logical_regions()
                    .get(origin.logical.0)
                    .ok_or(OperatingError::NotRun)?;
                let region = program
                    .regions()
                    .get(region_id.0)
                    .ok_or(OperatingError::NotRun)?;
                if region.phase() != ryo_values::domain::plan_parts::types::StepScope::PerFile {
                    continue;
                }

                for source in region.inputs().iter() {
                    if let InputSource::Relation(relation) = source {
                        note_read(&mut reads, *relation);
                    }
                }
                let instruction = region
                    .instructions()
                    .iter()
                    .find(|instruction| instruction.logical() == origin.logical)
                    .ok_or(OperatingError::NotRun)?;
                for group in instruction.contract().derived.slots.iter().flatten() {
                    for use_ in std::iter::once(&group.primary).chain(group.secondary.iter()) {
                        for relation in use_.relations.iter().flatten() {
                            note_read(&mut reads, *relation);
                        }
                    }
                }
                graph |= matches!(
                    instruction.operation(),
                    ryo_values::domain::plans::types::Operation::Axis { .. }
                        | ryo_values::domain::plans::types::Operation::Join { reach: Some(_), .. }
                );
            }
            relations.push(reads.into_boxed_slice());
            if graph {
                graph_rules.insert(RuleIndex(rule));
            }
        }
        Ok(Self {
            relations: relations.into_boxed_slice(),
            graph_rules,
        })
    }
    pub(in crate::application::file_dag) fn relations_for(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Relation], OperatingError> {
        tracing::trace!(target: activation::TARGET, event = activation::RELATION_RULE, rule = rule.0);
        self.relations
            .get(rule.0)
            .map(AsRef::as_ref)
            .ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag) fn graph_for(
        &self,
        rule: RuleIndex,
    ) -> Result<GraphAdmission, OperatingError> {
        tracing::trace!(target: activation::TARGET, event = activation::GRAPH_RULE, rule = rule.0);
        let eligible = self.graph_rules.contains(&rule);
        if eligible {
            tracing::trace!(target: activation::TARGET, event = activation::GRAPH_HIT);
        }
        if rule.0 >= self.relations.len() {
            return Err(OperatingError::NotRun);
        }
        Ok(if eligible {
            GraphAdmission::Enabled
        } else {
            GraphAdmission::Disabled
        })
    }
}

/// A relation a rule reads is recorded once, in the order it is first read.
fn note_read(reads: &mut Vec<Relation>, relation: Relation) {
    if !reads.contains(&relation) {
        reads.push(relation);
    }
}
