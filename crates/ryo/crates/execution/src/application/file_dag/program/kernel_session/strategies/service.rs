//! Ordered-existence, selected and column strategies of a kernel session.
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use crate::{
    application::file_dag::workspace::InvocationWorkspace,
    domain::{
        operating::{
            errors::OperatingError,
            types::{Emit, OperatorContext},
        },
        table_model::{service::count_of, types::Rows},
    },
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use std::rc::Rc;

impl super::super::KernelSession<'_, '_> {
    pub(in crate::application::file_dag::program::kernel_session) fn ordered_session(
        &self,
        shape: &crate::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &crate::application::file_dag::program::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::StrategyOutcome,
        InvocationFailure,
    > {
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
                BorrowedRows::from(&rows),
            )
            .map_err(InvocationFailure::Semantic)?
        else {
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
        };
        let workspace = scratch.workspace.get_or_insert_with(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract().as_ref(),
            )
        });
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit.semantics(),
            failure: &failure,
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                native_join: false,
                source_len: context.source.0.len(),
            },
        };
        let result = crate::application::file_dag::ordered_existence::evaluate(
            &semantics,
            shape,
            &number,
            BorrowedRows::from(&rows),
            context,
            &mut scratch.ordered_row,
            &mut scratch.ordered,
            &workspace.work,
            output,
        );
        failure
            .into_inner()
            .map_or(result, Err)
            .map_err(InvocationFailure::Semantic)?;
        Ok(crate::application::file_dag::program::runtime::types::StrategyOutcome::Executed)
    }
    pub(in crate::application::file_dag::program::kernel_session) fn selected_session(
        &self,
        shape: &crate::domain::executable_program::types::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    > {
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
        let input = permit
            .selection_input(shape)
            .map_err(InvocationFailure::Semantic)?;
        let rows = input
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let workspace = scratch.workspace.get_or_insert_with(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract().as_ref(),
            )
        });
        crate::application::file_dag::program::selection::acquire(
            self.prepared,
            permit,
            shape,
            &rows,
            &self.owner,
            context,
            &workspace.work,
            &self.walk,
            self.kernels,
        )
        .map(|published| {
            published.map_or(
                crate::application::file_dag::program::runtime::types::Attempt::Declined,
                crate::application::file_dag::program::runtime::types::Attempt::Produced,
            )
        })
        .map_err(InvocationFailure::Semantic)
    }
    pub(in crate::application::file_dag::program::kernel_session) fn column_session(
        &self,
        shape: &crate::domain::executable_program::types::ColumnAlternative,
        permit: &KernelPermit<'_>,
        left: &crate::application::file_dag::program::admission::AdmittedInput,
        right: &crate::application::file_dag::program::admission::AdmittedInput,
        context: &mut OperatorContext<'_>,
        _scratch: &mut KernelScratch,
        counts: &crate::application::file_dag::program::runtime::types::ColumnCounters,
        rows: &mut ryo_values::domain::rows::types::Table,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::StrategyOutcome,
        InvocationFailure,
    > {
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
        let chain = shape.chain();
        let Some(index_plan) = &chain.index else {
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
        };
        let left_publication = left
            .publication(&self.owner)
            .map_err(InvocationFailure::Semantic)?;
        let right_publication = right
            .publication(&self.owner)
            .map_err(InvocationFailure::Semantic)?;
        let left_rows = left
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let right_rows = right
            .rows(&self.owner, self.read)
            .map_err(InvocationFailure::Semantic)?;
        let Rows::Stored(right_source) = &right_rows else {
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
        };
        use crate::domain::table_model::collections::representations::types::PhysicalShape;
        if self
            .owner
            .inspect(left_publication.as_ref(), BorrowedRows::from(&left_rows))
            .map_err(InvocationFailure::Semantic)?
            != PhysicalShape::Complete
            || self
                .owner
                .inspect(right_publication.as_ref(), BorrowedRows::from(&right_rows))
                .map_err(InvocationFailure::Semantic)?
                != PhysicalShape::Complete
            || index_plan
                .schemas
                .first()
                .is_none_or(|schema| schema.as_slice() != right_source.kinds)
        {
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            );
        }
        crate::domain::derived_views::service::validate(
            ryo_derived::domain::contract::types::DerivedKind::Equality,
            index_plan,
        )
        .map_err(InvocationFailure::Semantic)?;
        if count_of(&left_rows).0 == 0 {
            ryo_values::domain::rows::service::reset(
                rows,
                &permit.output_kinds().map_err(InvocationFailure::Semantic)?,
            );
            return Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Executed,
            );
        }
        let acquired = self
            .owner
            .stored_equality(index_plan, chain.right, *right_source)
            .map_err(InvocationFailure::Semantic)?;
        let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
            acquired.view().map_err(InvocationFailure::Semantic)?
        else {
            return Err(InvocationFailure::Semantic(OperatingError::WrongOperation));
        };
        let failure = std::sync::OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit.semantics(),
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                native_join: false,
                source_len: context.source.0.len(),
            },
            failure: &failure,
        };
        let mut measured = crate::domain::column_kernel::types::KernelCounts::default();
        let result = crate::domain::column_kernel::service::execute_with_index(
            &semantics,
            chain,
            &left_rows,
            &right_rows,
            *right_source,
            index,
            context.symbols.as_ref(),
            ryo_language::domain::symbols::types::ByteSlice(context.source.0),
            rows,
            &permit.output_kinds().map_err(InvocationFailure::Semantic)?,
            &mut measured,
        );
        counts.join.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(measured.covered_rows.0));
        counts.join.found(ryo_values::domain::plan_parts::types::InputNumber(1), crate::domain::operator::contract::types::FoundRows(measured.joined_hits.0));
        counts.filter.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(measured.joined_hits.0));
        counts.project.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(measured.filtered_rows.0));
        if let Some(error) = failure.get() {
            return Err(InvocationFailure::Semantic(error.clone()));
        }
        match result.map_err(InvocationFailure::Semantic)? {
            Some(_) => Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Executed,
            ),
            None => Ok(
                crate::application::file_dag::program::runtime::types::StrategyOutcome::Declined,
            ),
        }
    }
}
