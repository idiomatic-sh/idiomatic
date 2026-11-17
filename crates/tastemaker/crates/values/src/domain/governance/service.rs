//! The `governance` service: free functions named by verb over the files each placed rule governs.

use std::collections::{BTreeMap, HashMap};

use super::types::{Answers, Decision, Governance, Governing, Inclusion};
use crate::domain::applications::types::{GlobText, LintRuleId, Plan, PlannedRule, Purpose};
use crate::domain::paths::service::relative;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::{PathRest, PatternFailure, SetCompiler, SetGlob};

/// The rule's `globs` compiled as one set, or why one does not compile.
fn compiled(
    sets: &dyn SetCompiler,
    globs: &[GlobText],
) -> Result<std::sync::Arc<dyn crate::domain::selectors::types::PathSet>, PatternFailure> {
    sets.compile_set(
        &globs
            .iter()
            .map(|glob| SetGlob(glob.0.clone()))
            .collect::<Vec<_>>(),
    )
}

/// The rule's `include` and `exclude` compiled, or why one does not compile.
fn governing(sets: &dyn SetCompiler, rule: &PlannedRule) -> Result<Governing, PatternFailure> {
    let include = if rule.include.is_empty() {
        Inclusion::Every
    } else {
        Inclusion::Only(compiled(sets, &rule.include)?)
    };
    Ok(Governing {
        include,
        exclude: compiled(sets, &rule.exclude)?,
    })
}

/// Whether `test` lets the rule read `rest`, a path relative to the lint call's root.
fn reads(test: &Governing, rest: PathRest<'_>) -> Governance {
    let mut passing = Vec::new();
    let included = match &test.include {
        Inclusion::Every => true,
        Inclusion::Only(include) => {
            include.passing(rest, &mut passing);
            !passing.is_empty()
        }
    };
    test.exclude.passing(rest, &mut passing);
    if included && passing.is_empty() {
        Governance::Governs
    } else {
        Governance::Idle
    }
}

/// The answers of each placed rule in the files it governs: a rule reads other files, and a row about one of those is
/// not the rule's finding there. A rule that is not placed, or whose globs do not compile, keeps every answer.
#[must_use]
pub fn governed(
    plan: &Plan,
    Answers {
        answers,
        unknowns,
        unread,
    }: Answers,
    sets: &dyn SetCompiler,
) -> Answers {
    let rules = plan
        .rules
        .iter()
        .map(|rule| (&rule.id, rule))
        .collect::<BTreeMap<_, _>>();
    let mut tests: BTreeMap<&LintRuleId, Result<Governing, PatternFailure>> = BTreeMap::new();
    // A rule's findings fan out over the files they name: each rule and file is decided once.
    let mut decided: HashMap<Decision, Governance> = HashMap::new();
    let mut governs = |rule: &LintRuleId, path: &RepoPath| {
        let decision = Decision {
            rule: rule.clone(),
            path: path.clone(),
        };
        if let Some(known) = decided.get(&decision) {
            return *known;
        }
        let verdict = match (plan.purposes.get(rule), rules.get(rule)) {
            (Some(Purpose::Rule(_)), Some(planned)) => tests
                .entry(&planned.id)
                .or_insert_with(|| governing(sets, planned))
                .as_ref()
                .map_or(Governance::Governs, |test| {
                    let rest = relative(path, &plan.base).map_or(&*path.0, |rest| rest.0);
                    reads(test, PathRest(rest))
                }),
            _ => Governance::Governs,
        };
        let _: Option<Governance> = decided.insert(decision, verdict);
        verdict
    };
    let answers = answers
        .into_iter()
        .filter(|answer| governs(&answer.rule, &answer.path) == Governance::Governs)
        .collect();
    let unknowns = unknowns
        .into_iter()
        .filter(|unknown| governs(&unknown.rule, &unknown.path) == Governance::Governs)
        .collect();
    Answers {
        answers,
        unknowns,
        unread,
    }
}

/// Whether the rule `id` of `plan` governs any of the files `corpus` lists: it is idle when its `include` passes none
/// that its `exclude` does not stop. A rule with no `include`, or whose globs do not compile, governs.
#[must_use]
pub fn governance(
    plan: &Plan,
    id: &LintRuleId,
    corpus: &[RepoPath],
    sets: &dyn SetCompiler,
) -> Governance {
    let Some(planned) = plan.rules.iter().find(|rule| rule.id == *id) else {
        return Governance::Governs;
    };
    if planned.include.is_empty() {
        return Governance::Governs;
    }
    // A rule whose globs do not compile governs: the placement says why they do not.
    let Ok(test) = governing(sets, planned) else {
        return Governance::Governs;
    };
    if corpus
        .iter()
        .any(|path| reads(&test, PathRest(&path.0)) == Governance::Governs)
    {
        Governance::Governs
    } else {
        Governance::Idle
    }
}
