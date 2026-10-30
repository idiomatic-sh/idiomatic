//! The `list_relations` use case: it asks the language port for the catalog.

use std::sync::Arc;

use super::errors::ListRelationsError;
use super::interface::ListRelationsUseCase;
use super::types::ListRelationsReport;
use crate::outbound::language::interface::LanguagePort;
use ryo_values::domain::descriptions::service::summarize;

/// `list_relations`, over the language port.
pub struct ListRelationsUseCaseImpl {
    /// Lists the relations and what each linked language says of them.
    language: Arc<dyn LanguagePort>,
}

impl std::fmt::Debug for ListRelationsUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ListRelationsUseCaseImpl {
    /// `list_relations` over the port the edge injects.
    #[must_use]
    pub fn new(language: Arc<dyn LanguagePort>) -> Self {
        Self { language }
    }
}

impl ListRelationsUseCase for ListRelationsUseCaseImpl {
    fn list_relations(&self) -> Result<ListRelationsReport, ListRelationsError> {
        let mut relations: Vec<_> = self
            .language
            .relations()?
            .0
            .into_iter()
            .map(summarize)
            .collect();
        relations.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(ListRelationsReport { relations })
    }
}
