//! The types of the `init` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use std::sync::Arc;

use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::domain::ratchet::types::Writing;
use crate::outbound::hooks::interface::HooksPort;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::manifest::interface::ManifestPort;

/// What `init` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitRequest {
    /// Where to initialize: this directory, in a git repository.
    pub start: ProductRoot,
    /// Whether anything is written.
    pub writing: Writing,
}

/// Whether the product's `asymptote.toml` was there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Configuration {
    /// There was none: a minimal one, and an empty lock, are written.
    Created,
    /// One was there, and is left as it is.
    Kept,
}

/// What `init` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitReport {
    /// The product.
    pub root: ProductRoot,
    /// Whether its `asymptote.toml` was written.
    pub configuration: Configuration,
    /// The hooks whose script is written, or would be.
    pub installed: Vec<HookName>,
    /// The hooks whose script, asymptote's own, is removed, or would be: they are no longer in `[hooks]`.
    pub removed: Vec<HookName>,
    /// Whether anything was written.
    pub writing: Writing,
}

/// The ports `init` reaches the outside through.
#[derive(Debug)]
pub struct InitPorts {
    /// The product's `asymptote.toml`.
    pub manifest: Arc<dyn ManifestPort>,
    /// The lock, which an empty operation is written to.
    pub lock: Arc<dyn LockPort>,
    /// The repository's hook scripts.
    pub hooks: Arc<dyn HooksPort>,
}
