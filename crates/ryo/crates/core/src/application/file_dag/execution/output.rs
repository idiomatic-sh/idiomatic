//! One downstream context constructor and invocation-owned output scratch boundary.
use crate::domain::{
    operating::{
        errors::OperatingError,
        types::{Emit, Memo, OperatorContext},
    },
    table_scratch::types::OperatorScratch,
};
use ryo_language::domain::symbols::{service::query_reborrow, types::QuerySymbols};
use ryo_relations::domain::relations::types::Value;

pub(super) struct Headers<'a> {
    pub(super) prepared_matchers:
        Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    pub(super) prepared_kinds: Option<&'a crate::domain::kind_sets::types::PreparedVocabulary>,
    pub(super) bindings: &'a [crate::domain::table_model::types::ParameterBinding],
    pub(super) source: crate::domain::table_texts::types::SourceView<'a>,
    pub(super) paths: &'a [ryo_language::domain::files::types::FilePath],
    pub(super) arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}
impl<'a> Headers<'a> {
    pub(super) fn from_context(context: &OperatorContext<'a>) -> Self {
        Self {
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: context.bindings,
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
        }
    }
    pub(super) fn row(
        &self,
        emit: &mut dyn Emit,
        symbols: &mut QuerySymbols<'_>,
        scratch: &mut OperatorScratch,
        memo: &mut Vec<Memo>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        let mut downstream = OperatorContext {
            semantic: None,
            prepared_matchers: self.prepared_matchers,
            prepared_kinds: self.prepared_kinds,
            bindings: self.bindings,
            symbols: query_reborrow(symbols),
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
            memo,
            scratch,
        };
        emit.row(&mut downstream, row)
    }
}
/// Taken once per engine invocation, never once per row or physical strategy.
pub(in crate::application::file_dag) struct State {
    scratch: OperatorScratch,
    memo: Vec<Memo>,
}
impl State {
    pub(in crate::application::file_dag) fn take(
        scratch: &mut OperatorScratch,
        memo: &mut Vec<Memo>,
    ) -> Self {
        Self {
            scratch: std::mem::take(scratch),
            memo: std::mem::take(memo),
        }
    }
    pub(in crate::application::file_dag) fn restore(
        self,
        scratch: &mut OperatorScratch,
        memo: &mut Vec<Memo>,
    ) {
        *scratch = self.scratch;
        *memo = self.memo;
    }
    pub(in crate::application::file_dag) fn run(
        &mut self,
        emit: &mut dyn Emit,
        run: impl FnOnce(&mut dyn Emit) -> Result<(), OperatingError>,
    ) -> Result<(), OperatingError> {
        let mut output = Guard { emit, state: self };
        run(&mut output)
    }
}
struct Guard<'a> {
    emit: &'a mut dyn Emit,
    state: &'a mut State,
}
impl Emit for Guard<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        Headers::from_context(context).row(
            self.emit,
            &mut context.symbols,
            &mut self.state.scratch,
            &mut self.state.memo,
            row,
        )
    }
}

/// Observe the actual shared collection sink without allocating another table.
#[inline]
pub(super) fn collect(out: &mut ryo_values::domain::rows::types::Table) -> impl Emit + '_ {
    #[cfg(feature = "diagnostic-row-progress")]
    {
        ProgressCollect {
            inner: crate::domain::replay::types::Collect(out),
            origin: super::super::route_counts::capture(),
        }
    }
    #[cfg(not(feature = "diagnostic-row-progress"))]
    {
        crate::domain::replay::types::Collect(out)
    }
}
#[cfg(feature = "diagnostic-row-progress")]
struct ProgressCollect<'a> {
    inner: crate::domain::replay::types::Collect<'a>,
    origin: super::super::route_counts::CapturedOrigin,
}
#[cfg(feature = "diagnostic-row-progress")]
impl Emit for ProgressCollect<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        let _origin = super::super::route_counts::resume(self.origin);
        self.inner.row(context, row)?;
        super::super::route_counts::progress(
            crate::domain::operator::contract::views::ProgressCounter::EmittedRows,
            1,
        );
        Ok(())
    }
}
/// A streamed row is counted at the common callback boundary, not at cells or candidates.
#[cfg(feature = "diagnostic-row-progress")]
pub(super) struct ProgressEmit<'a> {
    pub(super) emit: &'a mut dyn Emit,
    pub(super) origin: super::super::route_counts::CapturedOrigin,
}
#[cfg(feature = "diagnostic-row-progress")]
impl Emit for ProgressEmit<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        let _origin = super::super::route_counts::resume(self.origin);
        super::super::route_counts::progress(
            crate::domain::operator::contract::views::ProgressCounter::EmittedRows,
            1,
        );
        self.emit.row(context, row)
    }
}
