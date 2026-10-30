//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::packages::service::{kind_of, kind_word};
use tastemaker_core::domain::packages::types::{DefinitionKind, DefinitionName};
use tastemaker_core::inbound::use_cases::create::types::{
    CreateReport, CreateRequest, Destination, Extension, ScenarioExtension,
};

/// The one value a skeleton reads from `--set`: the extension its scenario files end in.
const EXTENSION: &str = "extension";

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CreateSchemaError {
    /// Not `<kind>/<name>`.
    #[error("`{0}` is not `rule/<name>`, `spec/<name>` or `template/<name>`")]
    Definition(String),
    /// A name that is not kebab-case.
    #[error("`{0}` is not kebab-case: a name is lowercase words and digits joined by `-`")]
    Name(String),
    /// A `--set` without `=`.
    #[error("`--set {0}` names no value: write `--set name=value`")]
    Setting(String),
    /// A `--set` of a name no skeleton reads.
    #[error("`--set {0}` names a value no skeleton reads: `extension` is the one")]
    Unread(String),
    /// An `extension` no linked language reads: the scenarios of files ending in it would find nothing.
    #[error(
        "`--set extension={0}`: no linked language reads `.{0}` files, so the scenarios would find nothing (`tm -V` names the languages linked)"
    )]
    Extension(String),
}

/// The arguments as the command line gives them, offered to the conversion.
struct CreateArgumentsSchema<'a> {
    /// What to create: `rule/<name>`, `spec/<name>` or `template/<name>`.
    definition: String,
    /// The package directory it is created in; the workspace's first package by path when left out.
    package: Option<std::path::PathBuf>,
    /// A value the skeleton reads, `name=value`; repeat for more. A rule and a template read `extension`, the
    /// extension their scenario files end in, which Tastemaker never guesses. A name no skeleton reads is refused.
    set: Vec<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
    /// The languages this build links, which say whether they read a file of the extension named.
    languages: &'a ryo::linter::Languages,
}

impl TryFrom<CreateArgumentsSchema<'_>> for CreateRequest {
    type Error = CreateSchemaError;

    fn try_from(schema: CreateArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let refused = || CreateSchemaError::Definition(schema.definition.clone());
        let (kind, name) = schema.definition.split_once('/').ok_or_else(refused)?;
        let kind = kind_of(tastemaker_core::domain::packages::types::KindWord(kind))
            .ok_or_else(refused)?;
        if name.is_empty() || name.contains('/') {
            return Err(refused());
        }
        // A name is kebab-case, as every name of a Tastemaker file is: lowercase words and digits joined by single `-`.
        if !name.split('-').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit())
        }) {
            return Err(CreateSchemaError::Name(name.to_owned()));
        }
        let mut extension = Extension::Unnamed;
        for setting in &schema.set {
            let (named, value) = setting
                .split_once('=')
                .ok_or_else(|| CreateSchemaError::Setting(setting.clone()))?;
            if named != EXTENSION {
                return Err(CreateSchemaError::Unread(named.to_owned()));
            }
            let value = value.trim_start_matches('.');
            // A scenario file ends in the extension named: one no linked language reads would never be read.
            let scenario = ryo::linter::RelativePath(format!("scenario.{value}").into());
            if schema.languages.reads(&scenario) == ryo::languages::Recognition::Declines {
                return Err(CreateSchemaError::Extension(value.to_owned()));
            }
            extension = Extension::Named(ScenarioExtension(value.into()));
        }
        Ok(Self {
            start: edge.start.clone(),
            kind,
            name: DefinitionName(name.into()),
            into: schema
                .package
                .as_ref()
                .map_or(Destination::Workspace, |package| {
                    Destination::Package(MachinePath(
                        std::path::Path::new(&*edge.start.0)
                            .join(package)
                            .display()
                            .to_string()
                            .into(),
                    ))
                }),
            extension,
            writing: edge.writing,
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `Definition` when it is not `<kind>/<name>`; `Name` when the name is not kebab-case; `Setting` for a `--set` with no value; `Unread` for a `--set` of a
/// name no skeleton reads; `Extension` for an `extension` no language of `languages` reads.
pub(super) fn request(
    definition: String,
    package: Option<std::path::PathBuf>,
    set: Vec<String>,
    edge: &crate::edge::Edge,
    languages: &ryo::linter::Languages,
) -> Result<CreateRequest, CreateSchemaError> {
    CreateArgumentsSchema {
        definition,
        package,
        set,
        edge,
        languages,
    }
    .try_into()
}

/// What `create` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The file written, relative to the directory `tm` runs in when it is beneath it.
    pub file: String,
    /// What to write in it, and the command that proves it.
    pub next: String,
    /// Whether nothing was written.
    pub dry_run: bool,
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = crate::output::dry_run::written(self.dry_run);
        writeln!(f, "{verb} {}", self.file)?;
        if self.dry_run {
            return Ok(());
        }
        writeln!(f, "next: {}", self.next)
    }
}

/// The answer printed for the service's report: the file named from `start`, the directory `tm` runs in, when it is
/// beneath it, so no machine path is printed for a file of the project.
pub(super) fn answer(report: &CreateReport, dry_run: bool, start: &MachinePath) -> AnswerSchema {
    let written = std::path::Path::new(&*report.package.0).join(&*report.file.0);
    let file = written
        .strip_prefix(&*start.0)
        .unwrap_or(&written)
        .display()
        .to_string();
    let word = kind_word(report.kind).0;
    let write = match report.kind {
        DefinitionKind::Rule => "a failing and a passing [[scenarios]], then the query",
        DefinitionKind::Spec => "its [[scenarios]], then its entries",
        DefinitionKind::Template => "its content, then its [[scenarios]]",
    };
    AnswerSchema {
        file,
        next: format!(
            "write {write} (`tm explain {word}`), then run `tm validate {word}/{}`",
            report.name.0
        ),
        dry_run,
    }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for CreateSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::Definition(_) => crate::outcome::remedy::Next::Fix(
                "write `rule/<name>`, `spec/<name>` or `template/<name>`".to_owned(),
            ),
            Self::Name(_) => crate::outcome::remedy::Next::Fix(
                "name it in kebab-case, as `rule/no-tsv`".to_owned(),
            ),
            Self::Setting(_) => {
                crate::outcome::remedy::Next::Fix("write each `--set` as `name=value`".to_owned())
            }
            Self::Unread(_) => crate::outcome::remedy::Next::Fix(
                "name the scenario files' extension with `--set extension=<ext>`".to_owned(),
            ),
            Self::Extension(_) => crate::outcome::remedy::Next::Fix(
                "name an extension a linked language reads with `--set extension=<ext>`, as `--set extension=rs`".to_owned(),
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
