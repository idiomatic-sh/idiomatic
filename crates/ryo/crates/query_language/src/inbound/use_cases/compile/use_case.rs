//! The `compile` use case: it reads the text, checks it, and resolves every name in it once.

use std::sync::Arc;

use super::errors::CompileError;
use super::interface::CompileUseCase;
use super::service::{invalid, unresolved};
use super::types::{CompileReport, CompileRequest};
use crate::domain::assembling::service::lower;
use crate::domain::validation::service::validate;
use crate::outbound::catalog::interface::CatalogPort;
use crate::outbound::syntax::errors::SyntaxError;
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
    fn compile(&self, request: CompileRequest) -> Result<CompileReport, CompileError> {
        let parsed = self
            .syntax
            .read_bundle(&request.text)
            .map_err(|error| match error {
                SyntaxError::Unreadable { span, message } => CompileError::Syntax { span, message },
                SyntaxError::Located { origin, message } => CompileError::Located {
                    error: Box::new(CompileError::Syntax {
                        span: origin.span,
                        message,
                    }),
                    origin,
                },
            })?;
        let program =
            crate::domain::linking::service::link(&request.text, parsed.root, &parsed.fragments)
                .map_err(|error| {
                    let span = error.origin.span;
                    CompileError::Located {
                        origin: error.origin,
                        error: Box::new(CompileError::Invalid {
                            span,
                            message: error.message,
                        }),
                    }
                })?;
        let relations = self
            .catalog
            .relations()
            .map_err(|error| CompileError::Catalog {
                reason: error.to_string(),
            })?;
        let names: Vec<_> = request
            .hosted
            .iter()
            .map(|each| each.name.clone())
            .collect();
        validate(&program, &relations.0, &names).map_err(invalid)?;
        let plan = lower(&program, &relations.0, &request.hosted).map_err(unresolved)?;
        Ok(CompileReport { plan })
    }
}
