//! The `context` application operation.

use super::super::facade_runtime::service::ProgramServices;
use super::super::request_identity::service::RequestIdentity;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use ryo_language::domain::symbols::{
    service::{same_transport_generation, transport_permit},
    types::{Symbols, TransportPermit},
};
use ryo_values::domain::stored_facts::types::ArenaBytes;
use std::sync::Arc;
use {
    ryo_execution_domain::domain::coverage::types::RuleIndex,
    ryo_execution_domain::domain::operating::errors::OperatingError,
    ryo_execution_domain::domain::operator::types::OperatorContext,
    ryo_operators::domain::table_scratch::types::Memo,
    ryo_operators::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

pub(crate) struct RunContext<'a> {
    phase: ryo_execution_domain::domain::operating::types::AfterTheFiles,
    request: Arc<RequestIdentity>,
    source: SourceView<'a>,
    symbols: TransportPermit,
    semantic: Option<&'a dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    kinds: Option<
        Arc<ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>,
    >,
    arenas: &'a [ArenaBytes],
    memo: Vec<Memo>,
    scratch: OperatorScratch,
}

impl<'a> RunContext<'a> {
    pub(crate) fn project(
        services: &ProgramServices,
        symbols: TransportPermit,
        arenas: &'a [ArenaBytes],
    ) -> Self {
        Self {
            request: Arc::clone(services.request()),
            source: SourceView(&[]),
            symbols,
            semantic: None,
            kinds: None,
            phase: ryo_execution_domain::domain::operating::types::AfterTheFiles::After,
            arenas,
            memo: Vec::new(),
            scratch: Default::default(),
        }
    }
    pub(crate) fn borrow<'b>(
        &'b mut self,
        symbols: &'b mut Symbols,
        program: &'b PreparedProgram,
        rule: RuleIndex,
    ) -> Result<OperatorContext<'b>, OperatingError> {
        if same_transport_generation(&self.symbols, &transport_permit(symbols))
            == ryo_language::domain::symbols::types::Generation::Different
        {
            return Err(OperatingError::NotRead);
        }
        Ok(OperatorContext {
            phase: self.phase,
            semantic: self.semantic,
            prepared_kinds: self.kinds.as_deref(),
            prepared_matchers: Some(program.matchers().as_ref()),
            matcher: ryo_operators::domain::patterns::types::PatternMatcher(
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
            ),
            bindings: program.bindings(rule)?,
            symbols: symbols.into(),
            source: self.source,
            paths: self.request.paths(),
            arenas: self.arenas,
            memo: &mut self.memo,
            scratch: &mut self.scratch,
        })
    }
}
