//! The `applications` service: free functions named by verb that apply a rule entry where a scope places it, with no IO
//! and no ports: the rule it names loaded, its params resolved and bound, what its selectors can name, and the rows of
//! the files the entries placed.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::EXCEPTIONS;
use super::types::{
    Compared, Judged, Nameable, ParamSet, PlacedRow, PlacingSpec, Plan, SpannedFinding,
};
use crate::domain::comments::service::ignoring;
use crate::domain::contracts::types::Contract;
use crate::domain::definitions::types::{Exactness, LoadedWorkspace, RuleFile};
use crate::domain::entries::service::identity;
use crate::domain::entries::types::{ParamDeclarations, ParamName, ParamValue, RuleEntry, Value};
use crate::domain::files::types::SourceView;
use crate::domain::findings::constants::PARAMS;
use crate::domain::findings::service::{check_id, error, indexed_span};
use crate::domain::findings::types::{Finding, Fingerprint, Place};
use crate::domain::packages::service::release_id;
use crate::domain::packages::types::{DefinitionKind, DefinitionName, Release};
use crate::domain::paths::service::relative;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::service::{composition_of, labels};
use crate::domain::scopes::types::{Origin, Scope};
use crate::domain::selectors::types::{PartInstance, PlacedPath};

/// The rule an entry names, loaded.
///
/// # Errors
///
/// `Unloaded` when the entry names a rule that was not loaded.
pub fn loaded<'w>(
    workspace: &'w LoadedWorkspace,
    entry: &RuleEntry,
    release: &Release,
) -> Result<&'w RuleFile, PlacementError> {
    workspace
        .rules
        .get(release)
        .ok_or_else(|| PlacementError::Unloaded {
            entry: identity(&entry.header, DefinitionKind::Rule, &entry.rule).0,
            release: release_id(release).0,
        })
}

/// The instances and placed files a rule entry's selectors can name.
#[must_use]
pub fn known(workspace: &LoadedWorkspace, placement: &Placement) -> Nameable {
    let instances = placement
        .scopes
        .iter()
        .filter_map(|scope| match &scope.composition {
            crate::domain::scopes::types::CompositionRef::Spec(release) => Some(PartInstance {
                directory: scope.dir.clone(),
                spec: release.name.clone(),
            }),
            crate::domain::scopes::types::CompositionRef::Root
            | crate::domain::scopes::types::CompositionRef::Local(_) => None,
        })
        .collect();
    let placed = placement
        .placed
        .iter()
        .filter_map(|(path, file)| {
            let scope = placement.scopes.get(file.scope.0)?;
            let entry = composition_of(workspace, &scope.composition)
                .templates
                .get(file.entry.0)?;
            Some(PlacedPath {
                path: path.clone(),
                entry: crate::domain::entries::service::reference(&entry.header, &entry.template)
                    .clone(),
            })
        })
        .collect();
    Nameable { instances, placed }
}

/// The values a rule entry passes, its scope's params put in; none after reporting a param the rule does not declare,
/// or one it declares and is not given.
pub fn rule_params(
    findings: &mut Vec<Finding>,
    scope: &Scope,
    entry: &RuleEntry,
    declared: &ParamDeclarations,
) -> Option<ParamSet> {
    let entry_identity = identity(&entry.header, DefinitionKind::Rule, &entry.rule);
    let rule = check_id(&entry_identity, PARAMS);
    let mut values = BTreeMap::new();
    let mut complete = true;
    for (name, value) in &entry.header.params.0 {
        if !declared.0.contains_key(name) {
            findings.push(error(
                rule.clone(),
                scope.anchor.clone(),
                format!(
                    "`{entry_identity}` passes `{}`, which its rule does not declare",
                    name.0
                ),
            ));
            complete = false;
            continue;
        }
        match value {
            ParamValue::Literal(value) => {
                drop(values.insert(name.clone(), value.clone()));
            }
            ParamValue::Param(reference) => {
                if let Some(given) = scope.typed.get(reference) {
                    drop(values.insert(name.clone(), given.clone()));
                } else {
                    findings.push(error(
                        rule.clone(),
                        scope.anchor.clone(),
                        format!(
                            "`{entry_identity}` passes `{}`, which `{}` does not give",
                            reference.0, scope.identity.0
                        ),
                    ));
                    complete = false;
                }
            }
            ParamValue::Variable(_) => complete = false,
        }
    }
    for name in declared.0.keys() {
        if !values.contains_key(name) && complete {
            findings.push(error(
                rule.clone(),
                scope.anchor.clone(),
                format!(
                    "`{entry_identity}` gives no value for its rule's param `{}`",
                    name.0
                ),
            ));
            complete = false;
        }
    }
    complete.then_some(ParamSet(values))
}

/// A rule's params as the lint call binds them: the entry's values, and `$exceptions` when the rule has exceptions.
#[must_use]
pub fn parameters(rule: &RuleFile, params: ParamSet) -> BTreeMap<ParamName, Value> {
    let mut bound = params.0;
    if !rule.exceptions.is_empty() {
        drop(
            bound.insert(
                ParamName(EXCEPTIONS.into()),
                Value::List(
                    rule.exceptions
                        .values()
                        .flat_map(|exception| exception.admits.iter().cloned())
                        .collect(),
                ),
            ),
        );
    }
    bound
}

/// The rows of `tastemaker.placed`: one for each placed file, relative to the analysis root.
#[must_use]
pub fn placed_rows(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    base: &RepoPath,
) -> Vec<PlacedRow> {
    placement
        .placed
        .iter()
        .filter_map(|(path, file)| {
            let scope = placement.scopes.get(file.scope.0)?;
            let entry = composition_of(workspace, &scope.composition)
                .templates
                .get(file.entry.0)?;
            let spec = match scope.origin {
                Origin::File => PlacingSpec::Workspace,
                Origin::Instance { .. } => PlacingSpec::Instance(DefinitionName(
                    labels(workspace, &placement.scopes, scope).spec.0.into(),
                )),
            };
            Some(PlacedRow {
                path: RepoPath(relative(path, base)?.0.into()),
                entry: crate::domain::entries::service::reference(&entry.header, &entry.template)
                    .clone(),
                package: entry.header.package.clone(),
                template: entry.template.clone(),
                version: entry.header.version.clone(),
                spec,
            })
        })
        .collect()
}

/// Locate one file's findings from its resident source view.
#[must_use]
pub fn located(spans: Vec<SpannedFinding>, source: Option<SourceView<'_>>) -> Vec<Finding> {
    spans
        .into_iter()
        .map(|SpannedFinding { finding, span }| Finding {
            place: source
                .and_then(|view| indexed_span(view, span.start, span.end))
                .map_or(Place::Whole, Place::At),
            ..finding
        })
        .collect()
}

/// Every source read by the reporting stages: the files of findings placed in them, of comments that may hold an ignore,
/// and of contracts whose comparison reads their text. An import is spelled from the rows, so no file is read for one; a
/// file whose comments hold no ignore, or whose contract is compared by its rows alone, is not read either. Nor is a file in `skip`,
/// which has what it had when it was last settled and is as it was, unless a contract is compared by its text.
#[must_use]
pub fn source_paths(
    plan: &Plan,
    judged: &Judged,
    findings: &[Finding],
    skip: &BTreeSet<RepoPath>,
) -> Vec<RepoPath> {
    judged
        .spans
        .iter()
        .map(|span| span.finding.path.clone())
        .chain(
            findings
                .iter()
                .filter(|finding| finding.place != Place::Whole)
                .map(|finding| finding.path.clone()),
        )
        .filter(|path| !skip.contains(path))
        .chain(
            ignoring(&judged.comments)
                .filter(|path| !skip.contains(*path))
                .cloned(),
        )
        .chain(compared_paths(plan, judged))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The files a report need not settle again: each has before its ignores the findings it had when it was last settled, and no
/// template places it, whose comparison with its content may have changed whatever the file holds. A report that answers for a part
/// of the workspace settles every file again.
#[must_use]
pub fn reusable_paths(
    plan: &Plan,
    judged: &Judged,
    incoming: &BTreeMap<RepoPath, Fingerprint>,
    was: &BTreeMap<RepoPath, Fingerprint>,
    within: &[RepoPath],
) -> BTreeSet<RepoPath> {
    let compared = compared_paths(plan, judged);
    let templated: BTreeSet<&RepoPath> = plan
        .contracts
        .iter()
        .map(|contract| &contract.target)
        .collect();
    incoming
        .iter()
        .filter(|(path, now)| {
            within.is_empty()
                && !compared.contains(*path)
                && !templated.contains(*path)
                && was.get(*path) == Some(*now)
        })
        .map(|(path, _)| path.clone())
        .collect()
}

/// The files whose text comparing a contract reads: they are read whatever else is known of them.
#[must_use]
fn compared_paths(plan: &Plan, judged: &Judged) -> BTreeSet<RepoPath> {
    let compared = Compared {
        data: &judged.data,
        unread: &judged.unread,
        dropped: &judged.dropped,
    };
    plan.contracts
        .iter()
        .filter_map(|contract| compared_text(contract, &compared).cloned())
        .collect()
}

/// How `contract` is compared when both its files answered the rows the comparison reads; none when either went unread,
/// lost a row, or the content answered no relation the comparison reads.
pub(crate) fn readable_shape(
    contract: &Contract,
    Compared {
        unread, dropped, ..
    }: &Compared<'_>,
    expected: &crate::domain::contracts::types::Facts<'_>,
) -> Option<crate::domain::contracts::types::Shape> {
    use crate::domain::contracts::service::shape;
    use crate::domain::contracts::types::Shape;
    let read = |path: &RepoPath| !unread.contains(path) && !dropped.contains(path);
    let shape = shape(expected, contract.exact);
    (read(&contract.target) && read(&contract.rendered) && shape != Shape::Blind).then_some(shape)
}

/// The file whose text comparing `contract` reads: its target, when the comparison is verbatim, in order as a document,
/// or by the lines of a file it cannot read by its rows; none when its rows alone compare it, so the file is not read.
fn compared_text<'a>(contract: &'a Contract, compared: &Compared<'_>) -> Option<&'a RepoPath> {
    use crate::domain::contracts::service::facts;
    use crate::domain::contracts::types::Shape;
    let rendered = compared
        .data
        .get(&contract.rendered)
        .map_or(&[][..], Vec::as_slice);
    let by_rows = contract.exact != Exactness::Verbatim
        && matches!(
            readable_shape(contract, compared, &facts(rendered)),
            Some(Shape::Code | Shape::Data)
        );
    (!by_rows).then_some(&contract.target)
}
