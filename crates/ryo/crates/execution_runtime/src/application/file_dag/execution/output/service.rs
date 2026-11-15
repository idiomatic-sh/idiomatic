//! The `output` application operation.

use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::{Emit, OperatorContext};
use ryo_language::domain::symbols::{service::query_reborrow, types::QuerySymbols};
use ryo_operators::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers;
use ryo_relations::domain::relations::types::Value;

pub(crate) struct Headers<'a> {
    phase: ryo_execution_domain::domain::operating::types::AfterTheFiles,
    semantic: Option<&'a dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    pub(crate) prepared_matchers:
        Option<&'a ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
    pub(crate) prepared_kinds: Option<
        &'a ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary,
    >,
    pub(crate) bindings: &'a [ryo_values::domain::query_bindings::types::ParameterBinding],
    pub(crate) source: ryo_values::domain::files::types::SourceView<'a>,
    pub(crate) paths: &'a [ryo_language::domain::files::types::FilePath],
    pub(crate) arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}

impl<'a> Headers<'a> {
    pub(crate) fn from_context(context: &OperatorContext<'a>) -> Self {
        Self {
            phase: context.phase,
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: context.bindings,
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
        }
    }
    pub(crate) fn row(
        &self,
        emit: &mut dyn Emit,
        symbols: &mut QuerySymbols<'_>,
        buffers: &mut DownstreamBuffers,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        buffers.lend(|scratch, memo| {
            let mut downstream = OperatorContext {
                phase: self.phase,
                semantic: self.semantic,
                prepared_matchers: self.prepared_matchers,
                matcher: ryo_operators::domain::patterns::types::PatternMatcher(
                    &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
                ),
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
        })
    }
}
