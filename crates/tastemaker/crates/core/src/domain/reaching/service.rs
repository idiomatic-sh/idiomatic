//! The `reaching` service: free functions named by verb that find the files each rule entry of each scope reaches, with
//! no IO and no ports. A scope's entries are matched together: every glob of them compiled into one set, so each file is
//! tested once.

use super::constants::PATTERN_JOIN;
use std::collections::{BTreeMap, HashMap};

use rayon::prelude::*;

use super::types::{
    EntryLists, Item, Mark, ScopeLists, ScopeReach, Selection, SelectorPair, SetWidth,
};
use crate::domain::applications::constants::PATH_SEPARATOR;
use crate::domain::applications::types::{Carried, Compilers, Nameable};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::identity;
use crate::domain::entries::types::RuleEntry;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::service::beneath;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::types::{Scope, ScopeIndex};
use crate::domain::selectors::service::{covers_any, resolve};
use crate::domain::selectors::types::{
    Coverage, GlobForm, GlobIndex, MatcherShape, PathRest, PathSet, PatternCompiler, Resolved,
    ResolvedSelectors, SelectionContext, SetGlob,
};

/// What each scope's rule entries select, by the scope and the entry's `include` and `exclude`: the scopes in parallel,
/// and in each scope every glob of its entries compiled into one set, so each file is tested once.
#[must_use]
pub fn reaches<'w>(
    workspace: &'w LoadedWorkspace,
    placement: &'w Placement,
    applications: &[Carried<'w>],
    known: &Nameable,
    matching: Compilers<'_>,
) -> HashMap<Selection<'w>, Result<Vec<RepoPath>, PlacementError>> {
    let mut by_scope: BTreeMap<ScopeIndex, BTreeMap<SelectorPair<'w>, &'w RuleEntry>> =
        BTreeMap::new();
    for application in applications {
        let entry = application.entry;
        let _: &mut &RuleEntry = by_scope
            .entry(application.scope)
            .or_default()
            .entry(SelectorPair {
                include: &entry.header.include,
                exclude: &entry.header.exclude,
            })
            .or_insert(entry);
    }
    by_scope
        .into_par_iter()
        .flat_map_iter(|(index, entries)| {
            placement
                .scopes
                .get(index.0)
                .into_iter()
                .flat_map(move |scope| {
                    scope_reaches(workspace, placement, scope, &entries, known, matching)
                        .into_iter()
                        .map(move |reach| {
                            (
                                Selection {
                                    scope: index,
                                    pair: reach.pair,
                                },
                                reach.files,
                            )
                        })
                })
        })
        .collect()
}

/// The items of a resolved list, by the marks the scope's set gave them.
fn items<'r>(marks: &[Mark], resolved: &'r ResolvedSelectors) -> Vec<Item<'r>> {
    marks
        .iter()
        .zip(&resolved.0)
        .map(|(mark, item)| match mark {
            Mark::Alone => Item::Alone(item),
            Mark::Whole => Item::Whole,
            Mark::Glob(position) => Item::Glob(*position),
        })
        .collect()
}

/// The files each entry of one scope reaches: the scope's governed files, those in its nested instances included, not
/// under a `rules-exclude`, that its `include` covers and its `exclude` does not. Every glob of the entries is compiled into one
/// set, so each file is tested once, and each of its directories within the scope once, as a directory a pattern
/// matches covers what it holds.
fn scope_reaches<'w>(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    scope: &Scope,
    entries: &BTreeMap<SelectorPair<'w>, &'w RuleEntry>,
    known: &Nameable,
    matching: Compilers<'_>,
) -> Vec<ScopeReach<'w>> {
    let ScopeLists { lists, mut answers } = scope_lists(scope, entries, known, matching.compile);
    let mut globs: Vec<SetGlob> = Vec::new();
    let marked = lists
        .iter()
        .map(|resolved| {
            (
                marks(&mut globs, &scope.dir, &resolved.include),
                marks(&mut globs, &scope.dir, &resolved.exclude),
            )
        })
        .collect::<Vec<_>>();
    let shaped = lists
        .iter()
        .zip(&marked)
        .map(|(resolved, (included, excluded))| EntryLists {
            pair: resolved.pair,
            include: items(included, &resolved.include),
            exclude: items(excluded, &resolved.exclude),
        })
        .collect::<Vec<_>>();
    let texts = globs
        .iter()
        .map(|glob| SetGlob(glob.0.clone()))
        .collect::<Vec<_>>();
    match matching.sets.compile_set(&texts) {
        Ok(set) => {
            let reached = scope_files(
                workspace,
                placement,
                scope,
                set.as_ref(),
                &shaped,
                SetWidth(globs.len()),
            );
            answers.extend(
                shaped
                    .into_iter()
                    .zip(reached)
                    .map(|(entry, files)| ScopeReach {
                        pair: entry.pair,
                        files: Ok(files),
                    }),
            );
        }
        Err(failure) => {
            // The set fails as a whole: each pattern is compiled alone to name the ones that fail, so the error says which
            // pattern to fix, never an empty one.
            let failing = texts
                .iter()
                .filter(|glob| {
                    matching
                        .sets
                        .compile_set(&[SetGlob(glob.0.clone())])
                        .is_err()
                })
                .map(|glob| glob.0.to_string())
                .collect::<Vec<_>>();
            // None fails alone: the set is named whole.
            let failing = if failing.is_empty() {
                texts
                    .iter()
                    .map(|glob| glob.0.to_string())
                    .collect::<Vec<_>>()
            } else {
                failing
            }
            .join(PATTERN_JOIN);
            answers.extend(shaped.into_iter().map(|entry| {
                let rule = entries[&entry.pair];
                ScopeReach {
                    pair: entry.pair,
                    files: Err(PlacementError::Selector {
                        entry: identity(&rule.header, DefinitionKind::Rule, &rule.rule).0,
                        source: crate::domain::selectors::errors::SelectorsError::Pattern {
                            pattern: failing.clone(),
                            reason: failure.0.to_string(),
                        },
                    }),
                }
            }));
        }
    }
    answers
}

/// Each entry's `include` and `exclude` resolved at the scope, and the answers of those that do not resolve.
fn scope_lists<'w>(
    scope: &Scope,
    entries: &BTreeMap<SelectorPair<'w>, &'w RuleEntry>,
    known: &Nameable,
    compile: &dyn PatternCompiler,
) -> ScopeLists<'w> {
    let context = SelectionContext {
        base: &scope.dir,
        values: &scope.values,
        instances: &known.instances,
        placed: &known.placed,
        compile,
    };
    let mut answers = Vec::new();
    let mut lists = Vec::new();
    for (&pair, entry) in entries {
        let failed = |source| PlacementError::Selector {
            entry: identity(&entry.header, DefinitionKind::Rule, &entry.rule).0,
            source,
        };
        match resolve(pair.include, &context)
            .map_err(failed)
            .and_then(|include| {
                resolve(pair.exclude, &context)
                    .map_err(failed)
                    .map(|exclude| (include, exclude))
            }) {
            Ok((include, exclude)) => lists.push(EntryLists {
                pair,
                include,
                exclude,
            }),
            Err(failure) => answers.push(ScopeReach {
                pair,
                files: Err(failure),
            }),
        }
    }
    ScopeLists { lists, answers }
}

/// Each item of `resolved` marked: a glob at the scope that decides alone by its place in `globs` (added once), the
/// pattern `.` as everything, anything else as tested alone.
fn marks(globs: &mut Vec<SetGlob>, dir: &RepoPath, resolved: &ResolvedSelectors) -> Vec<Mark> {
    resolved
        .0
        .iter()
        .map(|item| match item {
            Resolved::Pattern(matcher) if matcher.base == *dir => match &matcher.shape {
                MatcherShape::Compiled(compiled) => match &compiled.glob {
                    GlobForm::Alone(glob) => Mark::Glob(GlobIndex(
                        globs
                            .iter()
                            .position(|known| *known.0 == *glob.0)
                            .unwrap_or_else(|| {
                                globs.push(SetGlob(glob.0.clone()));
                                globs.len() - 1
                            }),
                    )),
                    GlobForm::Bound => Mark::Alone,
                },
                MatcherShape::Itself => Mark::Whole,
            },
            Resolved::Pattern(_) | Resolved::Paths(_) => Mark::Alone,
        })
        .collect()
}

/// The scope's candidate files each entry reaches, in the entries' order.
fn scope_files(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    scope: &Scope,
    set: &dyn PathSet,
    shaped: &[EntryLists<'_, Vec<Item<'_>>>],
    width: SetWidth,
) -> Vec<Vec<RepoPath>> {
    let candidates = beneath(&workspace.files, &scope.dir)
        .iter()
        .filter(|file| !placement.excluded.contains(*file));
    let prefix = scope.dir.0.len() + usize::from(!scope.dir.0.is_empty());
    let mut directories: HashMap<PathRest<'_>, Vec<GlobIndex>> = HashMap::new();
    let mut passing = Vec::new();
    let mut hit = vec![false; width.0];
    let mut reached: Vec<Vec<RepoPath>> = vec![Vec::new(); shaped.len()];
    for file in candidates {
        hit.fill(false);
        let rest = file.0.get(prefix..).unwrap_or_default();
        set.passing(PathRest(rest), &mut passing);
        for position in &passing {
            if let Some(mark) = hit.get_mut(position.0) {
                *mark = true;
            }
        }
        let mut above = rest;
        while let Some((directory, _)) = above.rsplit_once(PATH_SEPARATOR) {
            above = directory;
            let found = directories.entry(PathRest(directory)).or_insert_with(|| {
                let mut found = Vec::new();
                set.passing(PathRest(directory), &mut found);
                found
            });
            for position in found.iter() {
                if let Some(mark) = hit.get_mut(position.0) {
                    *mark = true;
                }
            }
        }
        let covered = |list: &[Item<'_>]| {
            list.iter().any(|item| match item {
                // ignore: idiomatic.arch/no-unwrap-or-on-lookup - `hit` holds one flag per glob of the same compiled set that gives `position`, so the lookup never misses; the flag table is the hot loop of reaching, and a set in its place would cost every file
                Item::Glob(position) => hit.get(position.0).copied().unwrap_or(false),
                Item::Whole => true,
                Item::Alone(resolved) => {
                    let one = ResolvedSelectors(vec![(*resolved).clone()]);
                    covers_any(&one, file) == Coverage::Covered
                }
            })
        };
        for (slot, entry) in reached.iter_mut().zip(shaped) {
            if covered(&entry.include) && !covered(&entry.exclude) {
                slot.push(file.clone());
            }
        }
    }
    reached
}
