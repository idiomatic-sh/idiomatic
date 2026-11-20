//! The services of the `units` part: a unit begun and ended around the words written for it, the plan point each word is
//! counted at among the code's, and a run of words given its place in the arena.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{CodeEntry, MeasuredForm, UnitCode, UnitKey, WordSlot};
use crate::domain::encoding::sites::service::{fault, past, slot_of};
use crate::domain::encoding::types::{Encoding, PlacedWord, UnitPoints};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::types::PlanPoint;
use ryo_relations::domain::relations::types::Count;

/// Starts a unit: its plan points follow the ones numbered so far.
pub(crate) fn begin_unit(
    encoding: &mut Encoding<'_>,
    key: UnitKey,
    form: MeasuredForm,
) -> Result<(), LoweringError> {
    let unit = slot_of(encoding, Count(encoding.code.units.len()), PastWidth::Sites)?;
    let first = encoding.code.counts.points;
    encoding.code.units.push(UnitCode {
        key,
        form,
        first,
        points: WordSlot(0),
    });
    encoding.unit = UnitPoints {
        unit,
        first,
        reach: WordSlot(0),
    };
    Ok(())
}

/// Ends the unit being written: its plan points are counted.
pub(crate) fn end_unit(encoding: &mut Encoding<'_>) -> Result<(), LoweringError> {
    let points = encoding.unit.reach;
    let total = encoding
        .unit
        .first
        .0
        .checked_add(points.0)
        .ok_or_else(|| past(encoding, PastWidth::Sites))?;
    let place = usize::try_from(encoding.unit.unit.0).map_err(|_| fault(encoding))?;
    if let Some(unit) = encoding.code.units.get_mut(place) {
        unit.points = points;
    }
    encoding.code.counts.points = WordSlot(total);
    Ok(())
}

/// The plan point a word is counted at among the code's: its unit's first and its own place in the unit; `NO_SLOT` for a word
/// with none.
pub(crate) fn point(
    encoding: &mut Encoding<'_>,
    local: Option<PlanPoint>,
) -> Result<WordSlot, LoweringError> {
    let Some(local) = local else {
        return Ok(NO_SLOT);
    };
    let reach = local
        .0
        .checked_add(1)
        .ok_or_else(|| past(encoding, PastWidth::Sites))?;
    if reach > encoding.unit.reach.0 {
        encoding.unit.reach = WordSlot(reach);
    }
    encoding
        .unit
        .first
        .0
        .checked_add(local.0)
        .map(WordSlot)
        .ok_or_else(|| past(encoding, PastWidth::Sites))
}

/// Gives a run of words its place in the arena, answering where it starts.
pub(crate) fn placed(
    encoding: &mut Encoding<'_>,
    words: &[PlacedWord],
) -> Result<CodeEntry, LoweringError> {
    let entry = CodeEntry(slot_of(encoding, Count(encoding.code.words.len()), PastWidth::Words)?.0);
    for each in words {
        encoding.code.words.push(each.word);
        encoding.code.points.push(each.point);
    }
    Ok(entry)
}
