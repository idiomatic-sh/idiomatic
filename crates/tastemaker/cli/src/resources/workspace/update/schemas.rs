//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use tastemaker_core::domain::packages::service::version_number;
use tastemaker_core::domain::packages::types::{DefinitionName, VersionText};
use tastemaker_core::inbound::use_cases::update::types::{
    Entries, UpdateReport, UpdateRequest, VersionChoice,
};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum UpdateSchemaError {
    /// `--to` that is not a version.
    #[error("`--to {0}` is not a version `major.minor.patch`")]
    Version(String),
}

/// The arguments as the command line gives them, offered to the conversion.
struct UpdateArgumentsSchema<'a> {
    /// The entry to move, by its alias or its definition's name; every entry when left out.
    entry: Option<String>,
    /// The version to move to; the latest its package offers when left out.
    to: Option<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<UpdateArgumentsSchema<'_>> for UpdateRequest {
    type Error = UpdateSchemaError;

    fn try_from(schema: UpdateArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let target = match schema.to {
            Some(to) => {
                let version = VersionText(to.clone().into());
                if version_number(&version).is_none() {
                    return Err(UpdateSchemaError::Version(to));
                }
                VersionChoice::Version(version)
            }
            None => VersionChoice::Latest,
        };
        Ok(Self {
            start: edge.start.clone(),
            entries: schema.entry.map_or(Entries::Every, |entry| {
                Entries::Named(DefinitionName(entry.into()))
            }),
            target,
            writing: edge.writing,
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `Version` for a `--to` that is not a version.
pub(super) fn request(
    entry: Option<String>,
    to: Option<String>,
    edge: &crate::edge::Edge,
) -> Result<UpdateRequest, UpdateSchemaError> {
    UpdateArgumentsSchema { entry, to, edge }.try_into()
}

/// One entry moved.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct MoveSchema {
    /// The entry.
    pub entry: String,
    /// From.
    pub from: String,
    /// To.
    pub to: String,
    /// The changelog sections crossed by the move.
    pub changes: String,
}

/// What `update` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The file.
    pub file: String,
    /// The entries moved, or that would move.
    pub moves: Vec<MoveSchema>,
    /// Whether nothing was written.
    pub dry_run: bool,
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = if self.dry_run { "would move" } else { "moved" };
        for moved in &self.moves {
            writeln!(
                f,
                "{verb} {} from {} to {}",
                moved.entry, moved.from, moved.to
            )?;
            if !moved.changes.is_empty() {
                writeln!(f, "{}", moved.changes)?;
            }
        }
        if self.moves.is_empty() {
            writeln!(f, "every entry of {} follows its latest version", self.file)?;
        } else if !self.dry_run {
            writeln!(f, "next: tm check")?;
        } else {
            // A dry run wrote nothing, so there is no next step to say.
        }
        Ok(())
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: UpdateReport, dry_run: bool) -> AnswerSchema {
    AnswerSchema {
        file: report.file.0.to_string(),
        moves: report
            .moves
            .into_iter()
            .map(|moved| MoveSchema {
                entry: moved.entry.0.to_string(),
                from: moved.from.0.to_string(),
                to: moved.to.0.to_string(),
                changes: moved.changes.0.to_string(),
            })
            .collect(),
        dry_run,
    }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for UpdateSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::Version(_) => {
                crate::outcome::remedy::Next::Fix("pass `--to major.minor.patch`".to_owned())
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
