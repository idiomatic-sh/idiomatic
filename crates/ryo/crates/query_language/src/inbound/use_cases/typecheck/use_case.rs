//! The `typecheck` use case: it reads the text, checks it, and resolves every name in it once.

use std::sync::Arc;

use super::errors::TypecheckError;
use super::interface::TypecheckUseCase;
use super::keeper;
use super::types::Checked;
use super::service::{invalid, unresolved};
use crate::domain::assembling::service::lower_with_observations;
use crate::domain::linking::errors::LinkingError;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::syntax::constants::operator_name;
use crate::domain::syntax::service::{operators_used, relations_read};
use crate::domain::validation::service::validate_with_observations;
use crate::outbound::catalog::interface::CatalogPort;
use crate::outbound::syntax::errors::SyntaxError;
use crate::outbound::syntax::interface::SyntaxPort;
use ryo_relations::domain::relations::service::name;
use crate::domain::kind_check::service::{unknown_message, unknown_word};
use ryo_values::domain::descriptions::types::{KindVocabularies,
    OperatorName, RelationName, StepCount, ValidateReport,
};
use ryo_values::domain::program::types::{ParsedModules, WrittenProgram};
use ryo_values::domain::query_requests::types::QueryRequest;
use ryo_values::domain::query_tokens::types::{ByteOffset, QuerySpan};

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

/// Whether the checks stopped at a computed column: the one refusal that is the checks' plan's, not the query's.
fn computed(error: &LoweringError) -> bool {
    match error {
        LoweringError::ComputedColumn { .. } => true,
        LoweringError::Located { error, .. } => computed(error),
        _ => false,
    }
}

impl TypecheckUseCaseImpl {
    /// The checks, in order: the text, its links, the declared inputs, the validation and the resolution of every name. The program
    /// they read, and what they made of it.
    fn checked(
        &self,
        request: &QueryRequest,
    ) -> Result<(Checked, WrittenProgram, ParsedModules, WrittenProgram), TypecheckError> {
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
        let root = parsed.root.clone();
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
        let checked = match lower_with_observations(
            &program,
            &relations.0,
            &request.hosted,
            &request.observations,
        ) {
            Ok(plan) => Checked::Plan(plan),
            Err(error) if computed(&error) => Checked::Deferred(unresolved(error)),
            Err(error) => return Err(unresolved(error)),
        };
        Ok((checked, root, ParsedModules(Arc::new(parsed.fragments)), program))
    }
}

impl TypecheckUseCase for TypecheckUseCaseImpl {
    fn typecheck(&self, request: QueryRequest) -> Result<ValidateReport, TypecheckError> {
        self.typecheck_kinds(request, &KindVocabularies(Vec::new()))
    }

    fn typecheck_kinds(&self, request: QueryRequest, kinds: &KindVocabularies) -> Result<ValidateReport, TypecheckError> {
        let (checked, root, modules, program) = self.checked(&request)?;
        let steps = if request.observations.is_empty() {
            let nowhere = QuerySpan { start: ByteOffset(0), end: ByteOffset(0) };
            let (plan, steps) = keeper::lowered_plan(root, modules)
                .map_err(|message| TypecheckError::Invalid { span: nowhere, message })?;
            // A word written for a kind no linked language has is refused as a run refuses it before it reads a file; with no
            // language linked there is no kind to check against.
            if !kinds.0.is_empty()
                && let Some(word) = unknown_word(&plan, kinds)
            {
                return Err(TypecheckError::UnknownKind { reason: unknown_message(&word, kinds) });
            }
            steps
        } else {
            // The plan that runs lowers a rule over the outputs of the rules that produce its observations, which a validation has not
            // got: a query that reads declared inputs is judged by the checks alone, and counts the steps of their plan.
            match checked {
                Checked::Plan(plan) => plan.steps.len(),
                Checked::Deferred(error) => return Err(error),
            }
        };
        Ok(ValidateReport {
            relations: relations_read(&program)
                .into_iter()
                .map(|relation| RelationName(Box::from(name(relation).0)))
                .collect(),
            operators: operators_used(&program)
                .into_iter()
                .map(|operator| OperatorName(Box::from(operator_name(operator))))
                .collect(),
            steps: StepCount(steps),
        })
    }

    fn check(&self, request: QueryRequest) -> Result<Checked, TypecheckError> {
        self.checked(&request).map(|(checked, ..)| checked)
    }
}
