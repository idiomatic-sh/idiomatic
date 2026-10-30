//! What the user does next about each error a command can end with (clig.dev, Errors),
//! printed as `help:` under it by `crate::output::fail`. Each product implements
//! [`Remedy`] here for every error type its commands fail with, and every match names
//! every variant: a wildcard is refused, so a new variant does not compile until it is
//! given its next step.
#![deny(clippy::wildcard_enum_match_arm)]

use tastemaker_core::inbound::use_cases::{
    add, check, create, explain, list, scaffold, update, validate,
};
use tastemaker_core::outbound::analysis::errors::AnalysisError;
use tastemaker_core::outbound::manifest::errors::ManifestError;
use tastemaker_core::outbound::packages::errors::PackagesError;
use tastemaker_core::outbound::repository::errors::RepositoryError;

#[cfg(test)]
#[path = "remedy_test.rs"]
mod tests;

/// What the user does next about an error, printed as `help:` under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// A command line that moves the user on.
    Run(String),
    /// What to change, said as an instruction.
    Fix(String),
    /// Nothing the user did: a bug, to report at the repository's issues.
    Report,
}

impl std::fmt::Display for Next {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Run(command) => write!(f, "run `{command}`"),
            Self::Fix(change) => f.write_str(change),
            Self::Report => write!(
                f,
                "this is a bug in {} {}; please report it, with the command line you ran: {}/issues/new",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_REPOSITORY"),
            ),
        }
    }
}

/// What the user does about a path given on the command line that is not text.
#[must_use]
pub fn path_not_text() -> Next {
    Next::Fix("name the path in UTF-8".to_owned())
}

/// An error that says what to do about it.
pub trait Remedy {
    /// What the user does next.
    fn next(&self) -> Next;
}

/// `--timings` could not install its subscriber: another was installed first, which no command line the user writes
/// can cause.
impl Remedy for tracing::subscriber::SetGlobalDefaultError {
    fn next(&self) -> Next {
        Next::Report
    }
}

/// The next step when no workspace or git repository holds the command, or its files cannot be read or written.
fn repository(error: &RepositoryError) -> Next {
    match error {
        RepositoryError::NoWorkspace { .. } => Next::Fix("run in a workspace, or name one with `--root <dir>`: a directory at or above it holds a `tastemaker.toml`".to_owned()),
        RepositoryError::NoRepository { .. } => Next::Fix("run `git init` at the repository's root: Tastemaker reads a workspace inside a git repository".to_owned()),
        RepositoryError::Read { .. } => Next::Fix("make the path readable, or remove it".to_owned()),
        RepositoryError::Write { .. } => Next::Fix("make the directory writable, or free space on its disk".to_owned()),
    }
}

/// The next step for a `tastemaker.toml` or other manifest that is malformed.
fn manifest(error: &ManifestError) -> Next {
    match error {
        ManifestError::Shape { .. } => Next::Fix(
            "correct the file where the message says; `tm explain <topic>` shows each file's keys"
                .to_owned(),
        ),
        ManifestError::LockVersion { .. } => Next::Run("tm check --fix".to_owned()),
        // ignore: idiomatic.arch/no-language-names - the repository's installer is a file named `install.py`; the text points at that file
        ManifestError::Needs { needs, .. } => Next::Fix(format!(
            "install tastemaker {needs}, the version the workspace's `tastemaker.toml` needs: in a repository that has one, run `uv run --script tools/install.py`, which installs every tool at the version the repository declares; elsewhere install it the way Tastemaker's README installs it",
        )),
    }
}

/// The next step for a package dependency that has an unsupported source, is missing or cannot be read.
fn packages(error: &PackagesError) -> Next {
    match error {
        PackagesError::Unsupported { .. } => Next::Fix(
            "give the dependency a `path`: git and registry sources come with the registry"
                .to_owned(),
        ),
        PackagesError::Missing { .. } => {
            Next::Fix("correct the dependency's `path` in the root tastemaker.toml".to_owned())
        }
        PackagesError::Read { .. } => Next::Fix("make the package's files readable".to_owned()),
    }
}

/// The next step when a rule fails to evaluate, or the analysis engine itself fails.
fn analysis(error: &AnalysisError) -> Next {
    match error {
        AnalysisError::Rule { .. } => Next::Run("tm validate".to_owned()),
        AnalysisError::Engine { .. } => Next::Report,
    }
}

/// The next step for an entry whose paths place nothing.
fn placement() -> Next {
    Next::Fix("correct the entry's paths where the message says".to_owned())
}

impl Remedy for check::errors::CheckError {
    fn next(&self) -> Next {
        use check::errors::CheckError;
        match self {
            CheckError::Scheduler(_) => Next::Report,
            CheckError::Repository(error) => repository(error),
            CheckError::Manifest { source, .. } => manifest(source),
            CheckError::Packages(error) => packages(error),
            CheckError::NotARoot { .. } => {
                Next::Fix("run from the workspace's root, or pass `--root <dir>`".to_owned())
            }
            CheckError::Resolution { .. } => Next::Fix(
                "name a package the root tastemaker.toml depends on, and a definition it holds"
                    .to_owned(),
            ),
            CheckError::Placement(_) => placement(),
            CheckError::Analysis(error) => analysis(error),
            CheckError::Within { .. } => {
                Next::Fix("name a path that holds a file of the workspace, or none".to_owned())
            }
            CheckError::Lock { .. } => Next::Run("tm check --fix".to_owned()),
            CheckError::Proof { .. } => Next::Run("tm validate".to_owned()),
        }
    }
}

impl Remedy for validate::errors::ValidateError {
    fn next(&self) -> Next {
        use validate::errors::ValidateError;
        match self {
            ValidateError::Scheduler(_) => Next::Report,
            ValidateError::Repository(error) => repository(error),
            ValidateError::Manifest { source, .. } => manifest(source),
            ValidateError::Packages(error) => packages(error),
            ValidateError::NotARoot { .. } => {
                Next::Fix("run from the workspace's root, or pass `--root <dir>`".to_owned())
            }
            ValidateError::Stray { .. } => Next::Fix(
                "move the file into its definition's version directory, or delete it".to_owned(),
            ),
            ValidateError::Unread { .. } => Next::Fix(
                "delete each version's directory: a workspace that needs one again pins it, and a new version is offered as the latest"
                    .to_owned(),
            ),
            ValidateError::NoMatch => Next::Fix(
                "use `tm list` to find a definition, or omit the filter to validate every version"
                    .to_owned(),
            ),
            ValidateError::Analysis(error) => analysis(error),
        }
    }
}

impl Remedy for scaffold::errors::ScaffoldError {
    fn next(&self) -> Next {
        use scaffold::errors::ScaffoldError;
        match self {
            ScaffoldError::Analysis(_) | ScaffoldError::Undecided { .. } | ScaffoldError::Unsettled => Next::Run("tm check".to_owned()),
            ScaffoldError::InvalidPath { .. } => Next::Fix("use a relative scaffold target inside the repository".to_owned()),
            ScaffoldError::UnresolvedAt { .. } => {
                Next::Fix("pass `--at` the directory of one instance, each capture replaced by its name".to_owned())
            }
            ScaffoldError::OutsideTarget { .. } => {
                Next::Fix("pass `--at` a directory the entry's `include` selects".to_owned())
            }
            ScaffoldError::Unread { .. } => Next::Fix("correct the `--set` name, or drop it".to_owned()),
            ScaffoldError::Repository(error) => repository(error),
            ScaffoldError::Manifest { source, .. } => manifest(source),
            ScaffoldError::Packages(error) => packages(error),
            ScaffoldError::Placement(_) => placement(),
            ScaffoldError::NoEntry { .. } => Next::Run("tm explain".to_owned()),
            ScaffoldError::Capture { .. } | ScaffoldError::Param { .. } => {
                Next::Fix("pass the capture or the param the message names with `--set name=value`".to_owned())
            }
            ScaffoldError::Render { .. } => {
                Next::Fix("pass the params the template reads with `--set name=value`".to_owned())
            }
            ScaffoldError::Unwritten { .. } => Next::Fix(
                "run the tool that writes the file, or write it yourself: `tm check` then places it".to_owned(),
            ),
        }
    }
}

impl Remedy for explain::errors::ExplainError {
    fn next(&self) -> Next {
        use explain::errors::ExplainError;
        match self {
            ExplainError::Repository(error) => repository(error),
            ExplainError::Manifest { source, .. } => manifest(source),
            ExplainError::Packages(error) => packages(error),
            ExplainError::Placement(_) => placement(),
            ExplainError::Analysis(error) => analysis(error),
            ExplainError::Unknown { .. } => Next::Run("tm explain".to_owned()),
        }
    }
}

impl Remedy for list::errors::ListError {
    fn next(&self) -> Next {
        use list::errors::ListError;
        match self {
            ListError::Repository(error) => repository(error),
            ListError::Manifest { source, .. } => manifest(source),
            ListError::Packages(error) => packages(error),
            ListError::Analysis(error) => analysis(error),
            ListError::NoMatch { .. } => Next::Run("tm list".to_owned()),
        }
    }
}

impl Remedy for add::errors::AddError {
    fn next(&self) -> Next {
        use add::errors::AddError;
        match self {
            AddError::Repository(error) => repository(error),
            AddError::Manifest { source, .. } => manifest(source),
            AddError::Packages(error) => packages(error),
            AddError::Resolution { .. } => {
                Next::Fix("repair the workspace definitions before changing entries".to_owned())
            }
            AddError::Relock { .. } => Next::Run("tm check --fix".to_owned()),
            AddError::NoDefinition { .. } => Next::Run("tm list".to_owned()),
        }
    }
}

impl Remedy for create::errors::CreateError {
    fn next(&self) -> Next {
        use create::errors::CreateError;
        match self {
            CreateError::Repository(error) => repository(error),
            CreateError::Manifest { source, .. } => manifest(source),
            CreateError::Packages(error) => packages(error),
            CreateError::NoPackage => Next::Fix("pass `--package <dir>`".to_owned()),
            CreateError::NoExtension { kind, name } => Next::Fix(format!(
                "name the scenario files' extension: `tm create {kind}/{name} --set extension=<ext>`"
            )),
            CreateError::Exists { .. } => {
                Next::Fix("edit the definition's working version in place".to_owned())
            }
        }
    }
}

impl Remedy for update::errors::UpdateError {
    fn next(&self) -> Next {
        use update::errors::UpdateError;
        match self {
            UpdateError::Repository(error) => repository(error),
            UpdateError::Manifest { source, .. } => manifest(source),
            UpdateError::Packages(error) => packages(error),
            UpdateError::Resolution { .. } => {
                Next::Fix("repair the workspace definitions before changing entries".to_owned())
            }
            UpdateError::Relock { .. } => Next::Run("tm check --fix".to_owned()),
            UpdateError::NoEntry { .. } | UpdateError::NoVersion { .. } => {
                Next::Run("tm update -n".to_owned())
            }
        }
    }
}

impl Remedy for tastemaker_core::inbound::use_cases::init::errors::InitError {
    fn next(&self) -> Next {
        match self {
            Self::Exists { .. } => Next::Fix("use the existing configuration, or choose an uninitialized directory with `--root`".to_owned()),
            Self::Repository(error) => repository(error),
        }
    }
}
