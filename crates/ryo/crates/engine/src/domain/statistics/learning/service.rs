//! The service of the `learning` part: what a sample says of each unit of the programs it ran, and what the second sample
//! of a rule set decides of the first's units. Work is counted, never timed: a unit's work is the runs of its plan points
//! and, for a table, the rows its fill read and gave. The one rule for a form: a flip candidate (a unit the lowering
//! marked open) ran as a call in the first sample and as a table in the second, and keeps the form that did less work;
//! nothing else changes form. The one rule for an order: a unit keeps its measured order unless the second sample, which
//! ran it in that order, did more work in it than the first by over one part in fifty and fifty steps.

use crate::domain::code::types::{FillCode, MeasuredForm, ShapeChoice, UnitKey, WordSlot};
use crate::domain::lowered::service::fill_code;
use crate::domain::lowered::types::MadeFill;
use crate::domain::statistics::constants::{ORDER_SHARE, ORDER_STEPS};
use crate::domain::statistics::types::{Measured, MeasuredUnit, OrderVerdict};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// The unit of a name among units sorted by name.
#[must_use]
pub(crate) fn unit_named<'u>(units: &'u [MeasuredUnit], key: &UnitKey) -> Option<&'u MeasuredUnit> {
    match units.binary_search_by(|unit| unit.key.cmp(key)) {
        Ok(place) => units.get(place),
        Err(_) => None,
    }
}

/// The work a sample counted in a unit: the runs of its plan points, and the rows its fill read and gave.
#[must_use]
pub(crate) fn unit_work(unit: &MeasuredUnit) -> Count {
    let runs = unit.points.iter();
    Count(runs.fold(unit.counts.build.0, |sum, each| sum.saturating_add(each.0)))
}

/// Each unit of the programs a sample ran, with what the sample counted of it, in the form it ran in; sorted by name, a
/// name two units bear kept once (they are one program, lowered from one node).
#[must_use]
pub(crate) fn sampled(sample: Measured<'_>) -> Vec<MeasuredUnit> {
    let (code, counts) = (&sample.programs.0.code, &sample.counts.0);
    let place = |slot: WordSlot| usize::try_from(slot.0).unwrap_or(usize::MAX);
    let mut units: Vec<MeasuredUnit> = code
        .units
        .iter()
        .enumerate()
        .map(|(at, unit)| {
            let first = place(unit.first);
            let runs = counts
                .points
                .get(first..first.saturating_add(place(unit.points)));
            // A unit the sample never reached ran no point: its runs are noughts, one for each of its points.
            let points = runs.map_or_else(|| vec![Count(0); place(unit.points)], <[Count]>::to_vec);
            let counted = counts.units.get(at).copied().unwrap_or_default();
            MeasuredUnit {
                key: unit.key.clone(),
                form: unit.form,
                shape: unit.shape,
                order: OrderVerdict::Learned,
                points,
                counts: counted,
            }
        })
        .collect();
    units.sort_by(|left, right| left.key.cmp(&right.key));
    units.dedup_by(|later, kept| later.key == kept.key);
    units
}

/// The code of the fill of a definition's own rows: what the table of a pattern in table form is named with.
#[must_use]
pub(crate) fn rows_code() -> FillCode {
    fill_code(&MadeFill::Rows(Box::new([]), RepeatedRows::Kept))
}

/// The form a flip candidate keeps. Its call form cost the first sample the runs of its points. Its table form cost the
/// second sample its table's fill and the reads of the table, shared among the patterns of its definition that read the
/// table in table form. The table form is kept when it cost less; a candidate the second sample did not run as a table
/// keeps its call form.
fn form_kept(candidate: &MeasuredUnit, second: &[MeasuredUnit]) -> MeasuredForm {
    let node = match &candidate.key {
        UnitKey::Pattern(node, _, _) => *node,
        UnitKey::Rule(_) | UnitKey::Table(..) => return candidate.form,
    };
    let again = unit_named(second, &candidate.key).map(|unit| unit.form);
    let table = unit_named(second, &UnitKey::Table(node, rows_code()));
    match (candidate.form, again, table) {
        (MeasuredForm::Call, Some(MeasuredForm::Table), Some(table)) => {
            let readers = second.iter().filter(|unit| match &unit.key {
                UnitKey::Pattern(of, _, _) => *of == node && unit.form == MeasuredForm::Table,
                UnitKey::Rule(_) | UnitKey::Table(..) => false,
            });
            let as_table = unit_work(table).0.saturating_add(table.counts.calls.0);
            let as_call = unit_work(candidate).0.saturating_mul(readers.count());
            if as_table < as_call {
                MeasuredForm::Table
            } else {
                MeasuredForm::Call
            }
        }
        (MeasuredForm::Call, Some(MeasuredForm::Call | MeasuredForm::Table) | None, _)
        | (MeasuredForm::Table, _, _) => candidate.form,
    }
}

/// Whether a unit's measured order stands: the second sample ran it in that order, and did not do more work in it than the
/// first by over one part in fifty and fifty steps. A unit the second sample did not run was never run in its measured order.
fn order_kept(unit: &MeasuredUnit, second: &[MeasuredUnit]) -> OrderVerdict {
    let Some(again) = unit_named(second, &unit.key) else {
        return OrderVerdict::Denied;
    };
    let before = unit_work(unit).0;
    let allowed = before
        .saturating_add(before / ORDER_SHARE.0)
        .saturating_add(ORDER_STEPS.0);
    if unit_work(again).0 > allowed {
        OrderVerdict::Denied
    } else {
        OrderVerdict::Learned
    }
}

/// What the second sample decides of the first's units: each as the first counted it, a flip candidate in the form that did
/// less work, and each with its order's verdict. A candidate that keeps its call form is denied its measured order, which
/// never ran in that form; one that keeps its table form has no step of its own to order.
#[must_use]
pub(crate) fn decided(first: &[MeasuredUnit], second: &[MeasuredUnit]) -> Vec<MeasuredUnit> {
    let each = first.iter().map(|unit| {
        let (form, order) = match unit.shape {
            ShapeChoice::Open => match form_kept(unit, second) {
                MeasuredForm::Call => (MeasuredForm::Call, OrderVerdict::Denied),
                MeasuredForm::Table => (MeasuredForm::Table, OrderVerdict::Learned),
            },
            ShapeChoice::Decided => (unit.form, order_kept(unit, second)),
        };
        MeasuredUnit {
            form,
            order,
            ..unit.clone()
        }
    });
    each.collect()
}
