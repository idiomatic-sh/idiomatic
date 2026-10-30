//! The `scenario_proving` service: free functions named by verb over what a definition's scenarios are proven
//! with: the workspace a spec scenario is placed in, the globs and params a rule scenario runs with, the findings a scenario's
//! check reports, and the problems of a definition that does not explain itself.

use crate::domain::definitions::constants::WORKSPACE_FILE;
use crate::domain::definitions::service::scenario_files;
use crate::domain::definitions::types::{Definition, RuleFile, SpecFile, TemplateFile};
use crate::domain::entries::service::{rule_release, spec_release, template_release};
use crate::domain::entries::types::{Composition, Described};
use crate::domain::packages::service::{kind_directory, kind_file, kind_word};
use crate::domain::packages::types::PlacedDefinition;
use crate::domain::packages::types::{DefinitionKind, Ownership, Release};
use crate::domain::paths::types::RepoPath;
use crate::domain::proofs::types::{
    DefinitionLabel, JudgedScenario, Problem, Subject, SubjectDirectory, SubjectId,
};
use crate::domain::scenario_files::constants::{EMPTY, FAILING, NOT_EMPTY, PASSING};
use crate::domain::scenario_files::types::{WorkspaceText, WrittenWorkspace};
use crate::domain::scenarios::types::Expectation;

/// A problem for each way the scenarios of the definition `label` fail to cover it: two with one id, none that passes,
/// none that fails. Each says the scenario and the fix.
pub fn covered(
    problems: &mut Vec<Problem>,
    label: &DefinitionLabel,
    scenarios: &[JudgedScenario<'_>],
) {
    let mut seen = std::collections::BTreeSet::new();
    for scenario in scenarios {
        if !seen.insert(&scenario.id.0) {
            problems.push(Problem(
                format!(
                    "{} has two scenarios with the id `{}`: give each [[scenarios]] entry an `id` of its own",
                    label.0, scenario.id.0
                )
                .into(),
            ));
        }
    }
    for (verdict, word, flag) in [
        (Expectation::Passes, PASSING, EMPTY),
        (Expectation::Fails, FAILING, NOT_EMPTY),
    ] {
        if !scenarios.iter().any(|each| each.verdict == verdict) {
            problems.push(Problem(
                format!(
                    "{} has no {word} scenario: add a [[scenarios]] entry whose `then.findings` is {flag}",
                    label.0
                )
                .into(),
            ));
        }
    }
}

/// The scenarios of a rule, each with what it expects.
#[must_use]
pub fn judged_rule(rule: &RuleFile) -> Vec<JudgedScenario<'_>> {
    rule.scenarios
        .iter()
        .map(|scenario| JudgedScenario {
            id: &scenario.id,
            verdict: scenario.verdict,
        })
        .collect()
}

/// The scenarios of a spec, each with what it expects.
#[must_use]
pub fn judged_spec(spec: &SpecFile) -> Vec<JudgedScenario<'_>> {
    spec.scenarios
        .iter()
        .map(|scenario| JudgedScenario {
            id: &scenario.id,
            verdict: scenario.verdict,
        })
        .collect()
}

/// The scenarios of a template, each with what it expects.
#[must_use]
pub fn judged_template(template: &TemplateFile) -> Vec<JudgedScenario<'_>> {
    template
        .scenarios
        .iter()
        .map(|scenario| JudgedScenario {
            id: &scenario.id,
            verdict: scenario.verdict,
        })
        .collect()
}

/// The directory a `tastemaker.toml` written by a scenario of the definition file `file`, in the package at `directory`,
/// is rebased onto: where the file would be beside the definition, at its own path in the scenario's project.
#[must_use]
pub fn written_home(
    directory: &crate::domain::packages::types::PackageRoot,
    file: &crate::domain::proofs::types::ProofPath,
    written: &RepoPath,
) -> RepoPath {
    let parent = |path: &std::path::Path| {
        path.parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default()
    };
    RepoPath(
        std::path::Path::new(&*directory.0)
            .join(parent(std::path::Path::new(&*file.0)))
            .join(parent(std::path::Path::new(&*written.0)))
            .display()
            .to_string()
            .into(),
    )
}

/// The texts of the `tastemaker.toml` files the scenarios of `definition` write: the workspaces they are placed by, which
/// name releases other than the definition's own.
#[must_use]
pub fn written_workspaces(definition: &Definition) -> Vec<WrittenWorkspace<'_>> {
    scenario_files(definition)
        .into_iter()
        .flat_map(|files| files.0.iter())
        .filter(|(path, _)| path.0.rsplit('/').next() == Some(WORKSPACE_FILE))
        .map(|(path, text)| WrittenWorkspace {
            path,
            text: WorkspaceText(&text.0),
        })
        .collect()
}

/// A problem for a blank description or why.
pub fn explained(
    problems: &mut Vec<Problem>,
    description: &crate::domain::entries::types::Description,
    why: &crate::domain::definitions::types::Why,
) {
    if description.0.trim().is_empty() {
        problems.push(Problem(
            crate::domain::proofs::constants::BLANK_DESCRIPTION.into(),
        ));
    }
    if why.0.trim().is_empty() {
        problems.push(Problem(crate::domain::proofs::constants::BLANK_WHY.into()));
    }
}

/// A problem for each import of a spec that says nothing of what it is here.
pub fn described(problems: &mut Vec<Problem>, spec: &SpecFile) {
    let headers = spec
        .composition
        .rules
        .iter()
        .map(|entry| (&entry.header, &entry.rule))
        .chain(
            spec.composition
                .templates
                .iter()
                .map(|entry| (&entry.header, &entry.template)),
        )
        .chain(
            spec.composition
                .specs
                .iter()
                .map(|entry| (&entry.header, &entry.spec)),
        );
    for (header, name) in headers {
        if header.description == Described::Unsaid {
            problems.push(Problem(
                format!("its import `{}` has no `description`", name.0).into(),
            ));
        }
    }
}

/// Where a wrapper package holds the definition file of the spec version in `directory`.
#[must_use]
pub fn wrapper_file(directory: &SubjectDirectory, kind: DefinitionKind) -> RepoPath {
    RepoPath(format!("{}/{}", directory.0, kind_file(kind).0).into())
}

/// Where `release` lives in its package: `<kind directory>/<name>/<version>`.
#[must_use]
pub fn release_directory(release: &Release) -> SubjectDirectory {
    SubjectDirectory(
        format!(
            "{}/{}/{}",
            kind_directory(release.kind).0,
            release.name.0,
            release.version.0
        )
        .into(),
    )
}

/// What a version proves: `release`, by its id `<package>/<kind>/<name>@<version>` and its directory.
#[must_use]
pub fn subject_of(release: &Release) -> Subject {
    Subject {
        id: SubjectId(
            format!(
                "{}/{}/{}@{}",
                release.package.0,
                kind_word(release.kind).0,
                release.name.0,
                release.version.0
            )
            .into(),
        ),
        directory: release_directory(release),
    }
}

/// The releases `composition`'s entries name from another package: those it shares, not those a spec owns.
#[must_use]
pub fn shared_releases(composition: &Composition) -> Vec<Release> {
    let shared = |release: &Release| matches!(release.ownership, Ownership::Shared);
    composition
        .rules
        .iter()
        .map(rule_release)
        .filter(shared)
        .chain(composition.specs.iter().map(spec_release).filter(shared))
        .chain(
            composition
                .templates
                .iter()
                .map(template_release)
                .filter(shared),
        )
        .collect()
}

/// The definition a scenario's entry for `release` names: its kind, package and name; the scenario's check pins the
/// version it proves.
#[must_use]
pub fn entry_head(release: &Release) -> PlacedDefinition {
    PlacedDefinition {
        kind: release.kind,
        package: release.package.clone(),
        name: release.name.clone(),
    }
}
