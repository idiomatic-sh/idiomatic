//! The `loading` service: free functions named by verb that load a workspace's definitions, with no IO and no ports.
//! The caller reads each file the load asks for and hands back what it parsed; the load says what to read next.

use super::types::{DefinitionFile, LoadProblem, Loading, ReadFile, Wanted, WorkspaceLoad};
use crate::domain::definitions::types::{
    Definition, LoadedWorkspace, RuleFile, SpecFile, TemplateFile,
};
use crate::domain::entries::service::{adopt, releases};
use crate::domain::packages::service::release_file;
use crate::domain::packages::types::{PackageRoot, Release};
use crate::domain::variables::service::{apply, resolve};
use crate::domain::variables::types::VariableProblem;

/// A load of `workspace`, its root file and locals read and its packages resolved: every release their entries name is
/// asked for.
#[must_use]
pub fn begin(workspace: LoadedWorkspace) -> Loading {
    let mut loading = Loading {
        workspace,
        pending: Vec::new(),
        asked: std::collections::BTreeSet::new(),
        read: Vec::new(),
        problems: Vec::new(),
    };
    let named = releases(&loading.workspace.composition)
        .into_iter()
        .chain(
            loading
                .workspace
                .locals
                .iter()
                .flat_map(|local| releases(&local.composition)),
        )
        .collect::<Vec<_>>();
    ask(&mut loading, named);
    loading
}

/// Marks each release as asked and queues the ones not asked before.
fn ask(loading: &mut Loading, named: Vec<Release>) {
    for release in named {
        if loading.asked.insert(release.clone()) {
            loading.pending.push(release);
        }
    }
}

/// The next definition to read, when there is one; a release whose package no dependency resolves is a problem.
pub fn next(loading: &mut Loading) -> Option<Wanted> {
    while let Some(release) = loading.pending.pop() {
        match loading.workspace.packages.get(&release.package) {
            Some(package) => {
                return Some(Wanted {
                    file: DefinitionFile(release_file(&release).0.into()),
                    directory: package.directory.clone(),
                    release,
                });
            }
            None => loading.problems.push(LoadProblem(
                format!(
                    "`{}/{}` comes from package `{}`, which no dependency resolves",
                    release.package.0, release.name.0, release.package.0
                )
                .into(),
            )),
        }
    }
    None
}

/// Every definition the load asks for now, in the order it asks: what one wave of reads can fetch at once.
pub fn wanted(loading: &mut Loading) -> Vec<Wanted> {
    std::iter::from_fn(|| next(loading)).collect()
}

/// Records a file read, one the check's answer rests on.
pub fn note(loading: &mut Loading, directory: &PackageRoot, file: &DefinitionFile) {
    loading.read.push(ReadFile {
        directory: directory.clone(),
        file: file.clone(),
    });
}

/// A rule read.
pub fn rule_read(loading: &mut Loading, wanted: &Wanted, rule: RuleFile) {
    note(loading, &wanted.directory, &wanted.file);
    drop(loading.workspace.rules.insert(wanted.release.clone(), rule));
}

/// A definition read, of whichever kind it is: a spec's variable problems are its answer, and a rule or a template has
/// none.
pub fn definition_read(
    loading: &mut Loading,
    wanted: &Wanted,
    definition: Definition,
) -> Vec<VariableProblem> {
    match definition {
        Definition::Rule(rule) => {
            rule_read(loading, wanted, rule);
            Vec::new()
        }
        Definition::Spec(spec) => spec_read(loading, wanted, spec),
        Definition::Template(template) => {
            template_read(loading, wanted, template);
            Vec::new()
        }
    }
}

/// A definition read, of whichever kind it is, for a caller that answers nothing about its variables.
pub fn definition_loaded(loading: &mut Loading, wanted: &Wanted, definition: Definition) {
    match definition {
        Definition::Rule(rule) => rule_read(loading, wanted, rule),
        Definition::Spec(spec) => spec_loaded(loading, wanted, spec),
        Definition::Template(template) => template_read(loading, wanted, template),
    }
}

/// A spec read, as `spec_read` does, for a caller that answers nothing about its variables.
fn spec_loaded(loading: &mut Loading, wanted: &Wanted, mut spec: SpecFile) {
    note(loading, &wanted.directory, &wanted.file);
    adopt(&mut spec.composition, &wanted.release);
    apply(&mut spec.composition, &mut [&mut spec.rules_exclude]);
    ask(loading, releases(&spec.composition));
    drop(loading.workspace.specs.insert(wanted.release.clone(), spec));
}

/// A spec read: its owned entries adopted, its variables put in place, and the releases it names asked for.
pub fn spec_read(
    loading: &mut Loading,
    wanted: &Wanted,
    mut spec: SpecFile,
) -> Vec<VariableProblem> {
    note(loading, &wanted.directory, &wanted.file);
    adopt(&mut spec.composition, &wanted.release);
    let problems = resolve(&mut spec.composition, &mut [&mut spec.rules_exclude]);
    ask(loading, releases(&spec.composition));
    drop(loading.workspace.specs.insert(wanted.release.clone(), spec));
    problems
}

/// A template read.
pub fn template_read(loading: &mut Loading, wanted: &Wanted, template: TemplateFile) {
    note(loading, &wanted.directory, &wanted.file);
    drop(
        loading
            .workspace
            .templates
            .insert(wanted.release.clone(), template),
    );
}

/// The workspace loaded.
#[must_use]
pub fn finish(loading: Loading) -> WorkspaceLoad {
    WorkspaceLoad {
        workspace: loading.workspace,
        read: loading.read,
        problems: loading.problems,
    }
}
