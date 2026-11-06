//! Capture and commit of one file's observation frontiers.
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::facade_runtime::file_execution::FileRun;
use crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs;
use crate::application::file_dag::program::facade_runtime::source_admission::table_of;
use crate::application::file_dag::program::facade_runtime::types::{DeclaredIntent, FrontierCommit};
use crate::application::file_dag::program::protocol::types::CaptureIntent;
use crate::application::file_dag::program::protocol::types::CaptureScope;
use crate::application::file_dag::program::protocol::types::CapturedFrontier;
use crate::application::file_dag::program::protocol::types::FrontierCapture;
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::stored_facts::types::Bucket;
use std::rc::Rc;

impl FileRun<'_> {
    pub(crate) fn capture_frontiers<'facts>(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        capture: CaptureScope<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        if self.declared(rule)?.capture == CaptureIntent::Disabled {
            return Err(OperatingError::NotRun);
        }
        let captured = self.deliver_frontiers(rule, first, read, symbols, capture);
        self.walk.delivered(self.program, rule);
        captured
    }
    fn deliver_frontiers<'facts>(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        capture: CaptureScope<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        let DeclaredIntent {
            capture: capture_intent,
            ..
        } = self.declared(rule)?;
        self.prepare_walk(read, symbols)?;
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let mut captures = Vec::new();
        let sources = self
            .program
            .program()
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?
            .frontier
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let kernels = crate::application::file_dag::program::kernel_session::KernelSession::file(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            &self.walk,
        );
        let mut inputs = SourceInputs {
            execution: &self.execution,
            state: self.execution.file_state(),
            completion,
            owner: Rc::clone(&self.owner),
            read,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        let CaptureScope {
            file,
            rule_digest,
            worker,
            retained,
            words,
        } = capture;
        let crate::domain::finish::types::Retained { arena, places: _ } = retained;
        for (index, source) in sources.into_iter().enumerate() {
            if capture_intent == CaptureIntent::Observations
                && self.program.frontier_purpose(rule, InputNumber(index))? != crate::domain::executable_program::file_demand::types::FrontierPurpose::Observation
            {
                continue;
            }
            if let Ok(source) = source.as_ref()
                && matches!(
                    self.program.program().frontier_phase(*source),
                    Ok(ryo_values::domain::plans::types::StepScope::CrossFile)
                )
            {
                continue;
            }
            let input = InputNumber(index);
            let outcome = (|| {
                let admitted = inputs.admit_source(
                    source?,
                    &Partition::File(self.generation.clone()),
                    context,
                )?;
                let coverage = crate::domain::completion::service::file_frontier_coverage(
                    admitted.coverage()?,
                    file,
                )?;
                let mut table = table_of(&self.owner, &admitted, read)?;
                let frame_places = crate::domain::finish::service::escape_frame(
                    &mut table,
                    crate::domain::finish::types::Leaving {
                        symbols: context.symbols.as_ref(),
                        source: context.source,
                        file,
                        worker,
                        text: crate::domain::finish::types::Leaves::WithText,
                    },
                    crate::domain::finish::types::Arena(&mut *arena.0),
                )
                .map_err(|reason| OperatingError::ObservationIncomplete {
                    reason: reason.to_string(),
                })?;
                words.0.clear();
                ryo_values::domain::table_words::service::encode_table(
                    &table,
                    &frame_places,
                    words,
                );
                let packed = ryo_values::domain::table_words::service::pack(
                    ryo_values::domain::table_words::types::WordSlice(&words.0),
                );
                Ok(CapturedFrontier {
                    input,
                    bucket: ryo_values::domain::stored_facts::service::frontier_bucket(
                        first, input,
                    ),
                    proof: ryo_values::domain::records::types::FrontierProof {
                        rule: u32::try_from(rule.0).map_err(|_| OperatingError::ShapeMismatch)?,
                        input: u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                        rule_digest,
                        coverage,
                    },
                    words: packed,
                })
            })();
            captures.push(FrontierCapture { input, outcome });
        }
        Ok(captures)
    }
    pub(crate) fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: InputNumber,
    ) -> Result<(), OperatingError> {
        let DeclaredIntent { capture, .. } = self.declared(rule)?;
        if capture == CaptureIntent::Disabled
            || (capture == CaptureIntent::Observations
                && self.program.frontier_purpose(rule, input)? != crate::domain::executable_program::file_demand::types::FrontierPurpose::Observation)
            || input.0 >= self.program.frontiers(rule)?.len()
        {
            return Err(OperatingError::NotRun);
        }
        for &output in self.program.file_frontier_outputs(rule, input)? {
            self.roots
                .get_mut(&rule)
                .and_then(|outputs| outputs.get_mut(output.0 as usize))
                .ok_or(OperatingError::NotRun)?
                .commit = FrontierCommit::Committed;
        }
        Ok(())
    }
}
