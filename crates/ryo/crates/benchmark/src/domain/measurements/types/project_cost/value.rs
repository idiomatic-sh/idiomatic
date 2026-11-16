//! The candidate's cost over one project: each condition's runs and the store it leaves.
use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;

/// One condition of the candidate over every trial: its CPU and wall time and its peak resident set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunCost {
    /// User plus system CPU of the candidate process, one per trial.
    pub cpu: Vec<std::time::Duration>,
    /// Wall time of the candidate process, one per trial.
    pub wall: Vec<std::time::Duration>,
    /// Peak resident bytes of the candidate process, one per trial.
    pub peak: Vec<ResidentBytes>,
}

/// The bytes of a store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreBytes(pub u64);

/// The files of a store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreFiles(pub u64);

/// The bytes and files of the store a cold run leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreSize {
    /// Total bytes.
    pub bytes: StoreBytes,
    /// File count.
    pub files: StoreFiles,
}

/// The candidate's cost over one project of the language's benchmark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectCost {
    /// The project, as benchmark.toml names it.
    pub project:
        ryo_benchmark_contracts::domain::workloads::types::project_name::value::ProjectName,
    /// Empty store.
    pub cold: RunCost,
    /// Retained store, sources unchanged.
    pub warm: RunCost,
    /// Retained store after the project's patch.
    pub edited: RunCost,
    /// The store the cold run left.
    pub store: StoreSize,
}

/// Every project's cost, in benchmark.toml order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectCosts(pub Vec<ProjectCost>);
