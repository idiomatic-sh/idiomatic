//! The service of a measuring run's report: the rule set's work ledger, one line a unit, as the harness prints it. Nothing
//! here measures: the engine's measuring state counts, and its ledger is read through the port; the load's record of where
//! each step was written names the units.

use super::super::types::{LedgerLine, LedgerValue, ScanWorker};
use super::constants::{
    CALLED_FROM, GRAPH_FILL, GROUP_FILL, IN_DOCUMENT, LINE, NAVIGATION, ORDERED_FILL,
    OWNERS_BETWEEN, PATTERN, PROVIDER_FILL, RELATION, RENDER, RENDERED, ROWS_FILL, RULE, RULES_AT,
    SITES_BETWEEN, TABLE, UNWRITTEN,
};
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::counting::types::{LedgerRow, LedgerUnit, ProviderWork, WorkClass};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_relations::domain::relations::service::name;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::written_sites::types::{RulesWritten, StepWritten};

/// The work ledger of a rule set after a measuring run's files, one line a unit: what the states of `workers` counted, added
/// up and read by the engine. A line names the unit (what it is and its number), its work class, the rows it gave, the steps
/// it took with the rows its stages read and gave, its time and the part of it in files where it gave no row, in
/// nanoseconds, and the rules that reach it; then the part of its time spent deciding whether the file can hold a row, the
/// language's time it asked for, the rules that reach it by id, and the statements it was written as (`written`, the load's
/// record, names them). The language's own work is a line of its own for each relation it filled and for its rendering.
/// Workers whose state does not measure counted nothing.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::measuring::service::ledger_lines;
/// use ryo_values::domain::written_sites::types::RulesWritten;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// assert!(ledger_lines(&engine, &Programs::default(), &RulesWritten::default(), &[]).is_empty());
/// ```
#[must_use]
pub fn ledger_lines(
    engine: &dyn EnginePort,
    programs: &Programs,
    written: &RulesWritten,
    workers: &[&ScanWorker],
) -> Vec<LedgerLine> {
    let states: Vec<&ScanScratch> = workers.iter().map(|worker| &worker.scan).collect();
    let ledger = engine.ledger(programs, &states);
    let lines = ledger.0.iter().map(|row| {
        let (unit, number) = match row.unit {
            LedgerUnit::Rule(rule) => (RULE, rule.0.to_string()),
            LedgerUnit::Pattern(place) => (PATTERN, place.0.to_string()),
            LedgerUnit::Table(place) => (TABLE, place.0.to_string()),
            LedgerUnit::Provider(ProviderWork::Relation(relation)) => {
                (RELATION, name(relation).0.to_owned())
            }
            LedgerUnit::Provider(ProviderWork::Render) => (RENDER, RENDERED.to_owned()),
        };
        let class = match row.class {
            WorkClass::Navigation => NAVIGATION,
            WorkClass::RowsFill => ROWS_FILL,
            WorkClass::GroupFill => GROUP_FILL,
            WorkClass::OrderedFill => ORDERED_FILL,
            WorkClass::GraphFill => GRAPH_FILL,
            WorkClass::ProviderFill => PROVIDER_FILL,
        };
        let owners: Vec<String> = row.owners.iter().map(|rule| rule.0.to_string()).collect();
        let ids: Vec<String> = row
            .owners
            .iter()
            .map(|rule| rule_named(written, *rule).0)
            .collect();
        let values = [
            unit.to_owned(),
            number,
            class.to_owned(),
            row.rows.0.to_string(),
            row.steps.0.to_string(),
            row.nanos.0.to_string(),
            row.idle_nanos.0.to_string(),
            owners.join(OWNERS_BETWEEN),
            row.decide_nanos.0.to_string(),
            row.provider_nanos.0.to_string(),
            ids.join(OWNERS_BETWEEN),
            sites(written, row).0,
        ];
        // The line's names, each followed by its value.
        let written: Vec<String> = LINE
            .iter()
            .zip(values)
            .map(|(named, value)| [*named, &value].concat())
            .collect();
        LedgerLine(written.concat().into())
    });
    lines.collect()
}

/// The id of the rule at `rule`, as the load recorded it: its ordinal when the load recorded none.
fn rule_named(written: &RulesWritten, rule: SelectedRuleOrdinal) -> LedgerValue {
    let held = written.0.get(usize::try_from(rule.0).unwrap_or(usize::MAX));
    LedgerValue(held.map_or_else(|| rule.0.to_string(), |held| held.id.0.to_string()))
}

/// The statements a row's unit was written as, each once, in the order its owners first reach it, with how many owners reach
/// it there when more than one does (an owner whose plan holds no step of the unit is left out, but when no owner's does): a statement of a module as the module and its `let` name, then each document that
/// reaches it, innermost first, short of the rule's own query; a statement of a rule's own query as the rule and its name.
fn sites(written: &RulesWritten, row: &LedgerRow) -> LedgerValue {
    let mut seen: Vec<(LedgerValue, Count)> = Vec::new();
    let found = row.owners.iter().zip(&row.owner_steps);
    let found: Vec<LedgerValue> = found
        .map(|(rule, step)| site_of(written, *rule, *step))
        .collect();
    // An owner that reaches the unit through another's plan (it binds that rule's observation) names no statement: it is
    // left out when another owner names one.
    let named = found.iter().any(|site| site.0 != UNWRITTEN);
    for site in found
        .into_iter()
        .filter(|site| !named || site.0 != UNWRITTEN)
    {
        match seen.iter_mut().find(|(held, _)| *held == site) {
            Some((_, count)) => *count = Count(count.0.saturating_add(1)),
            None => seen.push((site, Count(1))),
        }
    }
    let shown: Vec<String> = seen
        .into_iter()
        .map(|(site, count)| match count.0 {
            1 => site.0,
            _ => [site.0, RULES_AT.to_owned(), count.0.to_string()].concat(),
        })
        .collect();
    LedgerValue(shown.join(SITES_BETWEEN))
}

/// Where the step `step` of the rule at `rule` was written, as one site.
fn site_of(written: &RulesWritten, rule: SelectedRuleOrdinal, step: Option<StepId>) -> LedgerValue {
    let held = written.0.get(usize::try_from(rule.0).unwrap_or(usize::MAX));
    let found =
        held.zip(step)
            .and_then(|(held, step)| match held.steps.get(usize::from(step))? {
                StepWritten::Written(site) => Some((held, &**site)),
                StepWritten::Unrecorded => None,
            });
    let Some((held, site)) = found else {
        return LedgerValue(UNWRITTEN.to_owned());
    };
    let own = site.document == held.root;
    let document = if own { &*held.id.0 } else { &*site.document.0 };
    let mut shown = [document, IN_DOCUMENT, &site.statement].concat();
    // The documents that reach it, innermost first: each call site's source once, the rule's own query left out.
    let mut callers: Vec<&str> = Vec::new();
    for call in site.trace.iter().rev() {
        let caller = &*call.source_id.0;
        if call.source_id != held.root && callers.last() != Some(&caller) && caller != document {
            callers.push(caller);
        }
    }
    for caller in callers {
        shown.push_str(CALLED_FROM);
        shown.push_str(caller);
    }
    LedgerValue(shown)
}
