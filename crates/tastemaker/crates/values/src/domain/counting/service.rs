//! The `counting` service: free functions named by verb that work out what the entries of a placement count and the
//! pairs their captures make, with no IO and no ports.

use std::collections::BTreeMap;

use super::types::Counted;
use crate::domain::captures::service::{
    captures_text, project, substitute_pattern, unescaped, wildcards,
};
use crate::domain::captures::types::Captures;
use crate::domain::captures::types::{CaptureBinding, Template, Wildcards};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, minimum};
use crate::domain::entries::types::Amount;
use crate::domain::findings::constants::{COUNT, MISSING};
use crate::domain::findings::service::{check_id, error};
use crate::domain::findings::types::Finding;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::service::join;
use crate::domain::paths::types::PathText;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{
    AppliedWorking, CapturesSpelling, CountCheck, CountMessage, CountUnit, CountedEntry,
    FileSpelling, Grouping, Identity, MatchedWorking, PairCheck, Scope, ScopeIndex,
};
use crate::domain::selectors::service::{covers_any, group};
use crate::domain::selectors::types::Coverage;

/// The counts and pairs a placement checks: the counts and pairs that read a condition, to judge once it is answered,
/// and the pairs of root spec and template entries, found now.
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
pub(crate) fn counts(
    scopes: &[Scope],
    matched: &[MatchedWorking],
    applied: &[AppliedWorking],
    applications: &[CountedEntry],
    workspace: &LoadedWorkspace,
) -> Counted {
    let entry_of = |found: &MatchedWorking| {
        let owner = &scopes[found.matched.scope.0];
        composition_of(workspace, &owner.composition).templates[found.matched.entry.0].clone()
    };
    let mut checks = Vec::new();
    for application in applications
        .iter()
        .filter(|application| application.grouped == Grouping::Whole)
    {
        let parent = &scopes[application.parent.0];
        checks.push(CountCheck {
            rule: check_id(&application.identity.0, COUNT),
            path: parent.anchor.clone(),
            scope: application.parent,
            count: application.count.clone(),
            found: application.counted,
            message: CountMessage::Spec {
                identity: application.identity.clone(),
            },
        });
    }
    for found in matched
        .iter()
        .filter(|found| found.matched.leading.is_empty())
    {
        let entry = entry_of(found);
        let scope = &scopes[found.matched.scope.0];
        let identity_text = identity(&entry.header, DefinitionKind::Template, &entry.template);
        checks.push(CountCheck {
            rule: check_id(&identity_text, COUNT),
            path: scope.anchor.clone(),
            scope: found.matched.scope,
            count: entry.count.clone(),
            found: found.matched.counted,
            message: CountMessage::Template {
                identity: Identity(identity_text.into()),
                file: FileSpelling(entry.file.0.clone()),
                unit: if found.matched.names.is_empty() {
                    CountUnit::Files
                } else {
                    CountUnit::Values
                },
                directory: scope.dir.clone(),
            },
        });
    }
    checks.extend(grouped(scopes, matched, applied, workspace));
    let pairs = pairs(scopes, matched, workspace);
    let found = spec_pairs(matched, applied, workspace);
    Counted {
        checks,
        pairs,
        findings: found,
    }
}

/// The counts of entries whose paths capture more than one name: each holds for each value of the captures before the
/// last, as the entry's own matches take it and as a spec entry of the same scope captures it under exactly those names.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
fn grouped(
    scopes: &[Scope],
    matched: &[MatchedWorking],
    applied: &[AppliedWorking],
    workspace: &LoadedWorkspace,
) -> Vec<CountCheck> {
    let mut entries = Vec::new();
    for working in applied
        .iter()
        .filter(|working| working.grouped == Grouping::Grouped)
    {
        for pattern in &working.patterns {
            entries.push((
                working.applied.identity.clone(),
                working.applied.scope,
                working.count.clone(),
                &pattern.leading,
                &pattern.tuples,
                &pattern.matcher,
                &working.exclude,
            ));
        }
    }
    for found in matched
        .iter()
        .filter(|found| !found.matched.leading.is_empty())
    {
        let owner = &scopes[found.matched.scope.0];
        let composition = composition_of(workspace, &owner.composition);
        let entry = &composition.templates[found.matched.entry.0];
        entries.push((
            Identity(identity(&entry.header, DefinitionKind::Template, &entry.template).into()),
            found.matched.scope,
            entry.count.clone(),
            &found.matched.leading,
            &found.matched.tuples,
            &found.matcher,
            &found.exclude,
        ));
    }
    let mut checks = Vec::new();
    for (identity_text, scope, count, leading, tuples, matcher, exclude) in entries {
        let owner = &scopes[scope.0];
        let mut groups: BTreeMap<Captures, usize> = BTreeMap::new();
        if leading.is_empty() {
            let _: Option<usize> = groups.insert(Captures::default(), 0);
        }
        for tuple in tuples {
            *groups.entry(project(tuple, leading)).or_default() += 1;
        }
        for other in applied {
            if other.applied.scope == scope && other.applied.names == *leading {
                for tuple in &other.applied.tuples {
                    let _: &mut usize = groups.entry(tuple.clone()).or_default();
                }
            }
        }
        for (captures, number) in groups {
            let directory = group(matcher, &captures);
            if number == 0
                && directory
                    .as_ref()
                    .is_some_and(|directory| covers_any(exclude, directory) == Coverage::Covered)
            {
                continue;
            }
            checks.push(CountCheck {
                rule: check_id(&identity_text.0, COUNT),
                path: directory.unwrap_or_else(|| owner.anchor.clone()),
                scope,
                count: count.clone(),
                found: Amount(u32::try_from(number).unwrap_or(u32::MAX)),
                message: CountMessage::Grouped {
                    identity: identity_text.clone(),
                    captures: CapturesSpelling(captures_text(&captures).into()),
                },
            });
        }
    }
    checks
}

/// Every captured value that has one file of a pair has the others its count requires: the findings, kept until the
/// count's condition is answered.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
fn pairs(
    scopes: &[Scope],
    matched: &[MatchedWorking],
    workspace: &LoadedWorkspace,
) -> Vec<PairCheck> {
    let entry_of = |found: &MatchedWorking| {
        let owner = &scopes[found.matched.scope.0];
        composition_of(workspace, &owner.composition).templates[found.matched.entry.0].clone()
    };
    let mut checks = Vec::new();
    for (position, required) in matched.iter().enumerate() {
        if required.matched.names.is_empty() {
            continue;
        }
        let entry = entry_of(required);
        let mut missing = BTreeMap::new();
        for (other_position, other) in matched.iter().enumerate() {
            if other_position == position
                || other.matched.scope != required.matched.scope
                || !required.matched.names.is_subset(&other.matched.names)
            {
                continue;
            }
            let other_entry = entry_of(other);
            for tuple in &other.matched.tuples {
                let projected = project(tuple, &required.matched.names);
                if !required.matched.tuples.contains(&projected) {
                    let _: &mut String = missing
                        .entry(projected)
                        .or_insert_with(|| other_entry.file.0.to_string());
                }
            }
        }
        if missing.is_empty() {
            continue;
        }
        let scope = &scopes[required.matched.scope.0];
        let identity_text = identity(&entry.header, DefinitionKind::Template, &entry.template);
        let mut findings = Vec::new();
        for (captures, other) in missing {
            let expected = substitute_pattern(
                Template(&entry.file.0),
                captures
                    .0
                    .iter()
                    .chain(scope.values.0.iter())
                    .map(|(name, value)| CaptureBinding { name, value }),
            );
            let path = if wildcards(Template(&expected)) == Wildcards::Held {
                scope.dir.clone()
            } else {
                join(&scope.dir, PathText(&unescaped(Template(&expected)).0))
                    .unwrap_or_else(|| scope.dir.clone())
            };
            findings.push(error(
                check_id(&identity_text, MISSING),
                path.clone(),
                format!(
                    "`{}` is missing: `{other}` has a file for {}, and `{identity_text}` requires its own",
                    path.0,
                    captures_text(&captures)
                ),
            ));
        }
        checks.push(PairCheck {
            scope: required.matched.scope,
            count: entry.count.clone(),
            findings,
        });
    }
    checks
}

/// A root spec entry and a root template entry that share a capture pair one to one: a captured value one has, the
/// other must have when its count requires it (`docs/{domain}` and `docs/{domain}.md`).
fn spec_pairs(
    matched: &[MatchedWorking],
    applied: &[AppliedWorking],
    workspace: &LoadedWorkspace,
) -> Vec<Finding> {
    let root_entries = matched
        .iter()
        .filter(|found| found.matched.scope == ScopeIndex(0))
        .filter_map(|found| {
            workspace
                .composition
                .templates
                .get(found.matched.entry.0)
                .map(|entry| (found, entry))
        })
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    for instances in applied {
        for (template, entry) in &root_entries {
            if template.matched.names.is_empty() {
                continue;
            }
            let template_identity =
                identity(&entry.header, DefinitionKind::Template, &entry.template);
            if minimum(&entry.count).unwrap_or_default() > Amount(0)
                && template.matched.names.is_subset(&instances.applied.names)
            {
                for tuple in &instances.applied.tuples {
                    let projected = project(tuple, &template.matched.names);
                    if !template.matched.tuples.contains(&projected) {
                        let path = unescaped(Template(&substitute_pattern(
                            Template(&entry.file.0),
                            pairs_of(&projected),
                        )))
                        .0;
                        findings.push(error(
                            check_id(&template_identity, MISSING),
                            join(&workspace.directory, PathText(&path)).unwrap_or_default(),
                            format!(
                                "`{path}` is missing: `{}` has an instance for {}, and `{template_identity}` requires its own",
                                instances.applied.identity.0,
                                captures_text(&projected)
                            ),
                        ));
                    }
                }
            }
            if instances.applied.minimum > Amount(0)
                && instances.applied.names.is_subset(&template.matched.names)
            {
                for tuple in &template.matched.tuples {
                    let projected = project(tuple, &instances.applied.names);
                    if !instances.applied.tuples.contains(&projected) {
                        let path = unescaped(Template(&substitute_pattern(
                            Template(&instances.applied.include.0),
                            pairs_of(&projected),
                        )))
                        .0;
                        findings.push(error(
                            check_id(&instances.applied.identity.0, MISSING),
                            join(&workspace.directory, PathText(&path)).unwrap_or_default(),
                            format!(
                                "`{path}` is missing: `{template_identity}` has a file for {}, and `{}` requires an instance there",
                                captures_text(&projected),
                                instances.applied.identity.0
                            ),
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// The captures as the name and value bindings a path template substitutes.
fn pairs_of(captures: &Captures) -> impl Iterator<Item = CaptureBinding<'_>> {
    captures
        .0
        .iter()
        .map(|(name, value)| CaptureBinding { name, value })
}
