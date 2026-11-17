//! The test support of the `definitions` concept: a workspace loaded with nothing but its files.

use std::collections::BTreeMap;

use crate::domain::definitions::types::{LoadedWorkspace, WorkspaceSection};
use crate::domain::entries::types::Composition;
use crate::domain::findings::types::WorkspaceName;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::Selectors;

/// A workspace `w` at the root that composes nothing, depends on no package and lists `files`.
#[must_use]
pub fn bare(files: &[&str]) -> LoadedWorkspace {
    LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section: WorkspaceSection {
            name: WorkspaceName("w".into()),
            rules_exclude: Selectors::default(),
            members: Selectors::default(),
            dependencies: Vec::new(),
        },
        composition: Composition::default(),
        locals: Vec::new(),
        packages: BTreeMap::new(),
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: files.iter().map(|file| RepoPath((*file).into())).collect(),
        local_packages: Vec::new(),
    }
}
