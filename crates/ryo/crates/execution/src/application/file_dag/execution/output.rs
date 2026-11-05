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
    pub(super) bindings: &'a [ryo_values::domain::query_bindings::types::ParameterBinding],
    pub(super) source: ryo_values::domain::files::types::SourceView<'a>,
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
