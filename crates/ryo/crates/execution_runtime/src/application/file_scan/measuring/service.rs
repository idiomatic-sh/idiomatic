//! The service of a measuring run's report: the rule set's work ledger, one line a unit, as the harness prints it. Nothing
//! here measures: the engine's measuring state counts, and its ledger is read through the port.

use super::super::types::{LedgerLine, ScanWorker};
use super::constants::{
    GRAPH_FILL, GROUP_FILL, LINE, NAVIGATION, ORDERED_FILL, OWNERS_BETWEEN, PATTERN, ROWS_FILL,
    RULE, TABLE,
};
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::counting::types::{LedgerUnit, WorkClass};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;

/// The work ledger of a rule set after a measuring run's files, one line a unit: what the states of `workers` counted, added
/// up and read by the engine. A line names the unit (what it is and its number), its work class, the rows it gave, the steps
/// it took with the rows its stages read and gave, its time and the part of it in files where it gave no row, in
/// nanoseconds, and the rules that reach it. Workers whose state does not measure counted nothing.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_execution_runtime::application::file_scan::measuring::service::ledger_lines;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// assert!(ledger_lines(&engine, &Programs::default(), &[]).is_empty());
/// ```
#[must_use]
pub fn ledger_lines(
    engine: &dyn EnginePort,
    programs: &Programs,
    workers: &[&ScanWorker],
) -> Vec<LedgerLine> {
    let states: Vec<&ScanScratch> = workers.iter().map(|worker| &worker.scan).collect();
    let ledger = engine.ledger(programs, &states);
    let lines = ledger.0.iter().map(|row| {
        let (unit, number) = match row.unit {
            LedgerUnit::Rule(rule) => (RULE, u64::from(rule.0)),
            LedgerUnit::Pattern(place) => (PATTERN, u64::try_from(place.0).unwrap_or(u64::MAX)),
            LedgerUnit::Table(place) => (TABLE, u64::try_from(place.0).unwrap_or(u64::MAX)),
        };
        let class = match row.class {
            WorkClass::Navigation => NAVIGATION,
            WorkClass::RowsFill => ROWS_FILL,
            WorkClass::GroupFill => GROUP_FILL,
            WorkClass::OrderedFill => ORDERED_FILL,
            WorkClass::GraphFill => GRAPH_FILL,
        };
        let owners: Vec<String> = row.owners.iter().map(|rule| rule.0.to_string()).collect();
        let values = [
            unit.to_owned(),
            number.to_string(),
            class.to_owned(),
            row.rows.0.to_string(),
            row.steps.0.to_string(),
            row.nanos.0.to_string(),
            row.idle_nanos.0.to_string(),
            owners.join(OWNERS_BETWEEN),
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
