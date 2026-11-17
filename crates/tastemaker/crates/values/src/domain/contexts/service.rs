//! The `contexts` service: free functions named by verb that build what a template's content sees for a file a scope
//! places, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{INSTANCE_PATH, NAME};
use super::types::{
    ContextKey, ContextRequest, ContextText, InstanceValues, Present, RenderContext, Surroundings,
    Unvalued,
};
use crate::domain::captures::service::project;
use crate::domain::captures::types::Captures;
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::reference;
use crate::domain::entries::types::{ParamSource, ParamValue};
use crate::domain::paths::service::{containment, name, relative};
use crate::domain::paths::types::Containment;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{CompositionRef, Matched, Origin, Scope, ScopeIndex};

/// What the content of `template`, placed by `entry` in scope `index` with `captures`, sees; the message of the param
/// that has no value when one has none.
///
/// # Errors
///
/// The message naming the param no value is found for.
pub fn context(request: ContextRequest<'_>) -> Result<RenderContext, Unvalued> {
    let ContextRequest {
        workspace,
        scopes,
        index,
        entry,
        template,
        file,
        captures,
        written,
        ..
    } = request;
    let Some(scope) = scopes.get(index.0) else {
        return Err(Unvalued(format!(
            "`{}` is placed in scope {}, which does not exist",
            entry.template.0, index.0
        )));
    };
    let mut context = RenderContext::default();
    for (capture, value) in scope.captures.0.iter().chain(captures.0.iter()) {
        drop(
            context
                .values
                .insert(ContextKey::from(&*capture.0), ContextText::from(&*value.0)),
        );
    }
    for (capture, value) in &scope.values.0 {
        drop(
            context
                .values
                .insert(ContextKey::from(&*capture.0), ContextText::from(&*value.0)),
        );
    }
    for (param, declaration) in &template.params.0 {
        match entry.header.params.0.get(param) {
            Some(ParamValue::Literal(value)) => {
                drop(context.values.insert(
                    ContextKey::from(&*param.0),
                    ContextText::from(&*written.write(value).0),
                ));
            }
            Some(ParamValue::Param(reference)) => {
                let found = scope
                    .values
                    .0
                    .iter()
                    .find(|(name, _)| name.0 == reference.0);
                match found {
                    Some((_, value)) => {
                        drop(
                            context
                                .values
                                .insert(ContextKey::from(&*param.0), ContextText::from(&*value.0)),
                        );
                    }
                    None => {
                        return Err(Unvalued(format!(
                            "`{}` passes `{}`, which `{}` does not give",
                            entry.template.0, reference.0, scope.identity.0
                        )));
                    }
                }
            }
            Some(ParamValue::Variable(_)) | None => {
                if context.values.contains_key(&ContextKey::from(&*param.0)) {
                    continue;
                }
                let defaulted = match declaration.source {
                    ParamSource::DirectoryName => Some(name(&scope.dir).0),
                    ParamSource::WorkspaceName => Some(workspace.section.name.0.clone()),
                    ParamSource::Given => None,
                };
                if let Some(value) = defaulted {
                    drop(
                        context
                            .values
                            .insert(ContextKey::from(&*param.0), ContextText::from(&*value)),
                    );
                } else {
                    return Err(Unvalued(format!(
                        "`{}` has no value for its param `{}` at `{}`",
                        entry.template.0, param.0, file.0
                    )));
                }
            }
        }
    }
    surroundings(
        Surroundings {
            workspace: request.workspace,
            scopes: request.scopes,
            matched: request.matched,
            index: request.index,
            captures: request.captures,
        },
        &mut context,
    );
    Ok(context)
}

/// Adds the placed siblings and nested instances to an already resolved parameter context.
pub fn surroundings(request: Surroundings<'_>, context: &mut RenderContext) {
    siblings(
        request.workspace,
        request.scopes,
        request.matched,
        request.index,
        request.captures,
        context,
    );
    context.instances = instances(request.scopes, request.index);
}

/// Fills `captures.<name>` and `present.<entry>` from what the template entries at the file's directory matched.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
fn siblings(
    workspace: &LoadedWorkspace,
    scopes: &[Scope],
    matched: &[Matched],
    index: ScopeIndex,
    captures: &Captures,
    context: &mut RenderContext,
) {
    let scope = &scopes[index.0];
    let dir = &scope.dir;
    let own: BTreeSet<_> = captures.0.keys().cloned().collect();
    let consistent = |tuple: &Captures| {
        tuple
            .0
            .iter()
            .all(|(name, value)| captures.0.get(name).is_none_or(|mine| mine == value))
    };
    // The part instances nested beneath the scope that the file is beside (each capture of the part is one of the
    // file's) come after the scope's own entries, so a module file reads what its own part places: an adapter's module
    // file declares `errors` when the adapter has one, not when its port has.
    let belongs = |other: &Scope| {
        other.captures.0.iter().all(|(name, value)| {
            captures.0.get(name).or_else(|| scope.captures.0.get(name)) == Some(value)
        })
    };
    let nested = |other: &Scope| {
        matches!(other.origin, Origin::Instance { .. })
            && other.dir != *dir
            && (containment(&other.dir, dir) == Containment::Inside)
            && !other.captures.0.is_empty()
            && belongs(other)
    };
    let mut values: BTreeMap<ContextKey, BTreeSet<ContextText>> = BTreeMap::new();
    let at = matched.iter().filter(|found| {
        scopes
            .get(found.scope.0)
            .is_some_and(|placed| placed.dir == *dir)
    });
    let beneath = matched
        .iter()
        .filter(|found| nested(&scopes[found.scope.0]));
    for found in at.chain(beneath) {
        let placed = &scopes[found.scope.0];
        let composition = composition_of(workspace, &placed.composition);
        let other = &composition.templates[found.entry.0];
        for tuple in found.tuples.iter().filter(|tuple| consistent(tuple)) {
            for (name, value) in &tuple.0 {
                if !captures.0.contains_key(name) {
                    let _: bool = values
                        .entry(ContextKey::from(&*name.0))
                        .or_default()
                        .insert(ContextText::from(&*value.0));
                }
            }
        }
        if found.names.is_subset(&own) {
            let present = if found.tuples.contains(&project(captures, &found.names)) {
                Present::Yes
            } else {
                Present::No
            };
            let _: Option<Present> = context.present.insert(
                ContextKey::from(&*reference(&other.header, &other.template).0),
                present,
            );
        }
    }
    for (name, found) in values {
        drop(context.captured.insert(name, found.into_iter().collect()));
    }
}

/// Each instance of a spec beneath scope `index` that no instance of the same spec beneath it holds, by spec, with its
/// name, its path from the scope, and its params and captures.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
pub(super) fn instances(
    scopes: &[Scope],
    index: ScopeIndex,
) -> BTreeMap<ContextKey, Vec<InstanceValues>> {
    let own = &scopes[index.0];
    let dir = &own.dir;
    let base = match own.origin {
        Origin::File => {
            crate::domain::paths::service::parent(&own.anchor).unwrap_or_else(|| dir.clone())
        }
        Origin::Instance { .. } => dir.clone(),
    };
    let beneath = scopes
        .iter()
        .enumerate()
        .filter(|(_, scope)| {
            matches!(scope.origin, Origin::Instance { .. })
                && scope.dir != *dir
                && (containment(&scope.dir, dir) == Containment::Inside)
        })
        .collect::<Vec<_>>();
    // Whether the instance at `child` was placed, through any number of nested specs, by the one at `placer`.
    let placed_by = |child: ScopeIndex, placer: ScopeIndex| {
        let mut at = child;
        for _ in 0..scopes.len() {
            let Some(Origin::Instance { parent, .. }) = scopes.get(at.0).map(|scope| scope.origin)
            else {
                return false;
            };
            if parent == placer {
                return true;
            }
            at = parent;
        }
        false
    };
    // An instance is seen unless an instance beneath the scope that holds it placed it, or is of its own spec: an
    // instance of another spec over the same path that placed nothing there (a language's spec over a product's
    // crates) hides nothing.
    let mut direct = beneath
        .iter()
        .filter(|(at, scope)| {
            !beneath.iter().any(|(other_at, other)| {
                other.dir != scope.dir
                    && (containment(&scope.dir, &other.dir) == Containment::Inside)
                    && (placed_by(ScopeIndex(*at), ScopeIndex(*other_at))
                        || match (&other.composition, &scope.composition) {
                            (CompositionRef::Spec(outer), CompositionRef::Spec(inner)) => {
                                outer.package == inner.package && outer.name == inner.name
                            }
                            (CompositionRef::Spec(_), _) | (_, CompositionRef::Spec(_)) => false,
                            (
                                CompositionRef::Root | CompositionRef::Local(_),
                                CompositionRef::Root | CompositionRef::Local(_),
                            ) => true,
                        })
            })
        })
        .map(|(_, scope)| *scope)
        .collect::<Vec<_>>();
    direct.sort_by(|left, right| left.dir.cmp(&right.dir));
    let mut found: BTreeMap<ContextKey, Vec<InstanceValues>> = BTreeMap::new();
    for scope in direct {
        let CompositionRef::Spec(release) = &scope.composition else {
            continue;
        };
        let mut values = BTreeMap::new();
        drop(values.insert(
            ContextKey::from(NAME),
            ContextText::from(&*name(&scope.dir).0),
        ));
        let path = relative(&scope.dir, &base)
            .map(|relative| relative.0)
            .or_else(|| relative(&scope.dir, dir).map(|relative| relative.0))
            .unwrap_or_default();
        drop(values.insert(ContextKey::from(INSTANCE_PATH), ContextText::from(path)));
        for (capture, value) in scope.captures.0.iter().chain(scope.values.0.iter()) {
            drop(values.insert(ContextKey::from(&*capture.0), ContextText::from(&*value.0)));
        }
        found
            .entry(ContextKey::from(&*release.name.0))
            .or_default()
            .push(InstanceValues(values));
    }
    found
}
