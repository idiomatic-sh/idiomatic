//! The test support of the `applications` concept: the `check` fixture's workspace loaded over a tree, placed, and the
//! matching port's compiles, which the tests of the concepts that plan the lint call build their subjects from.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::types::{Carried, Compilers, ParamSet};
use crate::domain::definitions::types::{LoadedWorkspace, Role};
use crate::domain::entries::service::rule_release;
use crate::domain::loading::service::{begin, finish, next, rule_read, spec_read, template_read};
use crate::domain::packages::types::{
    PackageName, PackageRoot, Reach, ResolvedPackage, SourcePath,
};
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::service::place;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::ScopeIndex;
use crate::domain::selectors::types::{PathSet, PathTest, PatternFailure, SetGlob, Spelling};
use crate::inbound::use_cases::check::fixtures::{PACKAGE, RULE, SPEC, TEMPLATE, WORKSPACE};
use crate::outbound::manifest::interface::ManifestPort as _;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort as _;

/// The `check` fixture's workspace over `files`, loaded.
#[must_use]
pub fn workspace(files: &[&str]) -> LoadedWorkspace {
    workspace_of(WORKSPACE, SPEC, files)
}

/// The workspace file `root` over `files`, loaded, each spec it applies read as `spec` and the `check` fixture's rule and
/// template read for every rule and template.
///
/// # Panics
///
/// When a fixture does not read, which it always does.
#[must_use]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test support: a fixture that does not read is a broken test, and the panic names which"
)]
pub fn workspace_of(root: &str, spec: &str, files: &[&str]) -> LoadedWorkspace {
    let manifest = crate::outbound::manifest::factory::toml();
    let root = manifest
        .workspace_file(&DefinitionText(root.into()))
        .expect("the workspace reads");
    let Role::Root(section) = root.role else {
        panic!("a root file")
    };
    let mut packages = BTreeMap::new();
    drop(
        packages.insert(
            PackageName("p".into()),
            ResolvedPackage {
                directory: PackageRoot("/p".into()),
                pin: crate::domain::packages::types::PackageSource::Path(SourcePath("p".into())),
                manifest: manifest
                    .package(&DefinitionText(PACKAGE.into()))
                    .expect("the package reads"),
                reach: Reach::Direct,
            },
        ),
    );
    let mut loading = begin(LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section,
        composition: root.composition,
        locals: Vec::new(),
        packages,
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: files.iter().map(|file| RepoPath((*file).into())).collect(),
        local_packages: Vec::new(),
    });
    while let Some(wanted) = next(&mut loading) {
        match &*wanted.file.0 {
            file if file.ends_with("rule.toml") => rule_read(
                &mut loading,
                &wanted,
                manifest
                    .rule(&DefinitionText(RULE.into()))
                    .expect("the rule reads"),
            ),
            file if file.ends_with("spec.toml") => {
                let problems = spec_read(
                    &mut loading,
                    &wanted,
                    manifest
                        .spec(&DefinitionText(spec.into()))
                        .expect("the spec reads"),
                );
                assert!(
                    problems.is_empty(),
                    "the fixture spec has no variable problems"
                );
            }
            _ => template_read(
                &mut loading,
                &wanted,
                manifest
                    .template(&DefinitionText(TEMPLATE.into()))
                    .expect("the template reads"),
            ),
        }
    }
    finish(loading).workspace
}

/// The matching port's compile of one pattern.
fn compile(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    crate::outbound::matching::factory::globset().compile(spelling)
}

/// The matching port's compile of a set of globs.
fn compile_set(globs: &[SetGlob]) -> Result<Arc<dyn PathSet>, PatternFailure> {
    crate::outbound::matching::factory::globset().compile_set(globs)
}

/// The matching port's compiles, as the domain is handed them.
#[must_use]
pub fn matching() -> Compilers<'static> {
    Compilers {
        compile: &compile,
        sets: &compile_set,
    }
}

/// `workspace` placed.
///
/// # Panics
///
/// When the fixture does not place, which it always does.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "test support: a fixture that does not place is a broken test, and the panic says so"
)]
pub fn placement(workspace: &LoadedWorkspace) -> Placement {
    place(
        workspace,
        matching().compile,
        &crate::outbound::manifest::factory::toml(),
    )
    .expect("the fixture places")
}

/// The marker rule's one application, at the workspace's root, with no params.
///
/// # Panics
///
/// When the fixture's workspace applies no rule at its root, which it always does.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "test support: the fixture's workspace applies its one rule at its root, and the panic says when it does not"
)]
pub fn applications<'w>(workspace: &'w LoadedWorkspace, placement: &Placement) -> Vec<Carried<'w>> {
    let scope = placement
        .scopes
        .first()
        .expect("the placement has its root scope");
    let entry = composition_of(workspace, &scope.composition)
        .rules
        .first()
        .expect("the root applies the marker rule");
    vec![Carried {
        scope: ScopeIndex(0),
        entry,
        release: rule_release(entry),
        params: ParamSet::default(),
    }]
}
