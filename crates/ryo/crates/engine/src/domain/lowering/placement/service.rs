//! The services of the `placement` part: the placement rule (scope-engine3 2.6), and what a load's statistics say of a
//! sample. No case refuses: Table is always admissible, and a definition whose plan joins with no key is a table with a
//! product stage, made once.

use super::constants::CALL_PATTERNS;
use super::types::{CallForm, FormChoice, PatternFacts, Placement, SampleStage, TableForm};
use crate::domain::code::types::{MeasuredForm, UnitKey};
use crate::domain::statistics::types::{LearnStage, ShapeChoice, Statistics};

/// A form the shape of a pattern's plans decides: no sample flips it.
const fn decided(form: FormChoice) -> Placement {
    Placement {
        form,
        shape: ShapeChoice::Decided,
    }
}

/// The form of a pattern. A pattern that binds nothing is a table. A pattern with no program is a table: what cannot be
/// keyed is made once. A pattern past its definition's call-form patterns is a table. Any other is Call, unless it is a flip
/// candidate (its table holds no product stage) and the statistics say Table: every candidate after a first run, the form
/// written for it after a second.
#[must_use]
pub(crate) const fn placed(facts: PatternFacts, stage: SampleStage) -> Placement {
    if facts.binds == 0 {
        return decided(FormChoice::Table);
    }
    match facts.call {
        CallForm::Scans => return decided(FormChoice::Table),
        CallForm::Keyed => {}
    }
    match facts.table {
        // The table's own pipeline runs this definition's programs: a pattern of it that has a program keeps it.
        TableForm::Calls => return decided(FormChoice::Call),
        TableForm::Keyed | TableForm::Product => {}
    }
    if facts.call_patterns >= CALL_PATTERNS {
        return decided(FormChoice::Table);
    }
    match facts.table {
        TableForm::Product | TableForm::Calls => decided(FormChoice::Call),
        TableForm::Keyed => Placement {
            form: match (stage, facts.measured) {
                (SampleStage::Absent, _)
                | (SampleStage::Second, None | Some(MeasuredForm::Call)) => FormChoice::Call,
                (SampleStage::First, _) | (SampleStage::Second, Some(MeasuredForm::Table)) => {
                    FormChoice::Table
                }
            },
            shape: ShapeChoice::Open,
        },
    }
}

/// What a load's statistics are of. At this contract, absent statistics and a first run's with no unit are one value: it is
/// read as absent, so a load given nothing takes Call for every candidate; a first run's that names a unit is a first run's.
#[must_use]
pub(crate) fn sampled(statistics: &Statistics) -> SampleStage {
    match (statistics.stage, statistics.units.is_empty()) {
        (LearnStage::First, true) => SampleStage::Absent,
        (LearnStage::First, false) => SampleStage::First,
        (LearnStage::Second, _) => SampleStage::Second,
    }
}

/// The form the statistics kept for a unit, when they name it.
#[must_use]
pub(crate) fn measured(statistics: &Statistics, key: &UnitKey) -> Option<MeasuredForm> {
    let units = statistics.units.iter();
    units.clone().find(|unit| unit.key == *key).map(|unit| unit.form)
}
