//! Immutable inputs and independently pinned shipped defaults.
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Suite {
    pub language: String,
    pub projects: Vec<Project>,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub repository: String,
    pub revision: String,
    pub patch: String,
}

#[derive(Clone, Debug)]
pub struct Tool {
    pub tool: String,
    pub version: String,
    pub sha256: String,
    pub defaults: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Cases {
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug)]
pub struct Case {
    pub name: String,
    pub files: BTreeMap<String, String>,
    pub expected: Vec<Expected>,
}

#[derive(Clone, Debug)]
pub struct Expected {
    pub file: String,
    pub start: usize,
    pub end: usize,
}

/// Both contestants receive exactly this list, never independently walk the tree.
#[derive(Clone, Debug)]
pub struct RunRequest {
    pub root: PathBuf,
    pub files: Vec<String>,
    pub cache: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Audit {
    pub implemented: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct WorkerRequest {
    pub root: PathBuf,
    pub rules: PathBuf,
    pub files: Vec<String>,
    pub cache: Option<PathBuf>,
}
