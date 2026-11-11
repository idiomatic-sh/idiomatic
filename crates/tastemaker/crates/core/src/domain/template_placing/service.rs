// ignore: idiomatic.arch/file-length - placing templates is one matching pass: each entry's files matched, the most specific entry kept and a file placed twice reported, over one shared set of candidate files.
//! The `template_placing` service: free functions named by verb that place the files of every template entry of a
//! placement's scopes, and find the files a `rules-exclude` covers, with no IO and no ports.

use crate::domain::captures::service::pieces;
use crate::domain::captures::types::{PatternPiece, Template};
use std::collections::{BTreeMap, BTreeSet};

use rayon::prelude::*;

use super::errors::TemplatePlacingError;
use super::types::{Beside, Known, MatchedFile, PartNaming, TemplateMatch, TemplatesPlaced};
use crate::domain::captures::types::{Captures, Interpolations};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, reference};
use crate::domain::entries::types::{Amount, EntryIdentity, TemplateEntry};
use crate::domain::findings::constants::{COUNT, COVERAGE, MISSING, TEMPLATE};
use crate::domain::findings::service::{check_id, error};
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::service::{beneath, containment, join, parent};
use crate::domain::paths::types::{Containment, PathText, RepoPath};
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{
    CompositionRef, EntryIndex, Matched, MatchedWorking, PlacedFile, Scope, ScopeIndex,
};
use crate::domain::selectors::service::{
    covering, covers_any, leading, names, place as place_pattern, resolve,
};
use crate::domain::selectors::types::{
    Coverage, Match, PartInstance, PathMatcher, PatternCompiler, PlacedPath, ResolvedSelectors,
    SelectionContext, Selector, Selectors,
};

/// Whether one of `lists` names instances or placed files, which are then gathered for it.
#[must_use]
pub fn names_parts(lists: &[&Selectors]) -> PartNaming {
    let names = lists.iter().any(|list| {
        list.0
            .iter()
            .any(|selector| matches!(selector, Selector::Part(_) | Selector::PlacedBy(_)))
    });
    if names {
        PartNaming::NamesParts
    } else {
        PartNaming::NamesNone
    }
}

/// The instances and placed files a selector of a scope can name; none gathered when its lists name neither.
#[must_use]
pub fn known(
    scopes: &[Scope],
    placed: &BTreeMap<RepoPath, PlacedFile>,
    workspace: &LoadedWorkspace,
    lists: &[&Selectors],
) -> Known {
    if names_parts(lists) == PartNaming::NamesNone {
        return Known::default();
    }
    let instances = scopes
        .iter()
        .filter_map(|scope| match &scope.composition {
            CompositionRef::Spec(release) => Some(PartInstance {
                directory: scope.dir.clone(),
                spec: release.name.clone(),
            }),
            CompositionRef::Root | CompositionRef::Local(_) => None,
        })
        .collect();
    let placed = placed
        .iter()
        .filter_map(|(path, file)| {
            let scope = scopes.get(file.scope.0)?;
            let composition = composition_of(workspace, &scope.composition);
            let entry = composition.templates.get(file.entry.0)?;
            Some(PlacedPath {
                path: path.clone(),
                entry: reference(&entry.header, &entry.template).clone(),
            })
        })
        .collect();
    Known { instances, placed }
}

/// Every template entry of every scope, matched and recorded into `placed`: what that finds, the checks that ran and
/// what each entry matched.
///
/// # Errors
///
/// `Selector` when an entry's file pattern or its `exclude` does not compile.
pub(crate) fn templates(
    workspace: &LoadedWorkspace,
    compile: &dyn PatternCompiler,
    scopes: &[Scope],
    placed: &mut BTreeMap<RepoPath, PlacedFile>,
) -> Result<TemplatesPlaced, TemplatePlacingError> {
    // The template entries in order. Each matches on its own, all in parallel; then each is recorded in that order, so
    // a file two entries match is the first's. An `exclude` naming placed files reads those placed before it, so then
    // each entry matches only once those before it are recorded.
    let jobs = scopes
        .iter()
        .enumerate()
        .flat_map(|(index, scope)| {
            let composition = composition_of(workspace, &scope.composition);
            composition
                .templates
                .iter()
                .enumerate()
                .map(move |(position, entry)| {
                    (scope, ScopeIndex(index), EntryIndex(position), entry)
                })
        })
        .collect::<Vec<_>>();
    let mut record = TemplatesPlaced::default();
    let read: &BTreeMap<RepoPath, PlacedFile> = placed;
    let found = jobs
        .par_iter()
        .map(|(scope, _, _, entry)| template_match(workspace, compile, scopes, read, scope, entry))
        .collect::<Vec<_>>();
    // A file several entries match is placed by the most specific: a capture never takes a path a more written-out
    // pattern places.
    let mut best: BTreeMap<RepoPath, Amount> = BTreeMap::new();
    for ((scope, _, _, entry), matched) in jobs.iter().zip(&found) {
        let own = specificity(scope, entry);
        for file in matched.iter().flat_map(|matched| &matched.files) {
            let _: &mut Amount = best
                .entry(file.file.clone())
                .and_modify(|most| *most = (*most).max(own))
                .or_insert(own);
        }
    }
    let specific = |scope: &Scope, entry: &TemplateEntry, mut matched: TemplateMatch| {
        let own = specificity(scope, entry);
        matched
            .files
            .retain(|file| best.get(&file.file).is_none_or(|most| *most <= own));
        matched
    };
    if jobs
        .iter()
        .any(|(_, _, _, entry)| names_parts(&[&entry.header.exclude]) == PartNaming::NamesParts)
    {
        for (scope, index, position, entry) in jobs {
            let specified = specific(
                scope,
                entry,
                template_match(workspace, compile, scopes, placed, scope, entry)?,
            );
            template_record(
                workspace,
                scopes,
                placed,
                &mut record,
                index,
                position,
                specified,
            );
        }
    } else {
        for ((scope, index, position, entry), ready) in jobs.into_iter().zip(found) {
            template_record(
                workspace,
                scopes,
                placed,
                &mut record,
                index,
                position,
                specific(scope, entry, ready?),
            );
        }
    }
    Ok(record)
}

/// How many segments of the path an entry of `scope` places its template at are written out: those of the scope's
/// directory, and each segment of its `file` with no glob and no capture, a value of the scope's put in place counting
/// as written out, and a `{name:dir}` counting as any capture.
fn specificity(scope: &Scope, entry: &TemplateEntry) -> Amount {
    let written = entry.file.0.split('/').filter(|segment| {
        pieces(Template(segment)).iter().all(|piece| match piece {
            PatternPiece::Literal(_) => true,
            PatternPiece::Capture(name)
            | PatternPiece::Span(name)
            | PatternPiece::Directory(name) => {
                scope.values.0.keys().any(|known| &*known.0 == name.0)
            }
            PatternPiece::Star | PatternPiece::Question => false,
        })
    });
    let count = scope
        .dir
        .0
        .split('/')
        .filter(|part| !part.is_empty())
        .count()
        + written.count();
    Amount(u32::try_from(count).unwrap_or(u32::MAX))
}

/// The files a template entry of `scope` matches beneath it, not in a nested instance, not excluded, and beside the
/// directory each `{name:dir}` names: a file left out here raises no entry's specificity, so a less specific entry can
/// place it.
///
/// # Errors
///
/// `Selector` when its file pattern or its `exclude` does not compile.
fn template_match(
    workspace: &LoadedWorkspace,
    compile: &dyn PatternCompiler,
    scopes: &[Scope],
    placed: &BTreeMap<RepoPath, PlacedFile>,
    scope: &Scope,
    entry: &TemplateEntry,
) -> Result<TemplateMatch, TemplatePlacingError> {
    let identity_text = identity(&entry.header, DefinitionKind::Template, &entry.template);
    let selector_error = |source| TemplatePlacingError::Selector {
        entry: identity_text.to_string(),
        source,
    };
    let matcher =
        place_pattern(&scope.dir, &entry.file, &scope.values, compile).map_err(selector_error)?;
    let known = known(scopes, placed, workspace, &[&entry.header.exclude]);
    let context = SelectionContext {
        base: &scope.dir,
        values: &scope.values,
        instances: &known.instances,
        placed: &known.placed,
        compile,
    };
    let exclude = resolve(&entry.header.exclude, &context).map_err(selector_error)?;
    let mut files = Vec::new();
    // The prefix itself, when it is a file: `binary_search` answers where it is, or where it would go.
    let exact = match workspace.files.binary_search(&matcher.prefix) {
        Ok(found) => workspace.files.get(found..=found).unwrap_or_default(),
        Err(_insertion) => &[],
    };
    for file in exact
        .iter()
        .chain(beneath(&workspace.files, &matcher.prefix))
    {
        if containment(file, &scope.dir) == Containment::Outside
            || scope
                .nested
                .iter()
                .any(|nested| containment(file, nested) == Containment::Inside)
            || covers_any(&exclude, file) == Coverage::Covered
        {
            continue;
        }
        if let Match::Matched(captures) = crate::domain::selectors::service::matches(&matcher, file)
            && beside(&matcher, file, &captures, &workspace.files) == Beside::Present
        {
            files.push(MatchedFile {
                file: file.clone(),
                captures,
            });
        }
    }
    Ok(TemplateMatch {
        identity: identity_text,
        files,
        matcher,
        exclude,
    })
}

/// Whether each `{name:dir}` of the matcher names a directory beside `file` that holds one of the sorted `files`: its
/// captured value, or the scope's value put in place.
fn beside(
    matcher: &PathMatcher,
    file: &RepoPath,
    captures: &Captures,
    files: &[RepoPath],
) -> Beside {
    let Some(holding) = parent(file) else {
        return Beside::Absent;
    };
    let all = matcher.directories.iter().all(|name| {
        captures
            .0
            .get(name)
            .or_else(|| matcher.values.0.get(name))
            .and_then(|value| join(&holding, PathText(&value.0)))
            // `.` and `..` name no directory beside the file.
            .filter(|directory| parent(directory).as_ref() == Some(&holding))
            .is_some_and(|directory| !beneath(files, &directory).is_empty())
    });
    if all { Beside::Present } else { Beside::Absent }
}

/// Records what a template entry of the scope at `index` matched: each file placed by it, unless an entry before placed
/// it (a `#coverage` error), and what its count is judged from.
fn template_record(
    workspace: &LoadedWorkspace,
    scopes: &[Scope],
    placed: &mut BTreeMap<RepoPath, PlacedFile>,
    record: &mut TemplatesPlaced,
    index: ScopeIndex,
    position: EntryIndex,
    found: TemplateMatch,
) {
    let TemplateMatch {
        identity: identity_text,
        files,
        matcher,
        exclude,
    } = found;
    for check in [COUNT, MISSING, COVERAGE, TEMPLATE] {
        let _: bool = record.evaluated.insert(check_id(&identity_text, check));
    }
    let file_names = names(&matcher);
    let tuples = files
        .iter()
        .map(|matched| matched.captures.clone())
        .collect::<BTreeSet<_>>();
    let counted = if file_names.is_empty() {
        files.len()
    } else {
        tuples.len()
    };
    for MatchedFile { file, captures } in files {
        if let Some(previous) = placed.get(&file) {
            record.findings.push(placed_twice(
                workspace,
                scopes,
                previous,
                &identity_text,
                &file,
            ));
            continue;
        }
        drop(placed.insert(
            file,
            PlacedFile {
                scope: index,
                entry: position,
                captures,
            },
        ));
    }
    let leading_names = leading(&matcher);
    record.matched.push(MatchedWorking {
        matched: Matched {
            scope: index,
            entry: position,
            names: file_names,
            tuples,
            counted: Amount(u32::try_from(counted).unwrap_or(u32::MAX)),
            leading: leading_names,
        },
        matcher,
        exclude,
    });
}

/// The `#coverage` error of a file an entry before already placed.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
fn placed_twice(
    workspace: &LoadedWorkspace,
    scopes: &[Scope],
    previous: &PlacedFile,
    identity_text: &EntryIdentity,
    file: &RepoPath,
) -> crate::domain::findings::types::Finding {
    let previous_scope = &scopes[previous.scope.0];
    let other = &composition_of(workspace, &previous_scope.composition).templates[previous.entry.0];
    let previous_identity = identity(&other.header, DefinitionKind::Template, &other.template);
    error(
        check_id(identity_text, COVERAGE),
        file.clone(),
        format!(
            "`{}` is placed by both `{previous_identity}` and `{identity_text}`: a file is placed by one template entry",
            file.0
        ),
    )
}

/// The files a `rules-exclude` covers, the workspace's from the repository root and each scope's own from its directory:
/// only placement and `closed` read them.
///
/// # Errors
///
/// `Selector` when a `rules-exclude` does not compile.
pub fn exclusion(
    workspace: &LoadedWorkspace,
    compile: &dyn PatternCompiler,
    scopes: &[Scope],
) -> Result<BTreeSet<RepoPath>, TemplatePlacingError> {
    let none = Interpolations::default();
    let base = workspace.directory.clone();
    let mut lists: Vec<ResolvedSelectors> = Vec::new();
    let workspace_context = SelectionContext {
        base: &base,
        values: &none,
        instances: &[],
        placed: &[],
        compile,
    };
    let selector_error = |source| TemplatePlacingError::Selector {
        entry: super::constants::WORKSPACE_TABLE.to_owned(),
        source,
    };
    lists.push(
        resolve(&workspace.section.rules_exclude, &workspace_context).map_err(selector_error)?,
    );
    for scope in scopes {
        if scope.rules_exclude.0.is_empty() {
            continue;
        }
        let context = SelectionContext {
            base: &scope.dir,
            values: &scope.values,
            instances: &[],
            placed: &[],
            compile,
        };
        lists.push(resolve(&scope.rules_exclude, &context).map_err(selector_error)?);
    }
    // Each list is read once over the files, each directory once.
    let files = workspace.files.iter().collect::<Vec<_>>();
    let mut covered = vec![Coverage::Uncovered; files.len()];
    for list in &lists {
        for (slot, coverage) in covered.iter_mut().zip(covering(list, &files)) {
            if coverage == Coverage::Covered {
                *slot = Coverage::Covered;
            }
        }
    }
    Ok(files
        .into_iter()
        .zip(covered)
        .filter(|(_, covered)| *covered == Coverage::Covered)
        .map(|(file, _)| file.clone())
        .collect())
}
