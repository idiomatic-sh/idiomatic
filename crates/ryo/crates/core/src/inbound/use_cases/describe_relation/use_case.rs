//! The `describe_relation` use case: it finds the relation among the ones the language port lists.

use std::sync::Arc;

use super::errors::DescribeRelationError;
use super::interface::DescribeRelationUseCase;
use super::types::{DescribeRelationReport, DescribeRelationRequest};
use crate::domain::relation_lookup::errors::RelationLookupError;
use crate::domain::relation_lookup::service::resolve;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::Absence;
use ryo_language::domain::answers::constants::reason_name;
use ryo_language::domain::relations::service::class_of;
use ryo_language::domain::relations::types::Class;
use ryo_values::domain::descriptions::service::describe;
use ryo_values::domain::descriptions::types::{
    AbsenceReason, LanguageDescription, RelationReadability,
};

/// `describe_relation`, over the language port.
pub struct DescribeRelationUseCaseImpl {
    /// Lists the relations and what each linked language says of them.
    language: Arc<dyn LanguagePort>,
}

impl std::fmt::Debug for DescribeRelationUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl DescribeRelationUseCaseImpl {
    /// `describe_relation` over the port the edge injects.
    #[must_use]
    pub fn new(language: Arc<dyn LanguagePort>) -> Self {
        Self { language }
    }
}

impl DescribeRelationUseCase for DescribeRelationUseCaseImpl {
    fn describe_relation(
        &self,
        request: DescribeRelationRequest,
    ) -> Result<DescribeRelationReport, DescribeRelationError> {
        let known = self
            .language
            .relations()
            .map_err(|error| DescribeRelationError::Language {
                reason: error.to_string(),
            })?;
        let relation = resolve(&known.0, &request.name).map_err(|error| match error {
            RelationLookupError::UnknownRelation { name } => {
                DescribeRelationError::UnknownRelation { name }
            }
        })?;
        let mut description = describe(relation);
        // A relation the engine gives is given for every file, whatever language reads it: no language answers for it.
        if class_of(relation) == Class::Engine {
            return Ok(DescribeRelationReport {
                relation: description,
            });
        }
        description.languages = self
            .language
            .supports(relation)
            .0
            .into_iter()
            .map(|answer| LanguageDescription {
                name: answer.language,
                reads: match answer.absence {
                    Absence::Reads => RelationReadability::Reads,
                    Absence::Because(_) => RelationReadability::DoesNotRead,
                },
                reason: match answer.absence {
                    Absence::Reads => AbsenceReason(Box::default()),
                    Absence::Because(reason) => AbsenceReason(reason_name(reason).into()),
                },
                unknowns: answer.unknowns.to_vec(),
            })
            .collect();
        Ok(DescribeRelationReport {
            relation: description,
        })
    }
}
