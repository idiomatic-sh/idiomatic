//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::list_operators::errors::ListOperatorsError as CoreError;
use ryo_core::inbound::use_cases::list_operators::types::ListOperatorsReport as CoreReport;

use super::errors::ListError;
use super::types::{ListReport, OperatorSummary};
use crate::domain::operator::types::{OperatorName, Purpose};

/// A core report, offered to the conversion.
struct ListOperatorsReportSchema(CoreReport);

/// A core failure, offered to the conversion.
struct ListOperatorsErrorSchema(CoreError);

impl From<ListOperatorsReportSchema> for ListReport {
    fn from(schema: ListOperatorsReportSchema) -> Self {
        Self {
            operators: schema
                .0
                .operators
                .into_iter()
                .map(|each| OperatorSummary {
                    name: OperatorName(each.name.0),
                    summary: Purpose(each.summary.0),
                })
                .collect(),
        }
    }
}

impl From<ListOperatorsErrorSchema> for ListError {
    fn from(schema: ListOperatorsErrorSchema) -> Self {
        match schema.0 {}
    }
}

/// The caller's report for the core's.
pub(super) fn report(report: CoreReport) -> ListReport {
    ListOperatorsReportSchema(report).into()
}

/// The caller's failure for the core's.
pub(super) fn error(error: CoreError) -> ListError {
    ListOperatorsErrorSchema(error).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
