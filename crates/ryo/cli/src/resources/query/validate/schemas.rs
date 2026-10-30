//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::validate::types::ValidateRequest;
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::plan_parts::types::QueryText;

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ValidateSchemaError {
    /// The query text is empty.
    #[error("the query text is empty")]
    BlankText,
}

/// The arguments as the command line gives them, offered to the conversion.
struct ValidateArgumentsSchema {
    /// The RyoQL text to check.
    text: String,
}

impl TryFrom<ValidateArgumentsSchema> for ValidateRequest {
    type Error = ValidateSchemaError;

    fn try_from(schema: ValidateArgumentsSchema) -> Result<Self, Self::Error> {
        if schema.text.trim().is_empty() {
            return Err(ValidateSchemaError::BlankText);
        }
        // A relation the host declares can only come from a program that links the SDK: the command line declares none.
        Ok(Self {
            text: QueryText(schema.text.into()),
            hosted: Vec::new(),
        })
    }
}

/// What the command prints: plain text lines, or the same as JSON.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The relations the query reads.
    pub relations: Vec<String>,
    /// The operations the query uses.
    pub operators: Vec<String>,
}

impl From<ValidateReport> for AnswerSchema {
    fn from(report: ValidateReport) -> Self {
        Self {
            relations: report
                .relations
                .into_iter()
                .map(|name| name.0.into_string())
                .collect(),
            operators: report
                .operators
                .into_iter()
                .map(|name| name.0.into_string())
                .collect(),
        }
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ok")?;
        writeln!(f, "relations: {}", self.relations.join(", "))?;
        write!(f, "operators: {}", self.operators.join(", "))?;
        writeln!(f)
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `BlankText` for a query text that is empty.
pub(super) fn request(text: String) -> Result<ValidateRequest, ValidateSchemaError> {
    ValidateArgumentsSchema { text }.try_into()
}

/// The answer printed for the service's report.
pub(super) fn answer(report: ValidateReport) -> AnswerSchema {
    AnswerSchema::from(report)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for ValidateSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::BlankText => {
                crate::outcome::remedy::Next::Fix("pass the query with `--text`".to_owned())
            }
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
