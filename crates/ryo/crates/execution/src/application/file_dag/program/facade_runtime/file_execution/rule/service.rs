//! Evaluation of one rule over the prepared file.
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::constants::{
    FILE_RULE_CONSUMER, SOURCE_FACTS_INCOMPLETE,
};
use crate::application::file_dag::program::facade_runtime::file_execution::FileRun;
use crate::application::file_dag::program::facade_runtime::file_execution::classify;
use crate::application::file_dag::program::facade_runtime::file_execution::outcome_of;
use crate::application::file_dag::program::facade_runtime::file_execution::readiness_of;
use crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs;
use crate::application::file_dag::program::facade_runtime::source_admission::observation_table;
use crate::application::file_dag::program::facade_runtime::types::DeclaredIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::application::file_dag::scope::ScopePartials;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;

impl FileRun<'_> {
    /// One delivery of the rule: whatever its outcome, the walk learns it is done.
    /// A call this entry point refuses (not the declared delivery) is not a delivery.
    pub(crate) fn rule<'facts>(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<RuleOutcome, OperatingError> {
        self.admit_rule(rule, demand)?;
        let outcome = self.deliver_rule(rule, demand, read, symbols);
        self.walk.delivered(self.program, rule);
        outcome
    }
    fn admit_rule(&self, rule: RuleIndex, demand: RuleDemand) -> Result<(), OperatingError> {
        let DeclaredIntent {
            delivery: diagnostic,
            demand: observations,
            ..
        } = self.declared(rule)?;
        let emit = match diagnostic {
            DiagnosticDelivery::Disabled => EmitDemand::Disabled,
            DiagnosticDelivery::Collected => EmitDemand::Enabled,
            DiagnosticDelivery::Forwarded => return Err(OperatingError::NotRun),
        };
        if (RuleDemand { emit, observations }) != demand {
            return Err(OperatingError::NotRun);
        }
        Ok(())
    }
    fn deliver_rule<'facts>(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<RuleOutcome, OperatingError> {
        let metadata = self.program.rule_metadata(rule)?;
        let owns_file_root = (demand.emit == EmitDemand::Enabled
            && metadata.emit_root.as_ref().is_ok_and(|id| {
                self.program.program().regions()[id.0].phase()
                    == ryo_values::domain::plan_parts::types::StepScope::PerFile
            }))
            || (demand.observations == ObservationDemand::Required
                && metadata.observations.iter().any(|root| {
                    root.phase == ryo_values::domain::plan_parts::types::StepScope::PerFile
                }));
        if owns_file_root {
            self.prepare_walk(read, symbols)?;
        }
        let _rule_origin = crate::application::file_dag::route_counts::rule(
            self.routes.profile(),
            rule.0,
            self.program.rule_id(rule)?,
        );
        let _consumer_origin =
            crate::application::file_dag::route_counts::consumer(self.routes.profile(), FILE_RULE_CONSUMER);
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let metadata = self.program.rule_metadata(rule)?;
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let kernels = crate::application::file_dag::program::kernel_session::KernelSession::file(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            &self.walk,
        );
        let mut sources = SourceInputs {
            execution: &self.execution,
            state: self.execution.file_state(),
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id)
                    if self.program.program().regions()[id.0].phase()
                        != ryo_values::domain::plan_parts::types::StepScope::PerFile =>
                {
                    RootOutcome::Deferred {
                        phase: self.program.program().regions()[id.0].phase(),
                    }
                }
                Ok(id) => outcome_of(
                    self.execution.invoke(
                        self.execution.file_state(),
                        *id,
                        &mut sources,
                        &kernels,
                        &completion,
                        context,
                    ),
                    &self.owner,
                    read,
                ),
                Err(error) => RootOutcome::Failed(error.clone()),
            },
        };
        let mut observations = Vec::new();
        let mut published = Vec::new();
        if matches!(demand.observations, ObservationDemand::Required) {
            for (index, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plan_parts::types::StepScope::PerFile {
                    continue;
                }
                let result = if completion.observation_admission(&root.reads)
                    == ryo_execution_domain::domain::completion::types::ObservationAdmission::Ready
                {
                    root.source.clone().and_then(|source| {
                        sources.admit_source(
                            source,
                            &Partition::File(self.generation.clone()),
                            context,
                        )
                    })
                } else {
                    Err(OperatingError::ObservationIncomplete {
                        reason: SOURCE_FACTS_INCOMPLETE.into(),
                    })
                };
                let outcome = match result {
                    Ok(input) => match observation_table(&self.owner, &input, read) {
                        Ok(table) => {
                            published.push((index, input));
                            RootOutcome::Complete(table)
                        }
                        Err(error) => classify(error),
                    },
                    Err(error) => classify(error),
                };
                observations.push(
                    crate::application::file_dag::program::rule_outcome::types::OutputOutcome {
                        output: ryo_values::domain::records::types::ObservationIndex(
                            u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                        ),
                        outcome,
                    },
                );
            }
        }
        drop(sources);
        for (index, input) in published {
            self.roots.get_mut(&rule).ok_or(OperatingError::NotRun)?[index].publication =
                Some(input);
        }
        for crate::application::file_dag::program::rule_outcome::types::OutputOutcome {
            output,
            outcome,
        } in &observations
        {
            self.roots.get_mut(&rule).ok_or(OperatingError::NotRun)?[output.0 as usize].readiness =
                readiness_of(outcome);
        }
        Ok(RuleOutcome {
            emit,
            observations,
            counts: self
                .execution
                .logical_counts(self.execution.file_state(), rule)?,
        })
    }
}
