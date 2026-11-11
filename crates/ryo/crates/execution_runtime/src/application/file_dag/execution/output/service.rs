//! The `output` application operation.

use ryo_execution_domain::domain::operator::types::{Emit, OperatorContext};
use ryo_execution_domain::domain::{
    operating::{errors::OperatingError, types::Memo},
    table_scratch::types::OperatorScratch,
};
use ryo_language::domain::symbols::{service::query_reborrow, types::QuerySymbols};
use ryo_relations::domain::relations::types::Value;

pub(crate) struct Headers<'a> {
    phase: ryo_execution_domain::domain::operating::types::AfterTheFiles,
    semantic: Option<&'a dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    pub(crate) prepared_matchers: Option<
        &'a ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers,
    >,
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
        scratch: &mut OperatorScratch,
        memo: &mut Vec<Memo>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        let mut downstream = OperatorContext {
            phase: self.phase,
            semantic: self.semantic,
            prepared_matchers: self.prepared_matchers,
            matcher: ryo_execution_domain::domain::matching::types::PatternMatcher(
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
    }
}

/// Deliver one committed publication through the same isolated downstream frame
/// used by primitive output. Mutable consumer state lives for this delivery
/// attempt, never in the producer context and never one allocation per row.
pub(crate) fn replay(
    rows: &ryo_execution_domain::domain::table_model::types::Rows<'_>,
    context: &mut OperatorContext<'_>,
    output: &mut dyn Emit,
) -> Result<(), OperatingError> {
    struct Delivery<'call, 'headers> {
        headers: Headers<'headers>,
        output: &'call mut dyn Emit,
        scratch: OperatorScratch,
        memo: Vec<Memo>,
    }
    impl Emit for Delivery<'_, '_> {
        fn row(
            &mut self,
            context: &mut OperatorContext<'_>,
            row: &[Value],
        ) -> Result<(), OperatingError> {
            self.headers.row(
                self.output,
                &mut context.symbols,
                &mut self.scratch,
                &mut self.memo,
                row,
            )
        }
    }
    let mut delivery = Delivery {
        headers: Headers::from_context(context),
        output,
        scratch: OperatorScratch::default(),
        memo: Vec::new(),
    };
    ryo_execution_domain::domain::replay::service::scan(
        rows,
        &mut Vec::new(),
        context,
        &mut delivery,
    )
}
