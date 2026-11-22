//! The services of the `counting` concept: what a measuring worker counted, the counts of several workers added up, and the
//! work ledger read off them. The counts are taken by a measuring worker's loop (`executing::measuring`).

use super::types::{LedgerRow, LedgerUnit, WorkClass, WorkLedger};
use crate::domain::code::types::{ProgramCode, UnitKey, WordSlot};
use crate::domain::executing::running::service::units_reached;
use crate::domain::made_tables::fills::service::fill_at;
use crate::domain::made_tables::fills::types::FillKind;
use crate::domain::programs::service::step_of_unit;
use crate::domain::programs::types::Programs;
use crate::domain::scratch::types::ScanScratch;
use crate::domain::statistics::types::{PointCounts, PointCountsEngine, UnitCounts};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::relations::types::Count;

/// What a worker counted since its scratch was made: the runs of each plan point, what each unit was asked and gave, and the
/// sizes of the files; nothing for a scratch that does not measure.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::counted;
/// use ryo_engine::domain::programs::service::scratch_measuring;
/// use ryo_engine::domain::programs::types::Programs;
///
/// let _counts = counted(&scratch_measuring(&Programs::default()));
/// ```
#[must_use]
pub fn counted(scratch: &ScanScratch) -> PointCounts {
    let Some(measure) = scratch.0.state.measure.as_deref() else {
        return PointCounts::default();
    };
    // SPIKE (never a contract): what the fills that needs started cost, a line a table, on stderr; `build/x/scripts/
    // need-fills.py` sums the workers' lines. The unit is the table's, as the ledger's lines name it.
    for (table, fill) in measure.need_fills.by_table.iter().enumerate() {
        if fill.files.0 > 0 {
            let unit = measure
                .table_units
                .get(table)
                .map_or(u32::MAX, |unit| unit.0);
            eprintln!(
                "ryo x: need-fill table={table} unit={unit} files={} empty={} idle={} nanos={}",
                fill.files.0, fill.empty.0, fill.idle.0, fill.nanos.0
            );
        }
    }
    // SPIKE: the language's fills, a line a relation, when the run counts them (`RYO_X_FILLS`).
    for (relation, fills) in ryo_language::domain::fillers::service::fill_counts_taken() {
        eprintln!(
            "ryo x: fact-fill relation={relation:?} whole={} keyed={} rows={} nanos={} unread={} unread_rows={} unread_nanos={}",
            fills.whole,
            fills.keyed,
            fills.rows,
            fills.nanos,
            fills.unread,
            fills.unread_rows,
            fills.unread_nanos
        );
    }
    // SPIKE: what the decisions visited.
    let visits = &crate::domain::executing::sources::types::NEED_VISITS;
    let taken = |at: usize| {
        visits.get(at).map_or(0, |count| {
            count.swap(0, std::sync::atomic::Ordering::Relaxed)
        })
    };
    eprintln!(
        "ryo x: need-visits decisions={} free={} asked={} musts-failed={} rule-files-run={}",
        taken(0),
        taken(1),
        taken(2),
        taken(3),
        taken(4)
    );
    for (site, formula) in &measure.need_fills.formulas {
        eprintln!("ryo x: need-formula site={site} {formula}");
    }
    for (trace, files) in &measure.need_fills.traces {
        eprintln!("ryo x: need-trace files={files} {trace}");
    }
    // A worker keeps a column per count, each as long as the programs have units; the statistics hold a row per unit.
    let columns = measure.calls.iter().zip(&measure.keys).zip(&measure.rows);
    let columns = columns.zip(&measure.build).zip(&measure.nanos);
    let units = columns.zip(&measure.idle_nanos).enumerate().map(
        |(place, (((((calls, keys), rows), build), nanos), idle_nanos))| UnitCounts {
            calls: *calls,
            keys: *keys,
            rows: *rows,
            build: *build,
            nanos: *nanos,
            idle_nanos: *idle_nanos,
            decide_nanos: measure.decide.get(place).copied().unwrap_or_default(),
            provider_nanos: measure.provider.get(place).copied().unwrap_or_default(),
        },
    );
    PointCounts(PointCountsEngine {
        files: measure.files,
        points: measure.points.clone(),
        units: units.collect(),
        kinds: measure.kinds.clone(),
        fields: measure.fields.clone(),
        elements: measure.elements,
        literals: measure.literals.clone(),
        providers: measure.providers.clone(),
    })
}

/// Adds one worker's counts to another's.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::merged;
/// use ryo_engine::domain::statistics::types::PointCounts;
///
/// let mut all = PointCounts::default();
/// merged(&mut all, &PointCounts::default());
/// ```
pub fn merged(into: &mut PointCounts, from: &PointCounts) {
    let (all, one) = (&mut into.0, &from.0);
    all.files = Count(all.files.0.saturating_add(one.files.0));
    added(&mut all.points, &one.points);
    added(&mut all.kinds, &one.kinds);
    added(&mut all.fields, &one.fields);
    all.elements = Count(all.elements.0.saturating_add(one.elements.0));
    added(&mut all.literals, &one.literals);
    if all.units.len() < one.units.len() {
        all.units.resize(one.units.len(), UnitCounts::default());
    }
    for (sum, each) in all.units.iter_mut().zip(&one.units) {
        *sum = UnitCounts {
            calls: Count(sum.calls.0.saturating_add(each.calls.0)),
            keys: Count(sum.keys.0.saturating_add(each.keys.0)),
            rows: Count(sum.rows.0.saturating_add(each.rows.0)),
            build: Count(sum.build.0.saturating_add(each.build.0)),
            nanos: Count(sum.nanos.0.saturating_add(each.nanos.0)),
            idle_nanos: Count(sum.idle_nanos.0.saturating_add(each.idle_nanos.0)),
            decide_nanos: Count(sum.decide_nanos.0.saturating_add(each.decide_nanos.0)),
            provider_nanos: Count(sum.provider_nanos.0.saturating_add(each.provider_nanos.0)),
        };
    }
    for each in &one.providers {
        match all.providers.iter_mut().find(|held| held.work == each.work) {
            Some(held) => {
                held.asks = Count(held.asks.0.saturating_add(each.asks.0));
                held.rows = Count(held.rows.0.saturating_add(each.rows.0));
                held.nanos = Count(held.nanos.0.saturating_add(each.nanos.0));
                held.idle_nanos = Count(held.idle_nanos.0.saturating_add(each.idle_nanos.0));
            }
            None => all.providers.push(*each),
        }
    }
}

/// The work ledger of a measured run: one row for each unit of `programs` (a rule's own program, a definition's pattern, a
/// made table), with what `counts` holds of it, the rules that reach it and the step of each one's plan it makes the rows
/// of; then one row for each kind of work the language did for them, whose time no unit holds, in the order first done.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::ledger;
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::statistics::types::PointCounts;
///
/// assert!(ledger(&Programs::default(), &PointCounts::default()).0.is_empty());
/// ```
#[must_use]
pub fn ledger(programs: &Programs, counts: &PointCounts) -> WorkLedger {
    let code = &programs.0.code;
    let mut marks = crate::domain::executing::running::types::WalkMarks::default();
    let reached: Vec<Vec<WordSlot>> = code
        .rules
        .iter()
        .map(|rule| units_reached(code, rule, &mut marks))
        .collect();
    let rows = code.units.iter().enumerate().map(|(place, unit)| {
        let slot = WordSlot(u32::try_from(place).unwrap_or(u32::MAX));
        let counted = counts.0.units.get(place).copied().unwrap_or_default();
        let first = usize::try_from(unit.first.0).unwrap_or(usize::MAX);
        let points = usize::try_from(unit.points.0).unwrap_or(usize::MAX);
        let runs = counts.0.points.get(first..first.saturating_add(points));
        let steps = runs
            .unwrap_or_default()
            .iter()
            .fold(counted.build.0, |sum, each| sum.saturating_add(each.0));
        let own = code.rules.iter().find(|rule| rule.unit == slot);
        let owners: Vec<_> = code
            .rules
            .iter()
            .zip(&reached)
            .filter(|(_, units)| units.contains(&slot))
            .map(|(rule, _)| rule.rule)
            .collect();
        LedgerRow {
            unit: match (&unit.key, own) {
                (UnitKey::Rule(_), Some(rule)) => LedgerUnit::Rule(rule.rule),
                (UnitKey::Table(..), _) => LedgerUnit::Table(Count(place)),
                (UnitKey::Pattern(..), _) | (UnitKey::Rule(_), None) => {
                    LedgerUnit::Pattern(Count(place))
                }
            },
            class: class_of(code, &unit.key, slot),
            owner_steps: owners
                .iter()
                .map(|rule| step_of_unit(programs, *rule, &unit.key))
                .collect(),
            owners,
            rows: counted.rows,
            steps: Count(steps),
            nanos: counted.nanos,
            idle_nanos: counted.idle_nanos,
            decide_nanos: counted.decide_nanos,
            provider_nanos: counted.provider_nanos,
        }
    });
    let providers = counts.0.providers.iter().map(|done| LedgerRow {
        unit: LedgerUnit::Provider(done.work),
        class: WorkClass::ProviderFill,
        owners: Vec::new(),
        owner_steps: Vec::new(),
        rows: done.rows,
        steps: done.asks,
        nanos: done.nanos,
        idle_nanos: done.idle_nanos,
        decide_nanos: Count(0),
        provider_nanos: Count(0),
    });
    WorkLedger(rows.chain(providers).collect())
}

/// Adds each count of `one` to the count at its place in `all`.
fn added(all: &mut Vec<Count>, one: &[Count]) {
    if all.len() < one.len() {
        all.resize(one.len(), Count(0));
    }
    for (sum, each) in all.iter_mut().zip(one) {
        *sum = Count(sum.0.saturating_add(each.0));
    }
}

/// The class a unit's reference cost per row is read by: a navigation program, or the kind of fill of its table.
fn class_of(code: &ProgramCode, key: &UnitKey, unit: WordSlot) -> WorkClass {
    let table = code.tables.iter().position(|table| table.unit == unit);
    let made = table.map(|place| MadeTableId(u32::try_from(place).unwrap_or(u32::MAX)));
    match (key, made.map(|number| fill_at(code, number))) {
        (UnitKey::Table(..), Some(Ok(fill))) => match fill.kind {
            FillKind::Rows | FillKind::Constant => WorkClass::RowsFill,
            FillKind::Count | FillKind::Ends | FillKind::Places => WorkClass::GroupFill,
            FillKind::Follow | FillKind::Visits => WorkClass::GraphFill,
            FillKind::Sorted | FillKind::Match => WorkClass::OrderedFill,
        },
        (UnitKey::Table(..), Some(Err(_)) | None) => WorkClass::RowsFill,
        (UnitKey::Rule(_) | UnitKey::Pattern(..), _) => WorkClass::Navigation,
    }
}
