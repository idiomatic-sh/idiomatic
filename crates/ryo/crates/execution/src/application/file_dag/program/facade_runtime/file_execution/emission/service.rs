//! Delivery of the canonical publication of one file.
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::constants::FILE_STREAM_CONSUMER;
use crate::application::file_dag::program::facade_runtime::file_execution::FileRun;
use crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs;
use crate::application::file_dag::program::protocol::types::DiagnosticDelivery;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;

impl FileRun<'_> {
    /// Deliver the canonical publication; callbacks never participate in evaluation.
    pub(crate) fn emit_rows<'facts>(
        &mut self,
        rule: RuleIndex,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<crate::application::file_dag::program::protocol::types::RowEmission, OperatingError>
    {
        if self.declared(rule)?.0 != DiagnosticDelivery::Forwarded {
            return Err(OperatingError::NotRun);
        }
        let id = *self
            .program
            .program()
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?
            .emit
            .as_ref()
            .map_err(Clone::clone)?;
        let phase = self.program.program().regions()[id.0].phase;
        if phase != ryo_values::domain::plans::types::StepScope::PerFile {
            return Ok(
                crate::application::file_dag::program::protocol::types::RowEmission::Deferred {
                    phase,
                },
            );
        }
        self.prepare_walk(read, symbols)?;
        let _rule_origin = crate::application::file_dag::route_counts::rule(
            rule.0,
            &self.program.rule_id(rule)?.0,
        );
        let _consumer_origin =
            crate::application::file_dag::route_counts::consumer(FILE_STREAM_CONSUMER);
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
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
        struct ForwardRows<'a>(
            &'a mut dyn FnMut(
                &[ryo_relations::domain::relations::types::Value],
            ) -> Result<(), OperatingError>,
        );
        impl crate::domain::operating::types::Emit for ForwardRows<'_> {
            fn row(
                &mut self,
                _context: &mut OperatorContext<'_>,
                row: &[ryo_relations::domain::relations::types::Value],
            ) -> Result<(), OperatingError> {
                (self.0)(row)
            }
        }
        let published = match self.execution.invoke(
            self.execution.file_state(),
            id,
            &mut sources,
            &kernels,
            &completion,
            context,
        )? {
            InvocationOutcome::Published(input) => input,
            InvocationOutcome::Ready | InvocationOutcome::Suspended(_) => {
                return Err(OperatingError::NotRun);
            }
        };
        let coverage = published.coverage()?;
        // Delivery begins only after semantic evaluation has committed. A sink
        // refusal leaves that publication intact for replay without reevaluation.
        sources.replay(&published, context, &mut ForwardRows(each))?;
        Ok(match coverage {
            crate::domain::completion::types::CoverageState::Complete => {
                crate::application::file_dag::program::protocol::types::RowEmission::Complete
            }
            crate::domain::completion::types::CoverageState::Partial { files } => {
                crate::application::file_dag::program::protocol::types::RowEmission::Partial {
                    affected: files,
                }
            }
        })
    }
}
