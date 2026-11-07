//! Delivery of one project rule through its admitted roots.
use crate::application::file_dag::program::facade_runtime::file_execution::classify;
use crate::application::file_dag::program::facade_runtime::file_execution::outcome_of;
use crate::application::file_dag::program::facade_runtime::project_execution::ProjectRun;
use crate::application::file_dag::program::facade_runtime::source_admission::ProjectSources;
use crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs;
use crate::application::file_dag::program::facade_runtime::source_admission::table_of;
use crate::domain::rule_intent::types::EmitDemand;
use crate::domain::rule_intent::types::ObservationDemand;
use crate::application::file_dag::program::protocol::types::ObservedPublication;
use crate::application::file_dag::program::protocol::types::ProjectObservation;
use crate::application::file_dag::program::protocol::types::ProjectRuleOutcome;
use crate::application::file_dag::program::protocol::types::RootOutcome;
use crate::domain::rule_intent::types::RuleDemand;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use std::collections::HashMap;
use std::rc::Rc;

impl ProjectRun<'_> {
    pub(in crate::application::file_dag::program::facade_runtime::project_execution) fn deliver(
        &mut self,
        state: &crate::application::file_dag::program::runtime::service::RuntimeState,
        rule: RuleIndex,
        demand: RuleDemand,
        roots: &[crate::domain::executable_program::types::RegionId],
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError> {
        let completion = self.completion(rule)?;
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let metadata = self.program.rule_metadata(rule)?;
        let unavailable = |_relation| Err(OperatingError::NotRead);
        let kernels = crate::application::file_dag::program::kernel_session::KernelSession::project(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            &unavailable,
            &self.publication.walk,
        );
        let mut absent = HashMap::new();
        let mut sources = SourceInputs {
            execution: self.execution,
            state,
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read: &unavailable,
            observations: &mut absent,
            file_outputs: None,
            program: self.program,
            rule,
            project: Some(ProjectSources {
                program: self.program,
                rule,
                sources: &self.frontier_sources,
                ordinary: &mut self.ordinary,
                refused: &self.refused,
                bound: &mut self.bound,
                cache: &mut self.cache,
                transport: &self.transport,
                publication: self.publication,
            }),
        };
        // One ready cohort contains all actual roots of this compatible population.
        if !roots.is_empty() {
            self.execution.prepare_roots(
                state,
                roots,
                &mut sources,
                &kernels,
                &completion,
                context,
            )?;
        }
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id)
                    if self.program.program().regions()[id.0].phase()
                        != ryo_values::domain::plan_parts::types::StepScope::CrossFile =>
                {
                    RootOutcome::Deferred {
                        phase: self.program.program().regions()[id.0].phase(),
                    }
                }
                Ok(id) => outcome_of(
                    self.execution
                        .invoke(state, *id, &mut sources, &kernels, &completion, context),
                    &self.owner,
                    &unavailable,
                ),
                Err(error) => RootOutcome::Failed(error.clone()),
            },
        };
        let mut observations = Vec::new();
        if matches!(demand.observations, ObservationDemand::Required) {
            for (index, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plan_parts::types::StepScope::CrossFile {
                    continue;
                }
                let result = root
                    .source
                    .clone()
                    .and_then(|source| {
                        self.execution.admit_root(
                            state,
                            rule,
                            source,
                            &mut sources,
                            &kernels,
                            &completion,
                            context,
                        )
                    })
                    .map(|input| input.with_coverage(completion.coverage(&root.reads)));
                observations.push(
                    crate::application::file_dag::program::protocol::types::ProjectOutputOutcome {
                    output: ryo_values::domain::records::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    outcome: match result {
                        Ok(input) => match table_of(&self.owner, &input, &unavailable)
                            .and_then(|table| Ok((table, input.coverage()?)))
                        {
                            Ok((
                                table,
                                crate::domain::completion::types::CoverageState::Complete,
                            )) => ProjectObservation::Complete {
                                table,
                                publication: ObservedPublication::from_admitted(input),
                            },
                            Ok((
                                table,
                                crate::domain::completion::types::CoverageState::Partial { .. }
                                | crate::domain::completion::types::CoverageState::Inapplicable {
                                    ..
                                },
                            )) => ProjectObservation::Partial {
                                table,
                                publication: ObservedPublication::from_admitted(input),
                            },
                            Err(error) => ProjectObservation::Other(classify(error)),
                        },
                        Err(error) => ProjectObservation::Other(classify(error)),
                    },
                });
            }
        }
        Ok(ProjectRuleOutcome {
            emit,
            observations,
            counts: self.execution.logical_counts(state, rule)?,
        })
    }
}
