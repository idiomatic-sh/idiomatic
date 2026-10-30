// ignore: idiomatic.arch/file-length - placement is one fixed point over the scopes: specs applied, instances nested, templates placed, counts and closed directories judged from the same working state, which the concept holds as one service.
//! The `placement` service: free functions named by verb that place a workspace's entries, with no IO and no ports. Each
//! spec entry's directories become scopes, a nested instance owns its directory, and each template entry places its
//! files; counts that read a condition wait for it.

use std::collections::BTreeSet;

use super::errors::PlacementError;
use super::types::{Applying, Duplication, Placement, Selected, SelectedDirectory, Working};
use crate::domain::captures::service::inheriting;
use crate::domain::captures::types::Interpolations;
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, minimum, spec_release};
use crate::domain::entries::types::{Amount, EntryIdentity, SpecEntry};
use crate::domain::findings::constants::{CLOSED, COUNT, COVERAGE, PARAMS};
use crate::domain::findings::service::{check_id, error};
use crate::domain::instance_params::service::params;
use crate::domain::instance_params::types::Instance;
use crate::domain::packages::service::release_id;
use crate::domain::packages::types::{DefinitionKind, Release};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::service::{closes, composition_of, nest, scope_file};
use crate::domain::scopes::types::{
    AppliedWorking, CapturingEntry, Closing, CompositionRef, CountedEntry, EntryIndex, Grouping,
    Identity, IncludeSpelling, Origin, PatternWorking, Scope, ScopeIndex,
};
use crate::domain::selectors::service::{covers_any, first_match, leading, names_of, resolve};
use crate::domain::selectors::types::{
    Coverage, PatternCompiler, Resolved, ResolvedSelectors, SelectionContext, Selector,
};
use crate::domain::template_placing::service::{exclusion, known, templates};

/// Places every entry and reports structural findings; conditional counts wait for the lint answer.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile, `Unloaded` when an entry names a release that was not loaded.
pub fn place(
    workspace: &LoadedWorkspace,
    compile: &dyn PatternCompiler,
    written: &dyn crate::domain::entries::types::ValueWriter,
) -> Result<Placement, PlacementError> {
    place_at(workspace, compile, &BTreeSet::new(), written)
}

/// Places existing files and explicitly requested new instance roots through the same spec selectors.
///
/// # Errors
///
/// The same unloaded-definition and invalid-selector failures as `place`.
pub fn place_at(
    workspace: &LoadedWorkspace,
    compile: &dyn PatternCompiler,
    requested: &BTreeSet<RepoPath>,
    written: &dyn crate::domain::entries::types::ValueWriter,
) -> Result<Placement, PlacementError> {
    let mut working = Working::default();
    working.placement.scopes.push(scope_file(
        workspace.directory.clone(),
        Identity(super::constants::ROOT_FILE.into()),
        workspace.file.clone(),
        CompositionRef::Root,
        &workspace.composition,
    ));
    for (index, local) in workspace.locals.iter().enumerate() {
        working.placement.scopes.push(scope_file(
            local.directory.clone(),
            Identity(local.file.0.clone()),
            local.file.clone(),
            CompositionRef::Local(EntryIndex(index)),
            &local.composition,
        ));
    }
    let files = working.placement.scopes.len();
    // Every directory is a candidate, one holding only directories too: an instance there applies to its subtree.
    let mut candidates = crate::domain::paths::service::directories(&workspace.files);
    candidates.extend(requested.iter().cloned());
    candidates.sort();
    candidates.dedup();
    let mut parent = 0;
    while let Some(scope) = working.placement.scopes.get(parent) {
        let reference = scope.composition.clone();
        let composition = composition_of(workspace, &reference);
        for (position, entry) in composition.specs.iter().enumerate() {
            apply(
                Applying {
                    workspace,
                    compile,
                    written,
                },
                &mut working,
                ScopeIndex(parent),
                EntryIndex(position),
                entry,
                &candidates,
            )?;
        }
        parent += 1;
    }
    if let Some(nested) = working.placement.scopes.get_mut(files..) {
        nest(workspace, nested);
    }
    let placed = templates(
        workspace,
        compile,
        &working.placement.scopes,
        &mut working.placement.placed,
    )?;
    working.placement.findings.extend(placed.findings);
    working.placement.evaluated.extend(placed.evaluated);
    working.matched.extend(placed.matched);
    working.placement.excluded = exclusion(workspace, compile, &working.placement.scopes)?;
    let counted = crate::domain::counting::service::counts(
        &working.placement.scopes,
        &working.matched,
        &working.applied,
        &working.applications,
        workspace,
    );
    working.placement.counts = counted.checks;
    working.placement.pairs = counted.pairs;
    working.placement.findings.extend(counted.findings);
    let closings = crate::domain::closing::service::facts(
        workspace,
        &working.placement.scopes,
        &working.placement.placed,
    );
    working
        .placement
        .evaluated
        .extend(closings.evaluated.iter().cloned());
    working.placement.closings = closings;
    let outdated = crate::domain::scopes::service::outdated(workspace, &working.placement.scopes);
    working.placement.findings.extend(outdated.findings);
    working.placement.evaluated.extend(outdated.evaluated);
    working.placement.matched = working
        .matched
        .iter()
        .map(|found| found.matched.clone())
        .collect();
    working.placement.applied = working
        .applied
        .iter()
        .map(|applied| applied.applied.clone())
        .collect();
    Ok(working.placement)
}

/// Reports repeated application of the same spec at one directory as `#coverage`.
fn twice(
    working: &mut Working,
    directory: &RepoPath,
    release: &Release,
    identity_text: &EntryIdentity,
) -> Duplication {
    let duplicate = working.placement.scopes.iter().any(|scope| {
        scope.dir == *directory
            && matches!(scope.origin, Origin::Instance { .. })
            && matches!(&scope.composition, CompositionRef::Spec(other) if release_id(other) == release_id(release))
    });
    if !duplicate {
        return Duplication::Once;
    }
    working.placement.findings.push(error(
        check_id(identity_text, COVERAGE),
        directory.clone(),
        format!(
            "`{identity_text}` is applied twice at `{}`: a spec applies once at one directory",
            directory.0
        ),
    ));
    Duplication::Twice
}
/// Whether an entry's paths capture names, so its count holds for each value of the captures before the last.
fn grouping(include: &ResolvedSelectors) -> Grouping {
    let grouped = include.0.iter().any(|item| match item {
        Resolved::Pattern(matcher) => !leading(matcher).is_empty(),
        Resolved::Paths(_) => false,
    });
    if grouped {
        Grouping::Grouped
    } else {
        Grouping::Whole
    }
}

/// The entry's patterns with the leading captures and the tuples each pattern selected.
fn patterns_of(include: &ResolvedSelectors, found: &[SelectedDirectory]) -> Vec<PatternWorking> {
    include
        .0
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item {
            Resolved::Pattern(matcher) => Some(PatternWorking {
                matcher: matcher.clone(),
                leading: leading(matcher),
                tuples: found
                    .iter()
                    .filter(|selected| selected.pattern.0 == index)
                    .map(|selected| selected.captures.clone())
                    .collect(),
            }),
            Resolved::Paths(_) => None,
        })
        .collect()
}

/// The first glob an entry's `include` writes, as spelled.
fn first_glob(entry: &SpecEntry) -> IncludeSpelling {
    IncludeSpelling(
        entry
            .header
            .include
            .0
            .first()
            .map(|selector| match selector {
                Selector::Glob(pattern) => pattern.0.clone(),
                Selector::Regex(_)
                | Selector::Part(_)
                | Selector::PlacedBy(_)
                | Selector::Variable(_) => Box::default(),
            })
            .unwrap_or_default(),
    )
}

/// Resolves the entry selectors and captures each selected candidate directory.
fn selected(
    workspace: &LoadedWorkspace,
    compile: &dyn PatternCompiler,
    working: &Working,
    base: &RepoPath,
    entry: &SpecEntry,
    candidates: &[RepoPath],
    identity_text: &EntryIdentity,
) -> Result<Selected, PlacementError> {
    let known = known(
        &working.placement.scopes,
        &working.placement.placed,
        workspace,
        &[&entry.header.include, &entry.header.exclude],
    );
    let none = Interpolations::default();
    let context = SelectionContext {
        base,
        values: &none,
        instances: &known.instances,
        placed: &known.placed,
        compile,
    };
    let selector_error = |source| PlacementError::Selector {
        entry: identity_text.to_string(),
        source,
    };
    let include = resolve(&entry.header.include, &context).map_err(selector_error)?;
    let exclude = resolve(&entry.header.exclude, &context).map_err(selector_error)?;
    let found = candidates
        .iter()
        .filter(|directory| covers_any(&exclude, directory) == Coverage::Uncovered)
        .filter_map(|directory| {
            first_match(&include, directory).map(|found| SelectedDirectory {
                directory: directory.clone(),
                captures: found.captures,
                pattern: found.position,
            })
        })
        .collect::<Vec<_>>();
    Ok(Selected {
        include,
        exclude,
        found,
    })
}

/// The spec a spec entry applies, loaded, with the checks the entry runs recorded.
///
/// # Errors
///
/// `Unloaded` when the workspace did not load the spec.
fn loaded_spec<'w>(
    workspace: &'w LoadedWorkspace,
    working: &mut Working,
    entry: &SpecEntry,
    identity_text: &EntryIdentity,
) -> Result<&'w crate::domain::definitions::types::SpecFile, PlacementError> {
    for check in [COUNT, PARAMS, COVERAGE] {
        let _: bool = working
            .placement
            .evaluated
            .insert(check_id(identity_text, check));
    }
    let release = spec_release(entry);
    let spec = workspace
        .specs
        .get(&release)
        .ok_or_else(|| PlacementError::Unloaded {
            entry: identity_text.to_string(),
            release: release_id(&release).0,
        })?;
    if closes(workspace, &spec.composition) == Closing::Closes {
        let _: bool = working
            .placement
            .evaluated
            .insert(check_id(identity_text, CLOSED));
    }
    Ok(spec)
}

/// Applies one spec entry at a parent scope: records its counts and places an instance at each selected directory.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
fn apply(
    applying: Applying<'_>,
    working: &mut Working,
    parent: ScopeIndex,
    position: EntryIndex,
    entry: &SpecEntry,
    candidates: &[RepoPath],
) -> Result<(), PlacementError> {
    let Applying {
        workspace,
        compile,
        written,
    } = applying;
    let identity_text = identity(&entry.header, DefinitionKind::Spec, &entry.spec);
    let release = spec_release(entry);
    let spec = loaded_spec(workspace, working, entry, &identity_text)?;
    let parent_scope = working.placement.scopes[parent.0].clone();
    let anchor = parent_scope.anchor.clone();
    let Selected {
        include,
        exclude,
        found,
    } = selected(
        workspace,
        compile,
        working,
        &parent_scope.dir,
        entry,
        candidates,
        &identity_text,
    )?;
    let capture_names = names_of(&include);
    let tuples = found
        .iter()
        .map(|selected| selected.captures.clone())
        .collect::<BTreeSet<_>>();
    let counted = if capture_names.is_empty() {
        found.len()
    } else {
        tuples.len()
    };
    let grouped = grouping(&include);
    if !capture_names.is_empty() {
        let patterns = patterns_of(&include, &found);
        working.applied.push(AppliedWorking {
            applied: CapturingEntry {
                identity: Identity(identity_text.0.clone().into()),
                include: first_glob(entry),
                minimum: minimum(&entry.count).unwrap_or_default(),
                names: capture_names.clone(),
                tuples: tuples.clone(),
                scope: parent,
            },
            count: entry.count.clone(),
            grouped,
            patterns,
            exclude: exclude.clone(),
        });
    }
    for SelectedDirectory {
        directory,
        captures,
        ..
    } in found
    {
        if twice(working, &directory, &release, &identity_text) == Duplication::Twice {
            continue;
        }
        let captures = inheriting(&captures, &parent_scope.captures);
        let given = params(Instance {
            identity: &identity_text,
            entry,
            spec,
            anchor: &anchor,
            directory: &directory,
            parent: &parent_scope,
            captures: &captures,
            workspace: &workspace.section.name,
            written,
        });
        working.placement.findings.extend(given.findings);
        working.placement.scopes.push(Scope {
            anchor: directory.clone(),
            dir: directory,
            identity: Identity(identity_text.0.clone().into()),
            closed: spec.composition.closed,
            values: given.values,
            typed: given.typed,
            captures,
            composition: CompositionRef::Spec(release.clone()),
            rules_exclude: spec.rules_exclude.clone(),
            nested: Vec::new(),
            origin: Origin::Instance {
                parent,
                entry: position,
            },
        });
    }
    working.applications.push(CountedEntry {
        parent,
        identity: Identity(identity_text.0.into()),
        count: entry.count.clone(),
        counted: Amount(u32::try_from(counted).unwrap_or(u32::MAX)),
        grouped,
    });
    Ok(())
}
