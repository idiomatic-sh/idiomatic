//! The connections of `workspace check`: it opens the stores the edge reads and writes and returns their
//! clients. This is the only I/O of the edge; the composition takes the clients it returns and only constructs.

/// The clients of the stores `workspace check` reaches, opened once: where the store is, the workers the run's one pool
/// has. Values only: each adapter opens what it reads itself.
#[derive(Debug)]
pub(crate) struct WorkspaceCheckConnections {
    /// Where the project's store is, the build that writes it, and whether it is written.
    pub store: tastemaker_core::domain::settings::types::Store,
    /// How many workers the run's one pool has: one a core of the machine.
    pub workers: tastemaker_core::outbound::scheduler::types::WorkerCount,
}

/// Opens the stores and returns their clients: the store of the workspace nearest `start`, under `build`, written
/// unless `writing` says `-n/--dry-run`.
pub(crate) fn connect(
    store: &tastemaker_core::domain::files::types::DirectoryName,
    start: &tastemaker_core::domain::files::types::MachinePath,
    build: &tastemaker_core::domain::settings::types::Build,
    writing: tastemaker_core::domain::writing::types::Writing,
) -> WorkspaceCheckConnections {
    let start = std::path::Path::new(&*start.0);
    let workspace = start
        .ancestors()
        .find(|path| {
            path.join(tastemaker_core::domain::definitions::constants::WORKSPACE_FILE)
                .is_file()
        })
        .unwrap_or(start);
    let directory = workspace.join(&*store.0);
    WorkspaceCheckConnections {
        store: tastemaker_core::domain::settings::types::Store {
            directory: tastemaker_core::domain::files::types::MachinePath(
                directory.display().to_string().into(),
            ),
            build: build.clone(),
            writing,
        },
        workers: tastemaker_core::outbound::scheduler::types::WorkerCount(
            std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
        ),
    }
}
