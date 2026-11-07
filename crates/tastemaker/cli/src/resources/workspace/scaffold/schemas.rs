//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use std::collections::BTreeMap;

use tastemaker_core::domain::entries::types::Setting;
use tastemaker_core::domain::packages::types::DefinitionName;
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::inbound::use_cases::scaffold::types::{ScaffoldReport, ScaffoldRequest};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ScaffoldSchemaError {
    /// A `--set` without `=`.
    #[error("`--set {0}` names no value: write `--set name=value`")]
    Setting(String),
    /// A path that is not text.
    #[error("the path {0:?} is not text")]
    PathNotText(std::path::PathBuf),
}

/// The arguments as the command line gives them, offered to the conversion.
struct ScaffoldArgumentsSchema<'a> {
    /// The entry: its alias or its definition's name, or the name `tm explain` gives it (`package/name`,
    /// `package/spec/name`, `@version` after it), which names an entry of a spec instance at `--at`.
    entry: String,
    /// The directory a spec entry's instance is written at, relative to the current directory: one its `include` selects.
    at: std::path::PathBuf,
    /// A capture or param the files read, `name=value`; repeat for more. A name no file reads is refused.
    set: Vec<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<ScaffoldArgumentsSchema<'_>> for ScaffoldRequest {
    type Error = ScaffoldSchemaError;

    fn try_from(schema: ScaffoldArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let mut set = BTreeMap::new();
        for setting in schema.set {
            let (name, value) = setting
                .split_once('=')
                .ok_or_else(|| ScaffoldSchemaError::Setting(setting.clone()))?;
            drop(set.insert(Setting(name.into()), Setting(value.into())));
        }
        let at = schema
            .at
            .to_str()
            .ok_or_else(|| ScaffoldSchemaError::PathNotText(schema.at.clone()))?;
        Ok(Self {
            start: edge.start.clone(),
            entry: DefinitionName(schema.entry.into()),
            at: RepoPath(at.into()),
            set,
            writing: edge.writing,
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `Setting` for a `--set` with no value, `PathNotText` for a path that is not text.
pub(super) fn request(
    entry: String,
    at: std::path::PathBuf,
    set: Vec<String>,
    edge: &crate::edge::Edge,
) -> Result<ScaffoldRequest, ScaffoldSchemaError> {
    ScaffoldArgumentsSchema {
        entry,
        at,
        set,
        edge,
    }
    .try_into()
}

/// What `scaffold` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The files written, or that would be.
    pub written: Vec<String>,
    /// The files kept as they are.
    pub kept: Vec<String>,
    /// Whether nothing was written.
    pub dry_run: bool,
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = crate::output::dry_run::written(self.dry_run);
        for file in &self.written {
            writeln!(f, "{verb} {file}")?;
        }
        for file in &self.kept {
            writeln!(f, "kept {file}: it exists")?;
        }
        Ok(())
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: ScaffoldReport, dry_run: bool) -> AnswerSchema {
    AnswerSchema {
        written: report
            .written
            .into_iter()
            .map(|path| path.0.to_string())
            .collect(),
        kept: report
            .kept
            .into_iter()
            .map(|path| path.0.to_string())
            .collect(),
        dry_run,
    }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for ScaffoldSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::Setting(_) => {
                crate::outcome::remedy::Next::Fix("write each `--set` as `name=value`".to_owned())
            }
            Self::PathNotText(_) => crate::outcome::remedy::path_not_text(),
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
