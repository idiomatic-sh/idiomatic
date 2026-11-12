//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use std::collections::BTreeMap;

use tastemaker_core::domain::captures::service::{pieces, written_piece};
use tastemaker_core::domain::captures::types::{PatternPiece, Template};
use tastemaker_core::domain::entries::types::Setting;
use tastemaker_core::domain::packages::types::DefinitionKind;
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::inbound::use_cases::add::types::{
    AddReport, AddRequest, DefinitionAsked, Novelty,
};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AddSchemaError {
    /// A `--param` without `=`.
    #[error("`--param {0}` names no value: write `--param name=value`")]
    Param(String),
    /// A path that is not text.
    #[error("the path {0:?} is not text")]
    PathNotText(std::path::PathBuf),
    /// An argument cannot be represented in the printed shell command.
    #[error("cannot print the scaffold command: {0}")]
    Command(#[from] shlex::QuoteError),
}

/// The arguments as the command line gives them, offered to the conversion.
struct AddArgumentsSchema<'a> {
    /// The definition: `name`, or `package/name`.
    definition: String,
    /// Where it applies, relative to the current directory: a rule's or spec's directory, a template's file.
    at: std::path::PathBuf,
    /// A value the entry passes, `name=value`; repeat for more.
    params: Vec<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<AddArgumentsSchema<'_>> for AddRequest {
    type Error = AddSchemaError;

    fn try_from(schema: AddArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let mut params = BTreeMap::new();
        for param in schema.params {
            let (name, value) = param
                .split_once('=')
                .ok_or_else(|| AddSchemaError::Param(param.clone()))?;
            drop(params.insert(Setting(name.into()), Setting(value.into())));
        }
        let at = schema
            .at
            .to_str()
            .ok_or_else(|| AddSchemaError::PathNotText(schema.at.clone()))?;
        Ok(Self {
            start: edge.start.clone(),
            definition: DefinitionAsked(schema.definition.into()),
            at: RepoPath(at.trim_end_matches('/').into()),
            params,
            writing: edge.writing,
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `Param` for a `--param` with no value, `PathNotText` for a path that is not text.
pub(super) fn request(
    definition: String,
    at: std::path::PathBuf,
    params: Vec<String>,
    edge: &crate::edge::Edge,
) -> Result<AddRequest, AddSchemaError> {
    AddArgumentsSchema {
        definition,
        at,
        params,
        edge,
    }
    .try_into()
}

/// What `add` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The file written.
    pub file: String,
    /// The entry's definition.
    pub entry: String,
    /// The command that writes its files.
    pub next: String,
    /// Whether nothing was written.
    pub dry_run: bool,
    /// Whether the entry is new to the file: it is said once as it is, never as added again. Not part of the JSON.
    #[serde(skip)]
    pub new: bool,
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.new {
            let verb = if self.dry_run { "would add" } else { "added" };
            writeln!(f, "{verb} {} to {}", self.entry, self.file)?;
        } else {
            writeln!(f, "{} already holds {}", self.file, self.entry)?;
        }
        writeln!(f, "next: {}", self.next)
    }
}

/// The answer printed for the service's report: a rule places no file, so `check` runs it next; a spec or a template is
/// scaffolded at `at`, each capture it still holds written `<name>` for the instance's name the user puts in its place.
/// The command is run where `add` ran, so it names no machine path of the project's; with a `--root` that is not where
/// `add` ran, `root` is that directory and the command names it, since it would find no workspace otherwise.
pub(super) fn answer(
    report: &AddReport,
    dry_run: bool,
    at: &RepoPath,
    root: Option<&str>,
) -> Result<AnswerSchema, AddSchemaError> {
    let next = if report.kind == DefinitionKind::Rule {
        rooted(root, vec!["tm", "check"])?
    } else {
        let instance: String = pieces(Template(&at.0))
            .into_iter()
            .map(|piece| match piece {
                PatternPiece::Capture(name)
                | PatternPiece::Span(name)
                | PatternPiece::Directory(name) => format!("<{}>", name.0),
                PatternPiece::Literal(character) => character.0.to_string(),
                // A wildcard, or a piece a later core adds, is written as the pattern writes it.
                other => written_piece(other).0,
            })
            .collect();
        rooted(
            root,
            vec![
                "tm",
                "scaffold",
                &*report.reference.0,
                "--at",
                instance.as_str(),
            ],
        )?
    };
    Ok(AnswerSchema {
        file: report.file.0.to_string(),
        entry: report.entry.0.to_string(),
        next,
        dry_run,
        new: report.novelty == Novelty::New,
    })
}

/// `command` as one shell line, with `--root <root>` after it when `root` is given.
fn rooted<'a>(
    root: Option<&'a str>,
    mut command: Vec<&'a str>,
) -> Result<String, shlex::QuoteError> {
    if let Some(root) = root {
        command.extend(["--root", root]);
    }
    shlex::try_join(command)
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for AddSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::Param(_) => {
                crate::outcome::remedy::Next::Fix("write each `--param` as `name=value`".to_owned())
            }
            Self::PathNotText(_) => crate::outcome::remedy::path_not_text(),
            Self::Command(_) => crate::outcome::remedy::Next::Fix(
                "use a path and definition name without NUL characters".to_owned(),
            ),
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
