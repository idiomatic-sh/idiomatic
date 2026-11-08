//! Delivery of the canonical publication of one file.
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::constants::FILE_STREAM_CONSUMER;
use crate::application::file_dag::program::facade_runtime::file_execution::FileRun;
use crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs;
use crate::domain::rule_intent::types::DiagnosticDelivery;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use ryo_execution_store::application::file_dag::service::ScopePartials;
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
    ) -> Result<crate::domain::row_emission::types::RowEmission, OperatingError>
    {
        if self.declared(rule)?.delivery != DiagnosticDelivery::Forwarded {
            return Err(OperatingError::NotRun);
        }
        let emission = self.deliver_rows(rule, read, symbols, each);
        self.walk.delivered(self.program, rule);
        emission
    }
    fn deliver_rows<'facts>(
        &mut self,
        rule: RuleIndex,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<crate::domain::row_emission::types::RowEmission, OperatingError>
    {
        let id = *self
            .program
            .program()
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?
            .emit
            .as_ref()
            .map_err(Clone::clone)?;
        let phase = self.program.program().regions()[id.0].phase();
        if phase != ryo_values::domain::plan_parts::types::StepScope::PerFile {
            return Ok(
                crate::domain::row_emission::types::RowEmission::Deferred {
                    phase,
                },
            );
        }
        self.prepare_walk(read, symbols)?;
        let _rule_origin = self.routes.profile().rule(rule.0, self.program.rule_id(rule)?);
        let _consumer_origin =
            self.routes.profile().consumer(FILE_STREAM_CONSUMER);
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
                crate::domain::row_emission::types::RowEmission::Complete
            }
            affected @ (crate::domain::completion::types::CoverageState::Partial { .. }
            | crate::domain::completion::types::CoverageState::Inapplicable {
                ..
            }) => crate::domain::row_emission::types::RowEmission::Partial {
                affected: crate::domain::frontier_coverage::service::affected_files(&affected)
                    .unwrap_or_default(),
            },
        })
    }
}
