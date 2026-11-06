//! The `typecheck` use case: it reads the text, checks it, and resolves every name in it once.

use std::sync::Arc;

use super::errors::TypecheckError;
use super::interface::TypecheckUseCase;
use super::service::{invalid, unresolved};
use super::types::TypecheckRequest;
use crate::domain::assembling::service::lower_with_observations;
use crate::domain::linking::errors::LinkingError;
use crate::domain::syntax::constants::operator_name;
use crate::domain::syntax::service::{operators_used, relations_read};
use crate::domain::validation::service::validate_with_observations;
use crate::outbound::catalog::interface::CatalogPort;
use crate::outbound::syntax::errors::SyntaxError;
use crate::outbound::syntax::interface::SyntaxPort;
use ryo_relations::domain::relations::service::name;
use ryo_values::domain::descriptions::types::{
    OperatorName, RelationName, StepCount, ValidateReport,
};

/// `typecheck`, over the syntax and the catalog it calls.
pub struct TypecheckUseCaseImpl {
    /// Reads the program's text into its syntax.
    syntax: Arc<dyn SyntaxPort>,
    /// Lists the relations a program may read.
    catalog: Arc<dyn CatalogPort>,
}

impl std::fmt::Debug for TypecheckUseCaseImpl {
    /// The use case for a debug print: its ports are trait objects with no `Debug`, so it names only itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl TypecheckUseCaseImpl {
    /// `typecheck` over the ports the edge injects.
    #[must_use]
    pub fn new(syntax: Arc<dyn SyntaxPort>, catalog: Arc<dyn CatalogPort>) -> Self {
        Self { syntax, catalog }
    }
}

impl TypecheckUseCase for TypecheckUseCaseImpl {
    fn typecheck(&self, request: TypecheckRequest) -> Result<ValidateReport, TypecheckError> {
        let parsed = self
            .syntax
            .read_bundle(&request.text)
            .map_err(|error| match error {
                SyntaxError::Unreadable { span, message } => {
                    TypecheckError::Syntax { span, message }
                }
                SyntaxError::Located { origin, message } => TypecheckError::Located {
                    error: Box::new(TypecheckError::Syntax {
                        span: origin.span,
                        message,
                    }),
                    origin,
                },
            })?;
        let program = crate::domain::expanding::service::link_with_observations(
            &request.text,
            parsed.root,
            &parsed.fragments,
            &request.observations,
        )
        .map_err(|error| match error {
            LinkingError::Located { origin, message } => TypecheckError::Located {
                error: Box::new(TypecheckError::Invalid {
                    span: origin.span,
                    message,
                }),
                origin,
            },
        })?;
        let relations = self
            .catalog
            .relations()
            .map_err(|error| TypecheckError::Catalog {
                reason: error.to_string(),
            })?;
        crate::domain::observation_inputs::service::validate_inputs(
            &relations.0,
            &request.hosted,
            &request.observations,
        )
        .map_err(|error| TypecheckError::ObservationInputs {
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
        Ok(ValidateReport {
            relations: relations_read(&program)
                .into_iter()
                .map(|relation| RelationName(Box::from(name(relation).0)))
                .collect(),
            operators: operators_used(&program)
                .into_iter()
                .map(|operator| OperatorName(Box::from(operator_name(operator))))
                .collect(),
            steps: StepCount(plan.steps.len()),
        })
    }
}
