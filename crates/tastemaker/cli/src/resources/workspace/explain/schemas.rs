//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::inbound::use_cases::explain::types::{
    Asked, ExplainReport, ExplainRequest, NameAsked,
};

use tastemaker_core::domain::packages::constants::{RULE_WORD, SPEC_WORD, TEMPLATE_WORD};
use tastemaker_core::domain::topics::constants::{OPERATOR_WORD, RELATION_WORD};
use tastemaker_core::domain::topics::service::{authoring, names};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ExplainSchemaError {
    /// `--all` with a subject.
    #[error("`--all` explains everything: give no subject with it")]
    AllWithSubject,
    /// `--all` with `--dry-run`, which would write nothing.
    #[error("`--all --dry-run` would write nothing to say: leave out `--dry-run`, or `--all`")]
    DryRun,
}

/// The arguments as the command line gives them, offered to the conversion.
struct ExplainArgumentsSchema<'a> {
    /// A path, else `<kind>/<name>`, else a topic; the whole workspace when left out.
    subject: Option<String>,
    /// Write every topic and definition as Markdown, with an `llms.txt` index.
    all: bool,
    /// The directory `--all` writes to.
    output_directory: Option<std::path::PathBuf>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<ExplainArgumentsSchema<'_>> for ExplainRequest {
    type Error = ExplainSchemaError;

    fn try_from(schema: ExplainArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let asked = match (schema.all, schema.subject, schema.output_directory) {
            (true, Some(_), _) => return Err(ExplainSchemaError::AllWithSubject),
            (true, None, _)
                if edge.writing == tastemaker_core::domain::writing::types::Writing::DryRun =>
            {
                return Err(ExplainSchemaError::DryRun);
            }
            (true, None, Some(directory)) => {
                Asked::Everything(MachinePath(directory.display().to_string().into()))
            }
            (_, Some(subject), _) => Asked::Named(NameAsked(subject.into())),
            (_, None, _) => Asked::Workspace,
        };
        Ok(Self {
            start: edge.start.clone(),
            asked,
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `AllWithSubject` for `--all` with a subject.
pub(super) fn request(
    subject: Option<String>,
    all: bool,
    output_directory: Option<std::path::PathBuf>,
    edge: &crate::edge::Edge,
) -> Result<ExplainRequest, ExplainSchemaError> {
    ExplainArgumentsSchema {
        subject,
        all,
        output_directory,
        edge,
    }
    .try_into()
}

/// One section printed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct SectionSchema {
    /// The heading.
    pub title: String,
    /// Its lines.
    pub lines: Vec<String>,
}

/// What `explain` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The explanation.
    pub sections: Vec<SectionSchema>,
    /// The files written by `--all`.
    pub written: Vec<String>,
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for section in &self.sections {
            writeln!(f, "## {}\n", section.title)?;
            for line in &section.lines {
                writeln!(f, "{line}")?;
            }
            writeln!(f)?;
        }
        for file in &self.written {
            writeln!(f, "wrote {file}")?;
        }
        Ok(())
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: ExplainReport) -> AnswerSchema {
    AnswerSchema {
        sections: report
            .sections
            .into_iter()
            .map(|section| SectionSchema {
                title: section.title.0.to_string(),
                lines: section
                    .lines
                    .into_iter()
                    .map(|line| String::from(line.0))
                    .collect(),
            })
            .collect(),
        written: report
            .written
            .into_iter()
            .map(|path| path.0.to_string())
            .collect(),
    }
}

/// What `explain --help` says after the arguments: examples, every topic, the kinds a `<kind>/<name>` names, and how each
/// kind of asset is written.
pub(super) fn help() -> String {
    let topics = names()
        .iter()
        .map(|name| name.0)
        .collect::<Vec<_>>()
        .join(", ");
    let kinds = [
        RULE_WORD,
        SPEC_WORD,
        TEMPLATE_WORD,
        RELATION_WORD,
        OPERATOR_WORD,
    ]
    .map(|kind| format!("{kind}/<name>"))
    .join(", ");
    let steps = authoring()
        .iter()
        .map(|step| step.0)
        .collect::<Vec<_>>()
        .join("\n");
    format!("{EXAMPLES}\n\nTopics: {topics}\nKinds: {kinds}\n\nAuthoring:\n{steps}")
}

/// The examples `explain --help` opens with.
const EXAMPLES: &str = "Examples:\n  tm explain\n  tm explain tastemaker.toml\n  tm explain template/unchecked\n  tm explain relation/reference\n  tm explain rule";

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for ExplainSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::AllWithSubject | Self::DryRun => {
                crate::outcome::remedy::Next::Run("tm explain --all --output <dir>".to_owned())
            }
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

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
