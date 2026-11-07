//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::describe_relation::errors::DescribeRelationError as CoreError;
use ryo_core::inbound::use_cases::describe_relation::types::{
    DescribeRelationReport as CoreReport, DescribeRelationRequest as CoreRequest,
};
use ryo_values::domain::descriptions::types::RelationName as CoreRelationName;
use ryo_values::domain::descriptions::types::RelationReadability;

use super::errors::DescribeError;
use super::types::{
    ColumnDescription, ColumnKind, DescribeReport, DescribeRequest, KindWord, LanguageDescription,
    Reading, Reason, RelationDescription, UnknownName,
};
use crate::domain::language::types::LanguageName;
use crate::domain::relation::types::{ColumnName, Holds, RelationName};

/// A request of the caller's, offered to the conversion.
struct DescribeRelationRequestSchema(DescribeRequest);

/// A core report, offered to the conversion.
struct DescribeRelationReportSchema(CoreReport);

/// A core failure, offered to the conversion.
struct DescribeRelationErrorSchema(CoreError);

impl From<DescribeRelationRequestSchema> for CoreRequest {
    fn from(schema: DescribeRelationRequestSchema) -> Self {
        Self {
            name: CoreRelationName(schema.0.name.0),
        }
    }
}

impl From<DescribeRelationReportSchema> for DescribeReport {
    fn from(schema: DescribeRelationReportSchema) -> Self {
        let relation = schema.0.relation;
        Self {
            relation: RelationDescription {
                name: RelationName(relation.name.0),
                summary: Holds(relation.summary.0.into()),
                columns: relation
                    .columns
                    .into_iter()
                    .map(|each| ColumnDescription {
                        name: ColumnName(each.name.0),
                        kind: ColumnKind(each.kind.0),
                        words: each
                            .words
                            .into_iter()
                            .map(|word| KindWord(word.0))
                            .collect(),
                    })
                    .collect(),
                languages: relation
                    .languages
                    .into_iter()
                    .map(|each| LanguageDescription {
                        name: LanguageName(each.name.0),
                        reads: match each.reads {
                            RelationReadability::Reads => Reading::Reads,
                            RelationReadability::DoesNotRead => Reading::DoesNotRead,
                        },
                        reason: Reason(each.reason.0),
                        unknowns: each
                            .unknowns
                            .into_iter()
                            .map(|unknown| UnknownName(unknown.text.0.into()))
                            .collect(),
                    })
                    .collect(),
            },
        }
    }
}

impl From<DescribeRelationErrorSchema> for DescribeError {
    fn from(schema: DescribeRelationErrorSchema) -> Self {
        match schema.0 {
            CoreError::UnknownRelation { name } => Self::UnknownRelation { name },
            CoreError::Language { reason } => Self::Language { reason },
        }
    }
}

/// The core's request for the caller's.
pub(super) fn request(request: DescribeRequest) -> CoreRequest {
    DescribeRelationRequestSchema(request).into()
}

/// The caller's report for the core's.
pub(super) fn report(report: CoreReport) -> DescribeReport {
    DescribeRelationReportSchema(report).into()
}

/// The caller's failure for the core's.
pub(super) fn error(error: CoreError) -> DescribeError {
    DescribeRelationErrorSchema(error).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
