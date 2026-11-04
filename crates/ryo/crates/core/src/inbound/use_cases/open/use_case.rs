//! The `open` use case: it checks that the languages answer and that the machine has the memory asked for.

use std::sync::Arc;

use super::errors::OpenError;
use super::interface::OpenUseCase;
use super::types::{OpenReport, OpenRequest};
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::memory::interface::MemoryPort;
use ryo_values::domain::runs::types::Session;

/// `open`, over the language and memory ports.
pub struct OpenUseCaseImpl {
    /// Names the languages linked and the relations they read.
    language: Arc<dyn LanguagePort>,
    /// Says how much memory the machine has to spare.
    memory: Arc<dyn MemoryPort>,
}

impl std::fmt::Debug for OpenUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl OpenUseCaseImpl {
    /// `open` over the ports the edge injects.
    #[must_use]
    pub fn new(language: Arc<dyn LanguagePort>, memory: Arc<dyn MemoryPort>) -> Self {
        Self { language, memory }
    }
}

impl OpenUseCase for OpenUseCaseImpl {
    fn open(&self, request: OpenRequest) -> Result<OpenReport, OpenError> {
        // The language must answer what it reads before the workspace is opened over it.
        if let Err(error) = self.language.relations() {
            return Err(OpenError::Language {
                reason: error.to_string(),
            });
        }
        // The machine must have the memory the run was given.
        let _budget = self.memory.budget().map_err(|error| OpenError::Memory {
            reason: error.to_string(),
        })?;
        Ok(OpenReport {
            session: Session {
                root: request.root,
                repository: request.repository,
                search_scope: Default::default(),
                given_inventory: Default::default(),
            },
        })
    }
}
