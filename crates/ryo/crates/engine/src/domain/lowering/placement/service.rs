//! The services of the `placement` part: the placement rule (scope-engine3 2.6), and what a load's statistics say of a
//! sample. No case refuses: Table is always admissible, and a definition whose plan joins with no key is a table with a
//! product stage, made once.

use super::constants::CALL_PATTERNS;
use super::types::{CallForm, FormChoice, PatternFacts, PatternPlacement, SampleStage, TableForm};
use crate::domain::code::types::{MeasuredForm, ShapeChoice, UnitKey};
use crate::domain::statistics::types::{LearnStage, Statistics};

/// A form the shape of a pattern's plans decides: no sample flips it.
const fn decided(form: FormChoice) -> PatternPlacement {
    PatternPlacement {
        form,
        shape: ShapeChoice::Decided,
    }
}

/// The form of a pattern. A pattern that binds nothing is a table. A pattern with no program is a table: what cannot be
/// keyed is made once. A pattern past its definition's call-form patterns is a table. Any other is Call, unless it is a flip
/// candidate (its table holds no product stage) and the statistics say Table: every candidate after a first run, the form
/// written for it after a second.
#[must_use]
pub(crate) const fn placed(facts: PatternFacts, stage: SampleStage) -> PatternPlacement {
    if facts.binds.0 == 0 {
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
    if facts.call_patterns.0 >= CALL_PATTERNS {
        return decided(FormChoice::Table);
    }
    match facts.table {
        TableForm::Product | TableForm::Calls => decided(FormChoice::Call),
        TableForm::Keyed => PatternPlacement {
            form: match (stage, facts.measured) {
                (SampleStage::Absent, None | Some(MeasuredForm::Call | MeasuredForm::Table))
                | (SampleStage::Second, None | Some(MeasuredForm::Call)) => FormChoice::Call,
                (SampleStage::First, None | Some(MeasuredForm::Call | MeasuredForm::Table))
                | (SampleStage::Second, Some(MeasuredForm::Table)) => FormChoice::Table,
            },
            shape: ShapeChoice::Open,
        },
    }
}

/// What a load's statistics are of: none given, a first run's (with or without a unit), or a second run's.
#[must_use]
pub(crate) const fn sampled(statistics: Option<&Statistics>) -> SampleStage {
    match statistics {
        None => SampleStage::Absent,
        Some(statistics) => match statistics.stage {
            LearnStage::First => SampleStage::First,
            LearnStage::Second => SampleStage::Second,
        },
    }
}

/// The form the statistics kept for a unit, when they name it.
#[must_use]
pub(crate) fn measured(statistics: Option<&Statistics>, key: &UnitKey) -> Option<MeasuredForm> {
    let mut units = statistics
        .into_iter()
        .flat_map(|statistics| &statistics.units);
    units.find(|unit| unit.key == *key).map(|unit| unit.form)
}
