//! The `compile` use case: it reads the text, checks it, and resolves every name in it once.

use std::sync::Arc;

use super::errors::CompileError;
use super::interface::CompileUseCase;
use super::service::{invalid, syntax_refused, unresolved};
use super::types::CompileReport;
use ryo_values::domain::plans::types::QueryRequest;
use crate::domain::assembling::service::lower_with_observations;
use crate::domain::linking::errors::LinkingError;
use crate::domain::validation::service::validate_with_observations;
use crate::outbound::catalog::interface::CatalogPort;
use crate::outbound::syntax::interface::SyntaxPort;

/// `compile`, over the syntax and the catalog it calls.
pub struct CompileUseCaseImpl {
    /// Reads the program's text into its syntax.
    syntax: Arc<dyn SyntaxPort>,
    /// Lists the relations a program may read.
    catalog: Arc<dyn CatalogPort>,
}

impl std::fmt::Debug for CompileUseCaseImpl {
    /// The use case for a debug print: its ports are trait objects with no `Debug`, so it names only itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CompileUseCaseImpl {
    /// `compile` over the ports the edge injects.
    #[must_use]
    pub fn new(syntax: Arc<dyn SyntaxPort>, catalog: Arc<dyn CatalogPort>) -> Self {
        Self { syntax, catalog }
    }
}

impl CompileUseCase for CompileUseCaseImpl {
    fn compile(&self, request: QueryRequest) -> Result<CompileReport, CompileError> {
        // Modules parsed earlier in the session from the same catalog are not parsed again: the root alone is read.
        let (root, modules) = if request.modules.0.is_empty() {
            let parsed = self
                .syntax
                .read_bundle(&request.text)
                .map_err(syntax_refused)?;
            (
                parsed.root,
                ryo_values::domain::program::types::ParsedModules(Arc::new(parsed.fragments)),
            )
        } else {
            (
                self.syntax.read_root(&request.text).map_err(syntax_refused)?,
                request.modules.clone(),
            )
        };
        let program = crate::domain::expanding::service::link_with_observations(
            &request.text,
            root,
            &modules.0,
            &request.observations,
        )
        .map_err(|error| match error {
            LinkingError::Located { origin, message } => CompileError::Located {
                error: Box::new(CompileError::Invalid {
                    span: origin.span,
                    message,
                }),
                origin,
            },
        })?;
        let relations = self
            .catalog
            .relations()
            .map_err(|error| CompileError::Catalog {
                reason: error.to_string(),
            })?;
        crate::domain::observation_inputs::service::validate_inputs(
            &relations.0,
            &request.hosted,
            &request.observations,
        )
        .map_err(|error| CompileError::ObservationInputs {
            message: error.to_string(),
        })?;
        let names: Vec<_> = request
            .hosted
            .iter()
            .map(|each| each.name.clone())
            .collect();
        validate_with_observations(&program, &relations.0, &names, &request.observations)
            .map_err(invalid)?;
        let plan = lower_with_observations(
            &program,
            &relations.0,
            &request.hosted,
            &request.observations,
        )
        .map_err(unresolved)?;
        Ok(CompileReport { plan, modules })
    }
}
