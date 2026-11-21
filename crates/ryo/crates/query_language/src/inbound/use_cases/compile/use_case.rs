//! The `compile` use case: it reads the text, checks it, and resolves every name in it once.

use std::sync::{Arc, Mutex, PoisonError};

use super::errors::CompileError;
use super::interface::CompileUseCase;
use super::service::{invalid, syntax_refused, unresolved};
use super::types::CompileReport;
use crate::domain::assembling::service::lower_with_observations;
use crate::domain::checking::service::fragments_checked;
use crate::domain::checking::types::FragmentChecks;
use crate::domain::linking::errors::LinkingError;
use crate::domain::validation::service::validate_with_observations;
use crate::outbound::catalog::interface::CatalogPort;
use crate::outbound::syntax::interface::SyntaxPort;
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_requests::types::QueryRequest;

/// `compile`, over the syntax and the catalog it calls.
pub struct CompileUseCaseImpl {
    /// Reads the program's text into its syntax.
    syntax: Arc<dyn SyntaxPort>,
    /// Lists the relations a program may read.
    catalog: Arc<dyn CatalogPort>,
    /// The checks of the modules last compiled over: a catalog's fragments are checked once, however many rules import
    /// them. Held with the modules they are of, so they are never read for another catalog's.
    checked: Mutex<Option<(ParsedModules, Arc<FragmentChecks>)>>,
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
        Self {
            syntax,
            catalog,
            checked: Mutex::new(None),
        }
    }
}

impl CompileUseCaseImpl {
    /// The checks of `modules`, the fragments a request is compiled over: read when the last compile was over the same
    /// parsed modules, made and kept otherwise.
    fn checks_of(
        &self,
        bundle: &ryo_values::domain::query_tokens::types::QueryBundle,
        modules: &ParsedModules,
    ) -> Arc<FragmentChecks> {
        let held = self.checked.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((of, checks)) = held.as_ref()
            && Arc::ptr_eq(&of.0, &modules.0)
        {
            return Arc::clone(checks);
        }
        drop(held);
        let checks = Arc::new(fragments_checked(bundle, &modules.0));
        let mut held = self.checked.lock().unwrap_or_else(PoisonError::into_inner);
        *held = Some((modules.clone(), Arc::clone(&checks)));
        checks
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
                self.syntax
                    .read_root(&request.text)
                    .map_err(syntax_refused)?,
                request.modules.clone(),
            )
        };
        let checks = self.checks_of(&request.text, &modules);
        let program = crate::domain::expanding::service::link_checked(
            &request.text,
            root,
            &modules.0,
            &request.observations,
            &checks,
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
