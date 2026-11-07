//! What the user does next about each error a command can end with (clig.dev, Errors),
//! printed as `help:` under it by `crate::output::fail`. Each product implements
//! [`Remedy`] here for every error type its commands fail with, and every match names
//! every variant: a wildcard is refused, so a new variant does not compile until it is
//! given its next step.
#![deny(clippy::wildcard_enum_match_arm)]

use asymptote_core::domain::operations::errors::OperationsError;
use asymptote_core::inbound::use_cases::check::errors::CheckError;
use asymptote_core::inbound::use_cases::down::errors::DownError;
use asymptote_core::inbound::use_cases::init::errors::InitError;
use asymptote_core::inbound::use_cases::up::errors::UpError;
use asymptote_core::outbound::check_store::errors::CheckStoreError;
use asymptote_core::outbound::hooks::errors::HooksError;
use asymptote_core::outbound::lock::errors::LockError;
use asymptote_core::outbound::manifest::errors::ManifestError;
use asymptote_core::outbound::repository::errors::RepositoryError;

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

/// The next step for a manifest that is missing, unreadable, malformed or inconsistent with the workspace.
fn manifest(error: &ManifestError) -> Next {
    match error {
        ManifestError::Missing { .. } => Next::Fix(
            "run in a product, or name one with `--root <dir>`: a directory at or above it holds an `asymptote.toml`"
                .to_owned(),
        ),
        ManifestError::Unreadable { .. } => Next::Fix("make `asymptote.toml` readable".to_owned()),
        ManifestError::Shape { .. } => Next::Fix(
            "correct `asymptote.toml` where the message says: `[[tools]]` with `id`, `run`, `group` (such as lint, check, release or perf), and `fix`, `output = \"sarif\"` or `\"bmf\"`, `timeout-seconds` and `inputs` when needed; `members` in the workspace root"
                .to_owned(),
        ),
        ManifestError::Member { .. } => Next::Fix(
            "make each `members` glob of the root `asymptote.toml` match directories that each hold an `asymptote.toml` of their own, with no `members`"
                .to_owned(),
        ),
        ManifestError::Hook { .. } => Next::Fix(
            "map each `[hooks]` entry to groups some tool declares (`group = \"lint\"` ...), or declare the tool"
                .to_owned(),
        ),
        ManifestError::Unwritable { .. } => {
            Next::Fix("make the directory writable, or free space on its disk".to_owned())
        }
        ManifestError::Pinned { needed, .. } => Next::Fix(format!(
            "install asymptote {needed}: run `uv run --script tools/install.py`, which installs every tool at the version the repository declares",
        )),
        ManifestError::NotRoot { .. } => Next::Fix(
            "remove `members` from this `asymptote.toml`: only the workspace root, the topmost one, names members"
                .to_owned(),
        ),
    }
}

/// The next step when the git hooks cannot be found, read or written.
fn hooks(error: &HooksError) -> Next {
    match error {
        HooksError::NotARepository { .. } => {
            Next::Fix("run in a git repository: `git init`, then `asymptote init`".to_owned())
        }
        HooksError::Git { .. } => Next::Fix(
            "make git answer in the product: run `git rev-parse --git-path hooks` there".to_owned(),
        ),
        HooksError::Unreadable { .. } => Next::Fix("make the hooks directory readable".to_owned()),
        HooksError::Unwritable { .. } => {
            Next::Fix("make the hooks directory writable, or free space on its disk".to_owned())
        }
    }
}

/// The next step for a lock operation file that cannot be read, parsed or written.
fn lock(error: &LockError) -> Next {
    match error {
        LockError::Unreadable { .. } => Next::Fix("make `.asymptote/operations` readable".to_owned()),
        LockError::Malformed { .. } | LockError::Address { .. } => Next::Fix(
            "restore the operation from git (`git checkout -- .asymptote/operations`): an operation is never edited by hand"
                .to_owned(),
        ),
        LockError::Unwritable { .. } => Next::Fix("make `.asymptote/operations` writable, or free space on its disk".to_owned()),
    }
}

/// The next step when the operations do not fold into one lock: a parent is missing or a file was edited.
fn fold(error: &OperationsError) -> Next {
    match error {
        OperationsError::MissingParent { .. } => {
            Next::Fix("restore the lock's missing operations from git: a merge dropped a file of it".to_owned())
        }
        OperationsError::Invalid { .. } | OperationsError::Twice { .. } | OperationsError::Cycle => Next::Fix(
            "restore the operation from git (`git checkout -- .asymptote/operations`): an operation is never edited by hand"
                .to_owned(),
        ),
    }
}

/// The next step when git cannot answer the renames or the `--since` revision.
fn repository(error: &RepositoryError) -> Next {
    match error {
        RepositoryError::NotARepository { .. } => {
            Next::Fix("run in a git repository: renames and `--since` read its history".to_owned())
        }
        RepositoryError::UnknownRevision { .. } => Next::Fix(
            "name a revision this clone holds, or fetch it: `git fetch origin`".to_owned(),
        ),
        RepositoryError::Failed { .. } => {
            Next::Fix("make git answer in the product: run `git status` there".to_owned())
        }
    }
}

/// The next step when the store cannot be written.
fn store(error: &CheckStoreError) -> Next {
    match error {
        CheckStoreError::Unwritable { .. } => {
            Next::Fix("make `.asymptote/cache` writable, or free space on its disk".to_owned())
        }
    }
}

impl Remedy for CheckError {
    fn next(&self) -> Next {
        match self {
            Self::Manifest(error) => manifest(error),
            Self::Lock(error) => lock(error),
            Self::Fold(error) => fold(error),
            Self::Repository(error) => repository(error),
            Self::Store(error) => store(error),
            Self::NoHead => {
                Next::Fix("commit first, or check against the lock without `--since`".to_owned())
            }
        }
    }
}

impl Remedy for UpError {
    fn next(&self) -> Next {
        match self {
            Self::Manifest(error) => manifest(error),
            Self::Lock(error) => lock(error),
            Self::Fold(error) => fold(error),
            Self::Repository(error) => repository(error),
            Self::Matching(_) => Next::Fix("write each `--path` as a glob: `crates/core/**`, `src/*.rs`".to_owned()),
            Self::NoReport | Self::HeadMoved | Self::ManifestMoved | Self::Moved { .. } => Next::Run("asymptote check".to_owned()),
            Self::ToolsFailed { .. } => {
                Next::Fix("make every tool run, then `asymptote check`: what a failed tool reports is unknown".to_owned())
            }
            Self::NothingAt { nearest, .. } => Next::Fix(if nearest.is_empty() {
                "nothing is new: there is nothing to record".to_owned()
            } else {
                let places = nearest
                    .iter()
                    .map(|violation| match violation.place {
                        asymptote_core::domain::violations::types::Place::File => violation.path.0.to_string(),
                        asymptote_core::domain::violations::types::Place::Line(line)
                        | asymptote_core::domain::violations::types::Place::At(line, _) => {
                            format!("{}:{}", violation.path.0, line.0)
                        }
                    })
                    .collect::<Vec<_>>();
                format!("name a place `asymptote check` printed; the nearest new: {}", places.join(", "))
            }),
            Self::Outside { .. } => Next::Fix(
                "fix it, or have its tool report it beneath the product: a lock holds only the product's own files"
                    .to_owned(),
            ),
            Self::Unkeyed { .. } => Next::Fix(
                "make the file the finding is in readable beneath the product, or fix the finding".to_owned(),
            ),
        }
    }
}

impl Remedy for DownError {
    fn next(&self) -> Next {
        match self {
            Self::Manifest(error) => manifest(error),
            Self::Lock(error) => lock(error),
            Self::Fold(error) => fold(error),
            Self::Repository(error) => repository(error),
            Self::NoReport | Self::HeadMoved | Self::ManifestMoved | Self::Moved { .. } => Next::Run("asymptote check".to_owned()),
            Self::ToolsFailed { .. } => {
                Next::Fix("make every tool run, then `asymptote check`: what a failed tool reports is unknown".to_owned())
            }
        }
    }
}

impl Remedy for InitError {
    fn next(&self) -> Next {
        match self {
            Self::Manifest(error) => manifest(error),
            Self::Lock(error) => lock(error),
            Self::Hooks(error) => hooks(error),
            Self::Foreign { hook, .. } => Next::Fix(format!(
                "move what that script does into a hook asymptote does not own, or remove it, then `asymptote init`: asymptote checks the {hook} hook with `asymptote check --hook {hook}`"
            )),
        }
    }
}
