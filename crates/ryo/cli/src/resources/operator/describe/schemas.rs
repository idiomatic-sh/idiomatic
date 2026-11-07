//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::describe_operator::types::{
    DescribeOperatorReport, DescribeOperatorRequest,
};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorName};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DescribeSchemaError {
    /// The operator name is empty.
    #[error("the operator name is empty")]
    BlankName,
}

/// The arguments as the command line gives them, offered to the conversion.
struct DescribeArgumentsSchema {
    /// The operator to describe.
    name: String,
}

impl TryFrom<DescribeArgumentsSchema> for DescribeOperatorRequest {
    type Error = DescribeSchemaError;

    fn try_from(schema: DescribeArgumentsSchema) -> Result<Self, Self::Error> {
        if schema.name.trim().is_empty() {
            return Err(DescribeSchemaError::BlankName);
        }
        Ok(Self {
            name: OperatorName(schema.name.into()),
        })
    }
}

/// What the command prints: plain text lines, or the same as JSON.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The operator's name.
    pub name: String,
    /// What it does.
    pub summary: String,
    /// Its fields, in order.
    pub fields: Vec<FieldSchema>,
    /// The operator written as one statement of query text.
    pub example: String,
}

/// One field of the operator: printed as `name (required): summary`, or `name: summary`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct FieldSchema {
    /// The field's name.
    pub name: String,
    /// Whether a statement must give it.
    pub required: bool,
    /// What it holds.
    pub summary: String,
}

impl From<DescribeOperatorReport> for AnswerSchema {
    fn from(report: DescribeOperatorReport) -> Self {
        Self {
            name: report.operator.name.0.into_string(),
            summary: report.operator.summary.0.into_string(),
            fields: report
                .operator
                .fields
                .into_iter()
                .map(|field| FieldSchema {
                    name: field.name.0.to_string(),
                    required: field.required == FieldRequirement::Required,
                    summary: field.summary.0.to_string(),
                })
                .collect(),
            example: report.operator.example.0.into_string(),
        }
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.summary)?;
        for field in &self.fields {
            let required = if field.required { " (required)" } else { "" };
            write!(f, "\n  {}{required}: {}", field.name, field.summary)?;
        }
        write!(f, "\nexample: {}", self.example)?;
        writeln!(f)
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `BlankName` for a name that is empty.
pub(super) fn request(name: String) -> Result<DescribeOperatorRequest, DescribeSchemaError> {
    DescribeArgumentsSchema { name }.try_into()
}

/// The answer printed for the service's report.
pub(super) fn answer(report: DescribeOperatorReport) -> AnswerSchema {
    AnswerSchema::from(report)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for DescribeSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::BlankName => crate::outcome::remedy::Next::Run("ryo operator list".to_owned()),
        }
    }
}

impl crate::output::Encoded for AnswerSchema {
    fn json(
        &self,
        to: &mut dyn std::io::Write,
        spacing: crate::output::Spacing,
    ) -> std::io::Result<()> {
        match spacing {
            crate::output::Spacing::Compact => serde_json::to_writer(to, self),
            crate::output::Spacing::Indented => serde_json::to_writer_pretty(to, self),
        }
        .map_err(std::io::Error::from)
    }
}
