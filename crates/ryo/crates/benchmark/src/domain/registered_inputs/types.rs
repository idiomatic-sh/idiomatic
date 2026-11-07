//! Data owned by the registered input identities.
use ryo::linter::{GivenProjectInventory, RelativePath};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use std::collections::BTreeMap;

/// The words of a refusal that a library answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RefusalText(pub String);

/// Whether the profile requires an execute-bit observation of every file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservationDeclaration {
    /// Every registered file must carry its observed execute bit.
    Required,
    /// No file observation is declared.
    Absent,
}

/// What a registered case declares about its files and project scope.
#[derive(Clone, Debug)]
pub(crate) struct RegisteredInputs {
    /// The case's source files.
    pub files: Vec<RelativePath>,
    /// Whether a file observation is declared.
    pub observation: ObservationDeclaration,
    /// The declared execute bit of each file.
    pub metadata: Option<BTreeMap<RelativePath, FileExecutable>>,
    /// The declared project search roots.
    pub search_roots: Option<Vec<RelativePath>>,
    /// The declared project entries.
    pub inventory: GivenProjectInventory,
}
