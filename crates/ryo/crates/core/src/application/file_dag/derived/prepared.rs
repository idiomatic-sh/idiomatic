//! Finite request descriptors; no executable Plan or mutable worker pool survives.
use crate::domain::{
    coverage::types::{Partial, RuleIndex},
    executable_program::types::{ExecutableProgram, InputSource, RegionId},
    operating::errors::OperatingError,
    operator::contract::types::RequestOperators,
    stored_selection::types::{PreparedSelectionViews, Selector},
};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

pub(in crate::application::file_dag) struct SelectorUse {
    pub rule: RuleIndex,
    pub region: RegionId,
    pub selector: Arc<Selector>,
}
pub(in crate::application::file_dag) struct PreparedDerived {
    pub selectors: Box<[SelectorUse]>,
    pub views: Arc<PreparedSelectionViews>,
    pub relations: Box<[Box<[Relation]>]>,
    pub graph_rules: Box<[bool]>,
}
impl PreparedDerived {
    pub(in crate::application::file_dag) fn compile(
        program: &ExecutableProgram,
        prepared: &RequestOperators,
    ) -> Result<Self, OperatingError> {
        if prepared.selection_plans.len() != program.rules.len() {
            return Err(OperatingError::NotRun);
        }
        let mut selectors = Vec::new();
        let mut relations = Vec::with_capacity(program.rules.len());
        let mut graph_rules = Vec::with_capacity(program.rules.len());
        for (rule, (mapping, selected)) in program
            .rules
            .iter()
            .zip(prepared.selection_plans.iter())
            .enumerate()
        {
            if selected.steps.len() != mapping.original_steps.len() {
                return Err(OperatingError::NotRun);
            }
            let mut reads = Vec::new();
            let mut graph = false;
            for (at, origin) in mapping.original_steps.iter().enumerate() {
                let Ok(origin) = origin else {
                    continue;
                };
                let region_id = *program
                    .logical_regions
                    .get(origin.logical.0)
                    .ok_or(OperatingError::NotRun)?;
                let region = program
                    .regions
                    .get(region_id.0)
                    .ok_or(OperatingError::NotRun)?;
                if region.phase != ryo_values::domain::plans::types::StepScope::PerFile {
                    continue;
                }

                for source in region.inputs.iter() {
                    if let InputSource::Relation(relation) = source {
                        if !reads.contains(relation) {
                            reads.push(*relation);
                        }
                    }
                }
                let instruction = region
                    .instructions
                    .iter()
                    .find(|instruction| instruction.logical == origin.logical)
                    .ok_or(OperatingError::NotRun)?;
                for group in instruction
                    .contract
                    .derived
                    .streaming
                    .iter()
                    .chain(instruction.contract.derived.collected.iter())
                    .flatten()
                {
                    for use_ in std::iter::once(&group.primary).chain(group.secondary.iter()) {
                        for relation in use_.relations.iter().flatten() {
                            if !reads.contains(relation) {
                                reads.push(*relation);
                            }
                        }
                    }
                }
                graph |= matches!(
                    instruction.operation,
                    ryo_values::domain::plans::types::Operation::Axis { .. }
                        | ryo_values::domain::plans::types::Operation::Join { reach: Some(_), .. }
                );
                if let Some(selector) = &selected.steps[at] {
                    selectors.push(SelectorUse {
                        rule: RuleIndex(rule),
                        region: region_id,
                        selector: Arc::clone(selector),
                    });
                }
            }
            relations.push(reads.into_boxed_slice());
            graph_rules.push(graph);
        }
        Ok(Self {
            selectors: selectors.into_boxed_slice(),
            views: Arc::clone(&prepared.selection_views),
            relations: relations.into_boxed_slice(),
            graph_rules: graph_rules.into_boxed_slice(),
        })
    }
    pub(in crate::application::file_dag) fn wanted(
        &self,
        demanded: &[bool],
    ) -> Result<Vec<Relation>, OperatingError> {
        if demanded.len() != self.relations.len() {
            return Err(OperatingError::NotRun);
        }
        let mut wanted = Vec::new();
        for (active, relations) in demanded.iter().zip(&self.relations) {
            if *active {
                for relation in relations.iter() {
                    if !wanted.contains(relation) {
                        wanted.push(*relation);
                    }
                }
            }
        }
        Ok(wanted)
    }
}
