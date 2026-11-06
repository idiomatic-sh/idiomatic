//! Application execution capabilities beside pure domain run configuration.
use ryo_execution::inbound::use_cases::execution::types::{PreparedExecutionProgram, RequestExecution};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
#[derive(Clone)]
pub(crate) struct VisitSetting<'a> {
    /// Explicit request provider; none preserves ordinary path recognition.
    pub source_language: Option<&'a ryo_values::domain::descriptions::types::LanguageName>,
    /// One prepared executable program shared by all scopes in this request.
    pub program: Arc<PreparedExecutionProgram>,
    /// Required application capabilities; no domain code owns or constructs them.
    pub services: &'a RequestExecution,
    pub domain: crate::domain::workers::types::VisitSetting<'a>,
}
impl<'a> Deref for VisitSetting<'a> {
    type Target = crate::domain::workers::types::VisitSetting<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}
impl DerefMut for VisitSetting<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.domain
    }
}
#[derive(Clone)]
pub(crate) struct CrossingInput<'a> {
    /// Invocation-owned physical arena lease view, supplied by the application coordinator.
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    pub program: Arc<PreparedExecutionProgram>,
    pub domain: crate::domain::crossing::types::CrossingInput<'a>,
}
impl<'a> Deref for CrossingInput<'a> {
    type Target = crate::domain::crossing::types::CrossingInput<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}
