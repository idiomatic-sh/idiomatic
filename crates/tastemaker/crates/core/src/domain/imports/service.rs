//! The `imports` service: free functions named by verb that read the imports files make from the rows of the import
//! relations and judge them against the entries' `imports`, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::constants::{BENCHES, IMPORTS, TEST_SUFFIX, TESTS};
use super::types::{
    AllowItem, Breach, CodeKind, EntryImports, EntryName, EntryPlace, Import, Indexes,
    InstanceName, Member, PatternMatch, Refused, ScopeImports, Target, WrittenPath,
};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, reference};
use crate::domain::entries::types::{ImportDefault, Imports};
use crate::domain::findings::service::check_id;
use crate::domain::findings::types::MessageText;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::service::containment;
use crate::domain::paths::types::Containment;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{EntryIndex, Origin, ScopeIndex};

/// The lookups `checks` reads, made once.
fn indexes<'p>(workspace: &LoadedWorkspace, placement: &'p Placement) -> Indexes<'p> {
    let extra = placement
        .scopes
        .iter()
        .filter(|scope| matches!(scope.origin, Origin::File))
        .filter_map(|scope| {
            let allow = &composition_of(workspace, &scope.composition).imports_allow;
            (!allow.is_empty()).then(|| {
                (
                    scope.dir.clone(),
                    allow
                        .iter()
                        .map(|item| AllowItem(item.0.clone()))
                        .collect::<Vec<_>>(),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let mut placed_by: HashMap<EntryPlace, Vec<&RepoPath>> = HashMap::new();
    for (file, placed) in &placement.placed {
        if !placement.excluded.contains(file) {
            placed_by
                .entry(EntryPlace {
                    scope: placed.scope,
                    entry: placed.entry,
                })
                .or_default()
                .push(file);
        }
    }
    let mut instances_of: HashMap<EntryPlace, Vec<ScopeIndex>> = HashMap::new();
    for (child, scope) in placement.scopes.iter().enumerate() {
        if let Origin::Instance { parent, entry } = scope.origin {
            instances_of
                .entry(EntryPlace {
                    scope: parent,
                    entry,
                })
                .or_default()
                .push(ScopeIndex(child));
        }
    }
    Indexes {
        extra,
        placed_by,
        instances_of,
    }
}

/// What the imports of each scope that has an entry with an `imports` are held to: every entry of the scope by name, the
/// files each is, and the instances of the entries whose `imports` hold them. A file under a `rules-exclude` is placed,
/// never an entry's member.
#[must_use]
pub fn checks(workspace: &LoadedWorkspace, placement: &Placement) -> Vec<ScopeImports> {
    let Indexes {
        extra,
        placed_by,
        instances_of,
    } = indexes(workspace, placement);
    let within = |dir: &RepoPath| {
        workspace
            .files
            .iter()
            .filter(|file| {
                containment(file, dir) == Containment::Inside && !placement.excluded.contains(*file)
            })
            .cloned()
            .collect::<BTreeSet<_>>()
    };
    let mut found = Vec::new();
    for (index, scope) in placement.scopes.iter().enumerate() {
        let composition = composition_of(workspace, &scope.composition);
        let restricted = composition
            .specs
            .iter()
            .any(|entry| matches!(entry.imports, Imports::Restricted { .. }))
            || composition
                .templates
                .iter()
                .any(|entry| matches!(entry.imports, Imports::Restricted { .. }));
        if !restricted {
            continue;
        }
        let mut members: BTreeMap<EntryName, Vec<Member>> = BTreeMap::new();
        let mut checks = Vec::new();
        let place = |position| EntryPlace {
            scope: ScopeIndex(index),
            entry: EntryIndex(position),
        };
        for (position, entry) in composition.specs.iter().enumerate() {
            let name = EntryName(reference(&entry.header, &entry.spec).0.clone());
            let instances = instances_of
                .get(&place(position))
                .into_iter()
                .flatten()
                .filter_map(|child| {
                    let dir = &placement.scopes.get(child.0)?.dir;
                    Some(Member {
                        instance: InstanceName(dir.0.clone()),
                        files: within(dir),
                        extra: extra.get(dir).cloned().unwrap_or_default(),
                    })
                })
                .collect::<Vec<_>>();
            if let Imports::Restricted { default, allow } = &entry.imports {
                checks.push(EntryImports {
                    identity: identity(&entry.header, DefinitionKind::Spec, &entry.spec),
                    default: *default,
                    allow: allow.iter().map(|item| AllowItem(item.0.clone())).collect(),
                    members: instances.clone(),
                });
            }
            members.entry(name).or_default().extend(instances);
        }
        for (position, entry) in composition.templates.iter().enumerate() {
            let name = EntryName(reference(&entry.header, &entry.template).0.clone());
            let instances = placed_by
                .get(&place(position))
                .into_iter()
                .flatten()
                .map(|file| Member {
                    instance: InstanceName(file.0.clone()),
                    files: BTreeSet::from([(*file).clone()]),
                    extra: Vec::new(),
                })
                .collect::<Vec<_>>();
            if let Imports::Restricted { default, allow } = &entry.imports {
                checks.push(EntryImports {
                    identity: identity(&entry.header, DefinitionKind::Template, &entry.template),
                    default: *default,
                    allow: allow.iter().map(|item| AllowItem(item.0.clone())).collect(),
                    members: instances.clone(),
                });
            }
            members.entry(name).or_default().extend(instances);
        }
        found.push(ScopeImports {
            entries_of: entries_of(&members),
            names: members.into_keys().collect(),
            checks,
        });
    }
    found
}

/// The entries each file of a scope is of, by name, from each entry's members.
fn entries_of(members: &BTreeMap<EntryName, Vec<Member>>) -> BTreeMap<RepoPath, Vec<EntryName>> {
    let mut found: BTreeMap<RepoPath, Vec<EntryName>> = BTreeMap::new();
    for (name, instances) in members {
        for file in instances.iter().flat_map(|member| &member.files) {
            found.entry(file.clone()).or_default().push(name.clone());
        }
    }
    found
}

/// Whether `path` is test code: a file under a `tests` or `benches` directory, or a `_test` twin of the file it proves.
/// No `imports` reads its imports: a test uses the code it proves as any consumer would.
#[must_use]
pub(crate) fn test_code(path: &RepoPath) -> CodeKind {
    let mut parts = path.0.rsplit('/');
    let stem = parts
        .next()
        .map(|name| name.rsplit_once('.').map_or(name, |(stem, _)| stem))
        .unwrap_or_default();
    if stem.ends_with(TEST_SUFFIX) || parts.any(|part| part == TESTS || part == BENCHES) {
        CodeKind::Test
    } else {
        CodeKind::Production
    }
}

/// What each entry's `imports` of `scope` refuses of the production imports its instances make. `matches` says whether
/// a module pattern matches a written path or a module it lies in.
#[must_use]
pub fn refused(
    scope: &ScopeImports,
    by_source: &BTreeMap<&RepoPath, Vec<&Import>>,
    matches: &dyn Fn(&AllowItem, &WrittenPath) -> PatternMatch,
) -> Vec<Refused> {
    let mut found = Vec::new();
    for check in &scope.checks {
        for member in &check.members {
            let allow = check.allow.iter().chain(&member.extra).collect::<Vec<_>>();
            for file in &member.files {
                for import in by_source.get(file).map(Vec::as_slice).unwrap_or_default() {
                    let internal = matches!(&import.target, Target::File(target) if member.files.contains(target));
                    if internal {
                        continue;
                    }
                    let targets = match &import.target {
                        Target::File(target) => scope
                            .entries_of
                            .get(target)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                        Target::Module => &[],
                    };
                    let allowed = allow.iter().any(|item| {
                        if scope.names.contains(&*item.0) {
                            targets.iter().any(|name| name.0 == item.0)
                        } else {
                            matches(item, &import.written) == PatternMatch::Matches
                        }
                    });
                    if check.default == ImportDefault::Deny && !allowed {
                        found.push(Refused {
                            identity: check.identity.clone(),
                            instance: member.instance.clone(),
                            import: (*import).clone(),
                            breach: Breach::NotAllowed,
                        });
                    }
                }
            }
        }
    }
    found
}

/// The check a refused import is a finding of: `<identity>#imports`.
#[must_use]
pub fn rule_of(refused: &Refused) -> crate::domain::findings::types::FindingRule {
    check_id(&refused.identity.0, IMPORTS)
}

/// What the finding of a refused import says.
#[must_use]
pub fn message(refused: &Refused) -> MessageText {
    MessageText(
        match refused.breach {
            Breach::NotAllowed => format!(
                "`{}` is imported within `{}` at `{}`, which its `imports` does not allow",
                refused.import.written, refused.identity.0, refused.instance
            ),
        }
        .into(),
    )
}

/// What the finding of an `imports.allow` module pattern that does not compile says.
#[must_use]
pub fn unreadable_allow(
    identity: &crate::domain::entries::types::EntryIdentity,
    item: &AllowItem,
    failure: &crate::domain::selectors::types::PatternFailure,
) -> MessageText {
    MessageText(
        format!(
            "`{}` in the `imports.allow` of `{}` is no module pattern: {}",
            item.0, identity.0, failure.0
        )
        .into(),
    )
}

/// The production imports, by the file that makes them, of the files `excluded` does not hold.
#[must_use]
pub fn by_source<'i>(
    imports: &'i [Import],
    excluded: &BTreeSet<RepoPath>,
) -> BTreeMap<&'i RepoPath, Vec<&'i Import>> {
    let mut found: BTreeMap<&RepoPath, Vec<&Import>> = BTreeMap::new();
    for import in imports.iter().filter(|import| {
        test_code(&import.source) == CodeKind::Production && !excluded.contains(&import.source)
    }) {
        found.entry(&import.source).or_default().push(import);
    }
    found
}
