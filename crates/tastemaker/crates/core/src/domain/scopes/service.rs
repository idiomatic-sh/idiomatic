//! The `scopes` service: free functions named by verb over the scopes a placement made, with no IO and no ports: the
//! counts, pairs and outdated entries it checks, and the counts that wait for the conditions, judged once they are
//! answered.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    Closing, CompositionRef, EntryLabel, FileHolding, FilePlacing, Identity, Labels, Origin, Scope,
    ScopeCheck, SpecLabel,
};
use crate::domain::captures::types::Captures;
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, rule_release, spec_release, template_release};
use crate::domain::entries::types::{ClosedChoice, Composition};
use crate::domain::findings::constants::OUTDATED;
use crate::domain::findings::service::{check_id, finding};
use crate::domain::findings::types::Place;
use crate::domain::packages::service::version_number;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::service::{beneath, containment, parent};
use crate::domain::paths::types::Containment;
use crate::domain::paths::types::RepoPath;

/// A `tastemaker.toml`'s own scope at `dir`, named `identity`, anchored at the file.
#[must_use]
pub fn scope_file(
    dir: RepoPath,
    identity: Identity,
    anchor: RepoPath,
    composition: CompositionRef,
    written: &Composition,
) -> Scope {
    Scope {
        dir,
        identity,
        anchor,
        closed: written.closed,
        values: crate::domain::captures::types::Interpolations::default(),
        typed: BTreeMap::new(),
        captures: Captures::default(),
        composition,
        rules_exclude: crate::domain::selectors::types::Selectors::default(),
        nested: Vec::new(),
        origin: Origin::File,
    }
}

/// The composition a scope applies.
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
pub fn composition_of<'w>(
    workspace: &'w LoadedWorkspace,
    reference: &CompositionRef,
) -> &'w Composition {
    match reference {
        CompositionRef::Root => &workspace.composition,
        CompositionRef::Local(index) => &workspace.locals[index.0].composition,
        CompositionRef::Spec(release) => &workspace.specs[release].composition,
    }
}

/// Whether a file of the sorted `files` lies directly in `directory`.
#[must_use]
pub fn holds_file(files: &[RepoPath], directory: &RepoPath) -> FileHolding {
    if beneath(files, directory)
        .iter()
        .any(|file| parent(file).as_ref() == Some(directory))
    {
        FileHolding::HoldsFiles
    } else {
        FileHolding::HoldsNone
    }
}

/// Whether `composition` places files through templates, its own or those of the specs it places: only there does
/// `closed` have effect.
#[must_use]
pub(crate) fn places_files(workspace: &LoadedWorkspace, composition: &Composition) -> FilePlacing {
    let places = !composition.templates.is_empty()
        || composition.specs.iter().any(|entry| {
            workspace
                .specs
                .get(&spec_release(entry))
                .is_some_and(|spec| {
                    places_files(workspace, &spec.composition) == FilePlacing::PlacesFiles
                })
        });
    if places {
        FilePlacing::PlacesFiles
    } else {
        FilePlacing::PlacesNone
    }
}

/// Whether `composition` closes its directory: it is closed and places files.
#[must_use]
pub fn closes(workspace: &LoadedWorkspace, composition: &Composition) -> Closing {
    if composition.closed == ClosedChoice::Closed
        && places_files(workspace, composition) == FilePlacing::PlacesFiles
    {
        Closing::Closes
    } else {
        Closing::Open
    }
}

/// The spec's name and the spec entry's reference of an instance: empty for a file's own scope.
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "an instance's parent is a scope place_at made before it: an index past the scopes is a bug that must crash, never an unlabelled finding"
)]
pub fn labels(workspace: &LoadedWorkspace, scopes: &[Scope], scope: &Scope) -> Labels {
    match scope.origin {
        Origin::File => Labels::default(),
        Origin::Instance { parent, entry } => {
            let owner = &scopes[parent.0];
            let composition = composition_of(workspace, &owner.composition);
            let entry = &composition.specs[entry.0];
            Labels {
                spec: SpecLabel(entry.spec.0.to_string()),
                entry: EntryLabel(
                    crate::domain::entries::service::reference(&entry.header, &entry.spec)
                        .0
                        .to_string(),
                ),
            }
        }
    }
}

/// The entries of a `tastemaker.toml` whose source offers a newer version: each an error, as every finding is.
#[must_use]
pub fn outdated(workspace: &LoadedWorkspace, scopes: &[Scope]) -> ScopeCheck {
    let mut findings = Vec::new();
    let mut evaluated = BTreeSet::new();
    for scope in scopes
        .iter()
        .filter(|scope| matches!(scope.origin, Origin::File))
    {
        let composition = composition_of(workspace, &scope.composition);
        let entries = composition
            .rules
            .iter()
            .map(|entry| {
                (
                    identity(&entry.header, DefinitionKind::Rule, &entry.rule),
                    rule_release(entry),
                )
            })
            .chain(composition.specs.iter().map(|entry| {
                (
                    identity(&entry.header, DefinitionKind::Spec, &entry.spec),
                    spec_release(entry),
                )
            }))
            .chain(composition.templates.iter().map(|entry| {
                (
                    identity(&entry.header, DefinitionKind::Template, &entry.template),
                    template_release(entry),
                )
            }));
        for (identity_text, release) in entries {
            let _: bool = evaluated.insert(check_id(&identity_text, OUTDATED));
            // Only a run that judges `#outdated` (check) looks each shared release's latest version up, and it looks every
            // one up, its package always loaded; a definition the file owns has none, nor has any in another run.
            let Some(latest) = workspace.latest.get(&release) else {
                continue;
            };
            if version_number(latest) > version_number(&release.version) {
                findings.push(finding(
                    check_id(&identity_text, OUTDATED),
                    scope.anchor.clone(),
                    Place::Whole,
                    format!(
                        "`{identity_text}` follows {}; its source offers {}: `tm update -n` lists what moves, `tm update` moves it",
                        release.version.0, latest.0
                    ),
                    crate::domain::findings::types::Help::Silent,
                ));
            }
        }
    }
    ScopeCheck {
        findings,
        evaluated,
    }
}

/// Whether a scope owns its directory: it places templates or specs. A spec of rules alone places nothing and owns
/// nothing.
pub(crate) fn owns(workspace: &LoadedWorkspace, scope: &Scope) -> FilePlacing {
    let composition = composition_of(workspace, &scope.composition);
    if composition.templates.is_empty() && composition.specs.is_empty() {
        FilePlacing::PlacesNone
    } else {
        FilePlacing::PlacesFiles
    }
}

/// Each instance scope's nested instances, among `instances`: a spec instance nested inside another owns its
/// directory, and the outer one places no file there; the outer one's rules still read it, as their paths say. A spec of rules alone places nothing and owns nothing.
pub fn nest(workspace: &LoadedWorkspace, instances: &mut [Scope]) {
    let owns = |scope: &Scope| owns(workspace, scope) == FilePlacing::PlacesFiles;
    let owning = instances
        .iter()
        .filter(|scope| owns(scope))
        .map(|scope| scope.dir.clone())
        .collect::<Vec<_>>();
    for scope in instances.iter_mut() {
        let nested = if owns(scope) {
            owning
                .iter()
                .filter(|instance| {
                    **instance != scope.dir
                        && containment(instance, &scope.dir) == Containment::Inside
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        scope.nested = nested;
    }
}
