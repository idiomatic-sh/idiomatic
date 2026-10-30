//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_query_language::inbound::use_cases::describe::errors::DescribeError as CoreError;
use ryo_query_language::inbound::use_cases::describe::types::DescribeReport as CoreReport;

use super::errors::DescribeError;
use super::types::{DescribeReport, Form, Production, ProductionName};
use crate::domain::operator::types::Example;

/// A core report, offered to the conversion.
struct DescribeGrammarReportSchema(CoreReport);

/// A core failure, offered to the conversion.
struct DescribeGrammarErrorSchema(CoreError);

impl From<DescribeGrammarReportSchema> for DescribeReport {
    fn from(schema: DescribeGrammarReportSchema) -> Self {
        Self {
            productions: schema
                .0
                .grammar
                .0
                .into_iter()
                .map(|each| Production {
                    name: ProductionName(each.name.0),
                    form: Form(each.form.0),
                    example: Example(each.example.0),
                })
                .collect(),
        }
    }
}

impl From<DescribeGrammarErrorSchema> for DescribeError {
    fn from(schema: DescribeGrammarErrorSchema) -> Self {
        match schema.0 {}
    }
}

/// The caller's report for the core's.
pub(super) fn report(report: CoreReport) -> DescribeReport {
    DescribeGrammarReportSchema(report).into()
}

/// The caller's failure for the core's.
pub(super) fn error(error: CoreError) -> DescribeError {
    DescribeGrammarErrorSchema(error).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
