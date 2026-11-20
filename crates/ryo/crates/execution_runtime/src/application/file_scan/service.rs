//! The `file_scan` operation's harness (scope-engine2 2.9, `RYO_ENGINE=old|new|both`, for the harness only): the new engine
//! loads every selected rule, the rules it refuses are counted by why, and `both` compares the rules it accepts, per rule and
//! file, failing on a difference. Track H adds the file scan itself (walk, scan, rows to the store and the report).

use super::types::{HarnessDifference, HarnessLoad, HarnessRows, RuleFile};
use ryo_engine::domain::lowering::types::{SelectedRule, SelectedRules};
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::interface::LoadUseCase as _;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::spike::{
    EngineMode, engine_compared, engine_difference, engine_loaded, engine_mode, engine_refusal,
};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::sync::Arc;

/// The kind vocabulary the harness lowers against. Track H passes the linked language's (`kind_names`) when it adds the file scan;
/// until a rule is accepted, the lowering reads none.
fn harness_kinds() -> KindVocabulary {
    KindVocabulary {
        language: LanguageName::default(),
        kinds: Vec::new(),
    }
}

/// Loads each selected rule alone, counting the refused ones by why, then the accepted ones together.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::loaded;
///
/// assert!(loaded(&[]).accepted.is_empty());
/// ```
#[must_use]
pub fn loaded(plans: &[Arc<Plan>]) -> HarnessLoad {
    let kinds = harness_kinds();
    let mut loaded = HarnessLoad::default();
    let mut accepted: Vec<SelectedRule<'_>> = Vec::new();
    for (place, plan) in plans.iter().enumerate() {
        let rule = SelectedRule {
            rule: SelectedRuleOrdinal(u32::try_from(place).unwrap_or(u32::MAX)),
            plan,
        };
        match LoadUseCaseImpl.load(LoadSource::Plans(SelectedRules(&[rule])), &kinds) {
            Ok(_) => accepted.push(rule),
            Err(LoadError::Budget { .. }) => {
                loaded.over_budget = Count(loaded.over_budget.0.saturating_add(1))
            }
            Err(_) => loaded.unlowerable = Count(loaded.unlowerable.0.saturating_add(1)),
        }
    }
    loaded.accepted = accepted.iter().map(|rule| rule.rule).collect();
    if let Ok(programs) = LoadUseCaseImpl.load(LoadSource::Plans(SelectedRules(&accepted)), &kinds)
    {
        loaded.programs = programs;
    }
    loaded
}

/// Compares the two engines' rows of the accepted rules, per rule and file; answers how many rules were compared.
///
/// # Errors
///
/// The first rule and file whose rows differ.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::compared;
/// use ryo_execution_runtime::application::file_scan::types::HarnessRows;
/// use ryo_relations::domain::relations::types::Count;
///
/// assert_eq!(compared(&HarnessRows::default(), &HarnessRows::default(), &[]), Ok(Count(0)));
/// ```
pub fn compared(
    old: &HarnessRows,
    new: &HarnessRows,
    accepted: &[SelectedRuleOrdinal],
) -> Result<Count, HarnessDifference> {
    let places: std::collections::BTreeSet<RuleFile> = old
        .0
        .keys()
        .chain(new.0.keys())
        .filter(|place| accepted.contains(&place.rule))
        .copied()
        .collect();
    for at in places {
        let (before, after) = (
            old.0.get(&at).cloned().unwrap_or_default(),
            new.0.get(&at).cloned().unwrap_or_default(),
        );
        if before != after {
            return Err(HarnessDifference {
                at,
                old: before,
                new: after,
            });
        }
    }
    Ok(Count(accepted.len()))
}

/// The harness of a request's file phase, before any file: nothing for the old engine; for `new` and `both`, the new engine's
/// `load` of every selected rule, reported by count, and `both`'s comparison of the accepted rules.
///
/// # Errors
///
/// `EngineHarness` when `both` finds a difference, and when the new engine would have to run files it cannot run alone yet.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::service::harnessed;
///
/// assert!(harnessed(&[]).is_ok());
/// ```
pub fn harnessed(plans: &[Arc<Plan>]) -> Result<(), OperatingError> {
    let mode = engine_mode();
    if mode == EngineMode::Old {
        return Ok(());
    }
    let loaded = loaded(plans);
    engine_loaded(
        mode,
        loaded.accepted.len(),
        plans.len(),
        loaded.unlowerable.0,
        loaded.over_budget.0,
    );
    if mode == EngineMode::New || !loaded.accepted.is_empty() {
        return Err(OperatingError::EngineHarness {
            reason: engine_refusal(mode, loaded.accepted.len()),
        });
    }
    let compared = compared(
        &HarnessRows::default(),
        &HarnessRows::default(),
        &loaded.accepted,
    )
    .map_err(|difference| OperatingError::EngineHarness {
        reason: engine_difference(&difference),
    })?;
    engine_compared(compared.0);
    Ok(())
}
