//! The ordered-existence strategy of a kernel session.
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use ryo_execution_domain::domain::operating::{errors::OperatingError, types::{Emit, OperatorContext}};
use ryo_execution_store::application::file_dag::workspace::InvocationWorkspace;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use std::rc::Rc;

impl super::super::KernelSession<'_, '_> {
    pub(crate) fn ordered_session(
        &self,
        shape: &ryo_execution_domain::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &crate::application::file_dag::program::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure> {
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut rebound = OperatorContext {
            phase: context.phase,
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            matcher: context.matcher,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(permit.descriptor().producer())
                .map_err(InvocationFailure::Semantic)?,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        let context = &mut rebound;
        let publication = input
            .publication(&self.owner)
            .map_err(InvocationFailure::Semantic)?;
        let rows = input
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let Some(number) = self
            .owner
            .admit_number(
                publication,
                shape.source,
                &shape.number,
                RepresentationInput::from(&rows),
            )
            .map_err(InvocationFailure::Semantic)?
        else {
            // The region was classified ordered at compile; a source the number
            // admission refuses is a refusal, never a run of the other path.
            return Err(InvocationFailure::Semantic(OperatingError::NotRun));
        };
        let workspace = scratch.workspace.get_or_insert_with(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract().as_ref(),
            )
        });
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            #[cfg(feature = "diagnostic-row-progress")]
            progress: self.owner.routes(),
            port: self.kernels,
            permit: permit.semantics(),
            failure: &failure,
            binding: ryo_execution_domain::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                source_len: ryo_operators::domain::contract::types::SourceByteLength(context.source.0.len()),
            },
        };
        let result = crate::application::file_dag::ordered_existence::evaluate(
            &semantics,
            shape,
            &number,
            RepresentationInput::from(&rows),
            context,
            &mut scratch.ordered_row,
            &mut scratch.ordered,
            workspace.work(),
            output,
        );
        failure
            .into_inner()
            .map_or(result, Err)
            .map_err(InvocationFailure::Semantic)?;
        Ok(())
    }
}
