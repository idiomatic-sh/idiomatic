//! The `planning` service: free functions named by verb that turn a placement into the lint calls of a check, with no IO and no
//! ports. Each rule entry of each scope is applied with its params resolved (`applications`); the releases whose
//! applications are apart are handed whole (`carrying`), the others matched here (`applying`); the built-in rules are
//! `built_ins`', and the answer is read back by `answering`.

use crate::domain::applications::constants::{EVALUATED_CHECKS, PLACING_SEPARATOR};
use crate::domain::applications::service::{known, loaded, parameters, placed_rows, rule_params};
use crate::domain::applications::types::{
    Carried, Compilers, LintRuleId, Plan, PlannedRule, Purpose, Rendering,
};
use crate::domain::applying::service::{handed_reach, matched_rules};
use crate::domain::built_ins::service::{comments, conditions, imports};
use crate::domain::carrying::service::carried;
use crate::domain::carrying::types::CarriedSplit;
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, rule_release};
use crate::domain::entries::types::RuleEntry;
use crate::domain::findings::constants::PARAMS;
use crate::domain::findings::service::check_id;
use crate::domain::findings::types::RuleId;
use crate::domain::globs::service::counted;
use crate::domain::packages::service::release_id;
use crate::domain::packages::types::DefinitionKind;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::placement::types::Placement;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::ScopeIndex;

/// Every rule entry of every scope as it applies there with its params; what each entry evaluates is noted in
/// `plan` once, an entry of a spec being applied at every instance.
///
/// # Errors
///
/// `Unloaded` when an entry names a rule that was not loaded.
fn applications<'w>(
    workspace: &'w LoadedWorkspace,
    placement: &Placement,
    plan: &mut Plan,
) -> Result<Vec<Carried<'w>>, PlacementError> {
    let mut applications = Vec::new();
    let mut noted: std::collections::HashSet<*const RuleEntry> = std::collections::HashSet::new();
    for (index, scope) in placement.scopes.iter().enumerate() {
        let composition = composition_of(workspace, &scope.composition);
        for entry in &composition.rules {
            let release = rule_release(entry);
            let rule = loaded(workspace, entry, &release)?;
            if noted.insert(std::ptr::from_ref(entry)) {
                let id = release_id(&release);
                for check in EVALUATED_CHECKS {
                    let _: bool = plan.evaluated.insert(check_id(&id.0, check));
                }
                let entry_identity = identity(&entry.header, DefinitionKind::Rule, &entry.rule);
                let _: bool = plan.evaluated.insert(check_id(&entry_identity, PARAMS));
            }
            let Some(params) = rule_params(&mut plan.findings, scope, entry, &rule.params) else {
                continue;
            };
            applications.push(Carried {
                scope: ScopeIndex(index),
                entry,
                release,
                params,
            });
        }
    }
    Ok(applications)
}

/// The plan of `workspace` placed as `placement`: the rules, the conditions and the built-in comments rule of the one
/// lint call, rooted at `base`, with what placement found and every check that ran.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile, `Unloaded` when an entry names a rule that was not loaded.
pub fn plan(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    base: RepoPath,
    matching: Compilers<'_>,
) -> Result<Plan, PlacementError> {
    let mut plan = Plan {
        workspace: workspace.section.name.clone(),
        base,
        findings: placement.findings.clone(),
        counts: placement.counts.clone(),
        pairs: placement.pairs.clone(),
        evaluated: placement.evaluated.clone(),
        excluded: placement.excluded.clone(),
        closings: placement.closings.clone(),
        files: workspace.files.clone(),
        anchors: placement
            .scopes
            .iter()
            .map(|scope| scope.anchor.clone())
            .collect(),
        known: workspace
            .rules
            .keys()
            .map(|release| RuleId(release_id(release).0.into()))
            .collect(),
        ..Plan::default()
    };
    let known = known(workspace, placement);
    let applications = applications(workspace, placement, &mut plan)?;
    let CarriedSplit { handed, matched } = carried(
        placement,
        &applications,
        &known,
        matching.compile,
        &plan.base,
    )?;
    handed_reach(
        workspace,
        placement,
        &applications,
        &handed,
        &known,
        matching,
        &mut plan,
    )?;
    let matched = matched_rules(workspace, placement, &matched, &known, matching, &mut plan)?;
    let planned = handed.into_iter().chain(matched).collect::<Vec<_>>();
    for rule in &planned {
        let Some(loaded) = workspace.rules.get(&rule.release) else {
            let release = release_id(&rule.release).0;
            return Err(PlacementError::Unloaded {
                entry: release.clone(),
                release,
            });
        };
        // A rule is the lint call's by its definition's own id; a definition placed with other params is told apart by what
        // it is placed with, never by where in the plan it falls.
        let release = release_id(&rule.release).0;
        let placings = planned
            .iter()
            .filter(|other| other.release == rule.release)
            .count();
        let id = if placings == 1 {
            LintRuleId(release.into())
        } else {
            let mut hasher = std::hash::DefaultHasher::new();
            std::hash::Hash::hash(&format!("{:?}", rule.params), &mut hasher);
            LintRuleId(
                format!(
                    "{release}{PLACING_SEPARATOR}{:016x}",
                    std::hash::Hasher::finish(&hasher)
                )
                .into(),
            )
        };
        let rule = rule.clone();
        plan.rules.push(PlannedRule {
            id: id.clone(),
            query: loaded.query.clone(),
            parameters: parameters(loaded, rule.params),
            include: rule.include,
            exclude: rule.exclude,
        });
        drop(plan.purposes.insert(
            id,
            Purpose::Rule(Rendering {
                rule: RuleId(release_id(&rule.release).0.into()),
                diagnostic: loaded.diagnostic.clone(),
            }),
        ));
    }
    let governed = counted(&workspace.files);
    conditions(
        workspace,
        placement,
        &mut plan,
        &known,
        matching.compile,
        &governed,
    )?;
    comments(&mut plan);
    imports(&mut plan, workspace, placement);
    plan.placed = placed_rows(workspace, placement, &plan.base);
    Ok(plan)
}
