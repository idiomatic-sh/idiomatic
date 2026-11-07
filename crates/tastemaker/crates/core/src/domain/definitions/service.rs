//! The `definitions` service: free functions named by verb that load a workspace's definitions, with no IO and no ports.
//! The caller reads each file the load asks for and hands back what it parsed; the load says what to read next.

use std::collections::BTreeMap;

use super::constants::{ACYCLIC_KEY, INDEPENDENT_KEY, LAYERS_KEY, WORKSPACE_FILE};
use super::types::{
    Definition, FoldedKey, LoadedWorkspace, LocalFile, PackagePath, RepositoryText, Role,
    TemplateFile, TemplateText,
};
use crate::domain::entries::types::Description;
use crate::domain::entries::types::{Acyclicity, Amount, Composition};
use crate::domain::findings::constants::{FORMAT, TASTEMAKER};
use crate::domain::findings::service::check_id;
use crate::domain::findings::service::finding;
use crate::domain::findings::types::{Finding, Help, Place};
use crate::domain::packages::types::{Ownership, Release, ResolvedPackage};
use crate::domain::paths::service::{name, parent};
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::Coverage;
use crate::domain::variables::types::VariableProblem;

/// The template's content.
#[must_use]
pub fn content(template: &TemplateFile) -> TemplateText {
    template.content.clone()
}

/// What a definition says it is, of whichever kind it is.
#[must_use]
pub fn description(definition: &Definition) -> Description {
    match definition {
        Definition::Rule(rule) => rule.description.clone(),
        Definition::Spec(spec) => spec.description.clone(),
        Definition::Template(template) => template.description.clone(),
    }
}

/// The files each scenario of a definition writes, of whichever kind it is.
#[must_use]
pub fn scenario_files(
    definition: &Definition,
) -> Vec<&crate::domain::scenarios::types::ScenarioFiles> {
    match definition {
        Definition::Rule(rule) => rule
            .scenarios
            .iter()
            .map(|scenario| &scenario.files)
            .collect(),
        Definition::Spec(spec) => spec
            .scenarios
            .iter()
            .map(|scenario| &scenario.files)
            .collect(),
        Definition::Template(template) => template
            .scenarios
            .iter()
            .map(|scenario| &scenario.files)
            .collect(),
    }
}

/// Every shared release the workspace uses, for the lock and for `latest`.
#[must_use]
pub fn shared(workspace: &LoadedWorkspace) -> Vec<Release> {
    workspace
        .specs
        .keys()
        .chain(workspace.templates.keys())
        .chain(workspace.rules.keys())
        .filter(|release| release.ownership == Ownership::Shared)
        .cloned()
        .collect()
}

/// The files a workspace governs among `files`, every file beneath its directory: those of a nested workspace are
/// left out, the tree below a directory that holds a `tastemaker.toml` of its own no member directory names and whose
/// role `role_of` reads as a workspace's root file, one that declares a `workspace`. A `tastemaker.toml` that declares none
/// (a fixture, a local file outside the members) prunes nothing; one that does not read stops the walk with `role_of`'s
/// error, since whether it roots a workspace of its own cannot be told. A symbolic link is never in `files`.
///
/// # Errors
///
/// `role_of`'s, for the first nested `tastemaker.toml` that does not read.
pub fn governed<E>(
    files: Vec<RepoPath>,
    root_file: &RepoPath,
    member: impl Fn(&RepoPath) -> Coverage,
    role_of: impl Fn(&RepoPath) -> Result<Role, E>,
) -> Result<Vec<RepoPath>, E> {
    let mut nested = Vec::new();
    for file in files
        .iter()
        .filter(|file| &*name(file).0 == WORKSPACE_FILE && *file != root_file)
    {
        if let Role::Root(_) = role_of(file)? {
            nested
                .extend(parent(file).filter(|directory| member(directory) == Coverage::Uncovered));
        }
    }
    if nested.is_empty() {
        return Ok(files);
    }
    Ok(files
        .into_iter()
        .filter(|file| {
            !nested
                .iter()
                .any(|directory| file.0.starts_with(&*format!("{}/", directory.0)))
        })
        .collect())
}

/// The local `tastemaker.toml` files among `files`: each under a member directory, none under the workspace's
/// `rules-exclude` (answered by `excluded`) or beneath another. The caller reads and parses each.
#[must_use]
pub fn locals<'f>(
    files: &'f [RepoPath],
    root_file: &RepoPath,
    member: impl Fn(&RepoPath) -> Coverage,
    excluded: impl Fn(&RepoPath) -> Coverage,
) -> Vec<&'f RepoPath> {
    files
        .iter()
        .filter(|file| &*name(file).0 == WORKSPACE_FILE && *file != root_file)
        .filter(|file| excluded(file) == Coverage::Uncovered)
        .filter(|file| {
            parent(file).is_some_and(|directory| member(&directory) == Coverage::Covered)
        })
        .collect()
}

/// A local file read.
#[must_use]
pub fn local(file: &RepoPath, composition: Composition) -> LocalFile {
    LocalFile {
        file: file.clone(),
        directory: parent(file).unwrap_or_default(),
        composition,
    }
}

/// The `tastemaker#format` findings a composition's folded keys make: `layers`, `independent` and `acyclic` are folded
/// into `import-directions` placements, so each one written is to be converted.
#[must_use]
pub fn folded(composition: &Composition, file: &RepoPath) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut folded_key = |key: FoldedKey| {
        findings.push(finding(
            check_id(TASTEMAKER, FORMAT),
            file.clone(),
            Place::Whole,
            format!("`{}` is folded into `import-directions` placements: convert it to entries of that rule, one for each direction", match key {
                FoldedKey::Layers => LAYERS_KEY,
                FoldedKey::Independent => INDEPENDENT_KEY,
                FoldedKey::Acyclic => ACYCLIC_KEY,
            }),
            Help::Silent,
        ));
    };
    if composition.ordering.layers > Amount(0) {
        folded_key(FoldedKey::Layers);
    }
    if composition.ordering.independent > Amount(0) {
        folded_key(FoldedKey::Independent);
    }
    if composition.ordering.acyclic == Acyclicity::Written {
        folded_key(FoldedKey::Acyclic);
    }
    findings
}

/// The `tastemaker#format` errors a file's variable problems make.
#[must_use]
pub fn variable_findings(problems: &[VariableProblem], file: &RepoPath) -> Vec<Finding> {
    problems
        .iter()
        .map(|problem| {
            let message = match problem {
                VariableProblem::Undefined(variable) => format!("`{{ variable = \"{}\" }}` names no variable of this file", variable.0),
                VariableProblem::Unused(variable) => format!("the variable `{}` is never referred to: remove it", variable.0),
                VariableProblem::NotSelectors(variable) => format!("the variable `{}` is spliced into a selector list but holds something that is not a selector", variable.0),
            };
            finding(check_id(TASTEMAKER, FORMAT), file.clone(), Place::Whole, message, Help::Silent)
        })
        .collect()
}

/// The packages whose directories lie in the repository at `repository`, by their path in it: their files are governed
/// by `validate`, not placed.
#[must_use]
pub fn package_paths(
    packages: &BTreeMap<crate::domain::packages::types::PackageName, ResolvedPackage>,
    repository: RepositoryText<'_>,
) -> Vec<PackagePath> {
    packages
        .iter()
        .filter_map(|(name, package)| {
            let rest = package
                .directory
                .0
                .strip_prefix(repository.0)?
                .trim_start_matches('/');
            crate::domain::paths::service::normalize(crate::domain::paths::types::PathText(rest))
                .map(|path| PackagePath {
                    name: name.clone(),
                    path,
                })
        })
        .collect()
}

/// Starts a root workspace before its selected locals, packages and definitions are loaded.
#[must_use]
pub fn unloaded(
    file: &RepoPath,
    directory: &RepoPath,
    root: super::types::WorkspaceFile,
) -> Option<LoadedWorkspace> {
    let Role::Root(section) = root.role else {
        return None;
    };
    Some(LoadedWorkspace {
        file: file.clone(),
        directory: directory.clone(),
        section,
        composition: root.composition,
        locals: Vec::new(),
        packages: BTreeMap::new(),
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: Vec::new(),
        local_packages: Vec::new(),
    })
}
