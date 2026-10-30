//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::describe_operator::errors::DescribeOperatorError as CoreError;
use ryo_core::inbound::use_cases::describe_operator::types::{
    DescribeOperatorReport as CoreReport, DescribeOperatorRequest as CoreRequest,
};
use ryo_values::domain::descriptions::types::{
    FieldRequirement as CoreRequirement, OperatorName as CoreOperatorName,
};

use super::errors::DescribeError;
use super::types::{
    Choice, DescribeReport, DescribeRequest, FieldDescription, FieldName, OperatorDescription,
    Requirement,
};
use crate::domain::operator::types::{Example, OperatorName, Purpose};

/// A request of the caller's, offered to the conversion.
struct DescribeOperatorRequestSchema(DescribeRequest);

/// A core report, offered to the conversion.
struct DescribeOperatorReportSchema(CoreReport);

/// A core failure, offered to the conversion.
struct DescribeOperatorErrorSchema(CoreError);

impl From<DescribeOperatorRequestSchema> for CoreRequest {
    fn from(schema: DescribeOperatorRequestSchema) -> Self {
        Self {
            name: CoreOperatorName(schema.0.name.0),
        }
    }
}

impl From<DescribeOperatorReportSchema> for DescribeReport {
    fn from(schema: DescribeOperatorReportSchema) -> Self {
        let operator = schema.0.operator;
        Self {
            operator: OperatorDescription {
                name: OperatorName(operator.name.0),
                summary: Purpose(operator.summary.0),
                fields: operator
                    .fields
                    .into_iter()
                    .map(|each| FieldDescription {
                        name: FieldName(each.name.0),
                        summary: Purpose(each.summary.0),
                        required: match each.required {
                            CoreRequirement::Required => Requirement::Required,
                            CoreRequirement::Optional => Requirement::Optional,
                        },
                    })
                    .collect(),
                options: operator
                    .options
                    .into_iter()
                    .map(|each| Choice(each.0))
                    .collect(),
                example: Example(operator.example.0),
            },
        }
    }
}

impl From<DescribeOperatorErrorSchema> for DescribeError {
    fn from(schema: DescribeOperatorErrorSchema) -> Self {
        match schema.0 {
            CoreError::UnknownOperator { name } => Self::UnknownOperator { name },
        }
    }
}

/// The core's request for the caller's.
pub(super) fn request(request: DescribeRequest) -> CoreRequest {
    DescribeOperatorRequestSchema(request).into()
}

/// The caller's report for the core's.
pub(super) fn report(report: CoreReport) -> DescribeReport {
    DescribeOperatorReportSchema(report).into()
}

/// The caller's failure for the core's.
pub(super) fn error(error: CoreError) -> DescribeError {
    DescribeOperatorErrorSchema(error).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
