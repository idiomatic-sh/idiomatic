//! Run-bound invocation data. Callers lend only the original symbol pool again;
//! paths, source bytes, semantic lookup and scratch cannot be replaced per rule.
use super::facade_runtime::types::ProgramServices;
use super::request_identity::RequestIdentity;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use ryo_language::domain::symbols::{
    service::{same_transport_generation, transport_permit},
    types::{Symbols, TransportPermit},
};
use ryo_values::domain::stored_facts::types::ArenaBytes;
use std::sync::Arc;
use {
    ryo_execution_domain::domain::coverage::types::RuleIndex, ryo_execution_domain::domain::operating::errors::OperatingError,
    ryo_execution_domain::domain::operating::types::Memo, ryo_execution_domain::domain::operating::types::OperatorContext,
    ryo_execution_domain::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

pub(super) struct RunContext<'a> {
    phase: ryo_execution_domain::domain::operating::types::AfterTheFiles,
    request: Arc<RequestIdentity>,
    source: SourceView<'a>,
    symbols: TransportPermit,
    semantic: Option<&'a dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    kinds: Option<Arc<ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>>,
    arenas: &'a [ArenaBytes],
    memo: Vec<Memo>,
    scratch: OperatorScratch,
}
impl<'a> RunContext<'a> {
    /// Whether the caller lends the original symbol pool again.
    pub(super) fn validate(&self, symbols: &mut Symbols) -> Result<(), OperatingError> {
        if same_transport_generation(&self.symbols, &transport_permit(symbols))
            == ryo_language::domain::symbols::types::Generation::Different
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
    pub(super) fn file(
        services: &ProgramServices,
        source: SourceView<'a>,
        symbols: &mut Symbols,
        semantic: Option<&'a dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
        kinds: Option<Arc<ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>>,
    ) -> Self {
        Self {
            request: Arc::clone(services.request()),
            source,
            symbols: transport_permit(symbols),
            semantic,
            kinds,
            phase: ryo_execution_domain::domain::operating::types::AfterTheFiles::Before,
            arenas: &[],
            memo: Vec::new(),
            scratch: Default::default(),
        }
    }
    pub(super) fn project(
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
    pub(super) fn borrow<'b>(
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
            matcher: ryo_execution_domain::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
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
