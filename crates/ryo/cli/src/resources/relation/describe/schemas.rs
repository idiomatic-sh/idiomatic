//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::describe_relation::types::{
    DescribeRelationReport, DescribeRelationRequest,
};
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::descriptions::types::RelationReadability;

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DescribeSchemaError {
    /// The relation name is empty.
    #[error("the relation name is empty")]
    BlankName,
}

/// The arguments as the command line gives them, offered to the conversion.
struct DescribeArgumentsSchema {
    /// The relation to describe.
    name: String,
}

impl TryFrom<DescribeArgumentsSchema> for DescribeRelationRequest {
    type Error = DescribeSchemaError;

    fn try_from(schema: DescribeArgumentsSchema) -> Result<Self, Self::Error> {
        if schema.name.trim().is_empty() {
            return Err(DescribeSchemaError::BlankName);
        }
        Ok(Self {
            name: RelationName(schema.name.into()),
        })
    }
}

/// What the command prints: plain text lines, or the same as JSON, each column and each language an object.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The relation's name.
    pub name: String,
    /// What it holds.
    pub summary: String,
    /// Its columns, in order.
    pub columns: Vec<ColumnSchema>,
    /// What each linked language answers of it.
    pub languages: Vec<LanguageSchema>,
}

/// One column of the relation: printed as `name: kind`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct ColumnSchema {
    /// The column's name.
    pub name: String,
    /// The kind of value it holds.
    pub kind: String,
}

/// What one linked language answers of the relation: printed as `rust: reads`, or `toml: not in this language (not-code)`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct LanguageSchema {
    /// The language's name.
    pub name: String,
    /// Whether it reads the relation.
    pub reads: bool,
    /// Why the relation is not in the language: present exactly when it does not read it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<DescribeRelationReport> for AnswerSchema {
    fn from(report: DescribeRelationReport) -> Self {
        Self {
            name: report.relation.name.0.into_string(),
            summary: report.relation.summary.0.to_owned(),
            columns: report
                .relation
                .columns
                .into_iter()
                .map(|column| ColumnSchema {
                    name: column.name.0.to_string(),
                    kind: column.kind.0.to_string(),
                })
                .collect(),
            languages: report
                .relation
                .languages
                .into_iter()
                .map(|language| {
                    let reads = language.reads == RelationReadability::Reads;
                    LanguageSchema {
                        name: language.name.0.to_string(),
                        reads,
                        reason: (!reads).then(|| language.reason.0.to_string()),
                    }
                })
                .collect(),
        }
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.summary)?;
        for column in &self.columns {
            write!(f, "\n  {}: {}", column.name, column.kind)?;
        }
        for language in &self.languages {
            match &language.reason {
                None => write!(f, "\n  {}: reads", language.name)?,
                Some(reason) => {
                    write!(f, "\n  {}: not in this language ({reason})", language.name)?;
                }
            }
        }
        writeln!(f)
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `BlankName` for a name that is empty.
pub(super) fn request(name: String) -> Result<DescribeRelationRequest, DescribeSchemaError> {
    DescribeArgumentsSchema { name }.try_into()
}

/// The answer printed for the service's report.
pub(super) fn answer(report: DescribeRelationReport) -> AnswerSchema {
    AnswerSchema::from(report)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for DescribeSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::BlankName => crate::outcome::remedy::Next::Run("ryo relation list".to_owned()),
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
