//! The connections of `ratchet check`: it works out where the stores the edge reads and writes are kept, as plain
//! values, and returns them. Nothing is opened: each store's adapter reads and writes its own file. This is the only
//! place the edge decides where its stores live; the composition takes what it returns and only constructs.

use asymptote_core::domain::manifest::types::ProductRoot;
use asymptote_core::outbound::check_store::types::{Build, Store};

/// Where the stores `ratchet check` reaches are kept: the build that runs, which keeps them, and the directory beneath
/// the product's root they are kept in.
#[derive(Debug)]
pub(crate) struct RatchetCheckConnections {
    /// The build that runs: a store another build wrote reads as empty.
    pub build: Build,
    /// The directory beneath the product's root where the store is kept.
    pub cache: std::path::PathBuf,
}

/// Works out where the stores are kept: beneath the product's root in `cache`, by `build`.
#[must_use]
pub(crate) fn connect(build: &Build, cache: &std::path::Path) -> RatchetCheckConnections {
    RatchetCheckConnections {
        build: build.clone(),
        cache: cache.to_path_buf(),
    }
}

/// The check store of the product at `root`: its cache directory, kept by the build that runs.
pub(crate) fn store(connections: &RatchetCheckConnections, root: &ProductRoot) -> Store {
    Store {
        directory: std::path::Path::new(&*root.0).join(&connections.cache),
        build: connections.build.clone(),
    }
}
