//! The types of the `instance_params` concept: plain data, public fields and derives, with no constructor and no
//! inherent impl.

use std::collections::BTreeMap;

use crate::domain::captures::types::{Captures, Interpolations};
use crate::domain::entries::types::{EntryIdentity, ParamName, SpecEntry, Value};
use crate::domain::findings::types::Finding;
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::Scope;

/// One instance of a spec entry whose params are given values: the entry, its spec, where it is and what it captured.
#[derive(Clone, Copy, Debug)]
pub struct Instance<'a> {
    /// The spec entry's identity, as findings name it.
    pub identity: &'a EntryIdentity,
    /// The spec entry.
    pub entry: &'a SpecEntry,
    /// The spec it applies.
    pub spec: &'a crate::domain::definitions::types::SpecFile,
    /// The file the entry is written in, where a problem of its own params is reported.
    pub anchor: &'a RepoPath,
    /// The instance's directory.
    pub directory: &'a RepoPath,
    /// The scope the entry is written in.
    pub parent: &'a Scope,
    /// What the entry's `include` captured of the directory, with what the parent captured.
    pub captures: &'a Captures,
    /// The workspace's name, which a param `from = "workspace-name"` takes.
    pub workspace: &'a crate::domain::findings::types::WorkspaceName,
    /// How a param's value is written as a path or a template reads it.
    pub written: &'a dyn crate::domain::entries::types::ValueWriter,
}

/// The value of each param of a spec at one instance, as text and typed, and each problem as a `#params` finding.
#[derive(Clone, Debug, Default)]
pub struct InstanceParams {
    /// Each param's value as text, as a template or a pattern reads it.
    pub values: Interpolations,
    /// Each param's value as written.
    pub typed: BTreeMap<ParamName, Value>,
    /// What is wrong.
    pub findings: Vec<Finding>,
}
