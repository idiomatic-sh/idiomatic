//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::list_relations::errors::ListRelationsError as CoreError;
use ryo_core::inbound::use_cases::list_relations::types::ListRelationsReport as CoreReport;

use super::errors::ListError;
use super::types::{ListReport, RelationSummary};
use crate::domain::relation::types::{Holds, RelationName};

/// A core report, offered to the conversion.
struct ListRelationsReportSchema(CoreReport);

/// A core failure, offered to the conversion.
struct ListRelationsErrorSchema(CoreError);

impl From<ListRelationsReportSchema> for ListReport {
    fn from(schema: ListRelationsReportSchema) -> Self {
        Self {
            relations: schema
                .0
                .relations
                .into_iter()
                .map(|each| RelationSummary {
                    name: RelationName(each.name.0),
                    summary: Holds(each.summary.0.into()),
                })
                .collect(),
        }
    }
}

impl From<ListRelationsErrorSchema> for ListError {
    fn from(schema: ListRelationsErrorSchema) -> Self {
        match schema.0 {
            CoreError::Language(error) => Self::Language {
                reason: error.to_string(),
            },
        }
    }
}

/// The caller's report for the core's.
pub(super) fn report(report: CoreReport) -> ListReport {
    ListRelationsReportSchema(report).into()
}

/// The caller's failure for the core's.
pub(super) fn error(error: CoreError) -> ListError {
    ListRelationsErrorSchema(error).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
