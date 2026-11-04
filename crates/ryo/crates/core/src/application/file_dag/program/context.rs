//! Run-bound invocation data. Callers lend only the original symbol pool again;
//! paths, source bytes, semantic lookup and scratch cannot be replaced per rule.
use super::{
    facade::PreparedProgram, facade_runtime::ProgramServices, request_identity::RequestIdentity,
};
use crate::domain::{
    coverage::types::RuleIndex,
    operating::{
        errors::OperatingError,
        types::{Memo, OperatorContext},
    },
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::symbols::{
    service::{same_transport_generation, transport_permit},
    types::{Symbols, TransportPermit},
};
use ryo_values::domain::stored_facts::types::ArenaBytes;
use std::sync::Arc;

pub(super) struct RunContext<'a> {
    request: Arc<RequestIdentity>,
    source: Arc<[u8]>,
    symbols: TransportPermit,
    semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
    kinds: Option<Arc<crate::domain::kind_sets::types::PreparedVocabulary>>,
    arenas: &'a [ArenaBytes],
    memo: Vec<Memo>,
    scratch: OperatorScratch,
}
impl<'a> RunContext<'a> {
    pub(super) fn file(
        services: &ProgramServices<'_>,
        source: &[u8],
        symbols: &mut Symbols,
        semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
        kinds: Option<Arc<crate::domain::kind_sets::types::PreparedVocabulary>>,
    ) -> Self {
        Self {
            request: Arc::clone(services.request()),
            source: Arc::from(source),
            symbols: transport_permit(symbols),
            semantic,
            kinds,
            arenas: &[],
            memo: Vec::new(),
            scratch: Default::default(),
        }
    }
    pub(super) fn project(
        services: &ProgramServices<'_>,
        symbols: TransportPermit,
        arenas: &'a [ArenaBytes],
    ) -> Self {
        Self {
            request: Arc::clone(services.request()),
            source: Arc::from([]),
            symbols,
            semantic: None,
            kinds: None,
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
        if !same_transport_generation(&self.symbols, &transport_permit(symbols)) {
            return Err(OperatingError::NotRead);
        }
        Ok(OperatorContext {
            semantic: self.semantic,
            prepared_kinds: self.kinds.as_deref(),
            prepared_matchers: Some(program.matchers().as_ref()),
            bindings: program.bindings(rule)?,
            symbols: symbols.into(),
            source: SourceView(&self.source),
            paths: self.request.paths(),
            arenas: self.arenas,
            memo: &mut self.memo,
            scratch: &mut self.scratch,
        })
    }
}
