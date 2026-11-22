//! The services of the `needs` part: an emptiness formula written as need records, and the site a unit's decision is read at.
//! A formula's parts are written before it, so a record names only records before it; a definition's own formula is written
//! once, with the rule that first reached it, and every formula that reads it names its site.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::{push_list, push_need};
use crate::domain::code::records::types::NeedRecord;
use crate::domain::lowered::types::ByteLiteral;
use ryo_language::domain::facts::types::CellWord;
use crate::domain::code::small_codes::service::{comparison_code, comparison_regex};
use crate::domain::code::types::{NeedSite, WordSlot};
use crate::domain::encoding::sites::service::{constant, fault, reach_table, relation, slot_of};
use crate::domain::encoding::types::{Encoding, ReachedUnit};
use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::types::DefinitionId;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::service::column_place;

/// A need site with no record yet.
fn site(encoding: &mut Encoding<'_>) -> Result<NeedSite, LoweringError> {
    let site = NeedSite(
        slot_of(
            encoding,
            Count(encoding.need_places.len()),
            PastWidth::Sites,
        )?
        .0,
    );
    encoding.need_places.push(NO_SLOT);
    Ok(site)
}

/// Gives a need site its record's place.
fn set(encoding: &mut Encoding<'_>, site: NeedSite, place: WordSlot) -> Result<(), LoweringError> {
    let at = usize::try_from(site.0).map_err(|_| fault(encoding))?;
    match encoding.need_places.get_mut(at) {
        Some(slot) => {
            *slot = place;
            Ok(())
        }
        None => Err(fault(encoding)),
    }
}

/// The need site of a definition's own formula: the same site for every formula that reads it and for every use of
/// one of its patterns in call form.
pub(crate) fn definition_site(
    encoding: &mut Encoding<'_>,
    definition: DefinitionId,
) -> Result<NeedSite, LoweringError> {
    if let Some(site) = encoding.definition_needs.get(&definition) {
        return Ok(*site);
    }
    let site = site(encoding)?;
    let _new = encoding.definition_needs.insert(definition, site);
    encoding.reached.push(ReachedUnit::Need(definition, site));
    Ok(site)
}

/// Writes the record of a formula over parts: the parts' records first, then the record with each part's place after it.
fn combined(
    encoding: &mut Encoding<'_>,
    parts: &[Emptiness],
    record: fn(WordSlot) -> NeedRecord,
) -> Result<WordSlot, LoweringError> {
    let places: Result<Vec<WordSlot>, LoweringError> =
        parts.iter().map(|part| formula(encoding, part)).collect();
    let places = places?;
    let count = slot_of(encoding, Count(places.len()), PastWidth::Sites)?;
    let at = push_need(&mut encoding.need_records, record(count));
    for place in places {
        let offset = slot_of(
            encoding,
            Count(encoding.need_records.len()),
            PastWidth::Sites,
        )?;
        encoding.need_offsets.push(offset);
        encoding.need_records.push(place);
    }
    Ok(at)
}

/// Writes a formula's record, its parts before it, answering the record's place among the records.
fn formula(encoding: &mut Encoding<'_>, emptiness: &Emptiness) -> Result<WordSlot, LoweringError> {
    let record = match emptiness {
        Emptiness::Unknown => NeedRecord::Unknown,
        Emptiness::Kind(kinds) => {
            let slots: Vec<WordSlot> = kinds
                .0
                .iter()
                .map(|kind| WordSlot(u32::from(kind.0)))
                .collect();
            NeedRecord::Kinds(push_list(&mut encoding.code.pool, &slots))
        }
        Emptiness::Row(read, value) => {
            let relation = relation(encoding, *read)?;
            let (column, constant) = match value {
                Some(value) => (
                    WordSlot(column_place(value.column).0),
                    WordSlot(constant(encoding, value.value)?.0),
                ),
                None => (NO_SLOT, NO_SLOT),
            };
            NeedRecord::Row {
                relation,
                column,
                constant,
            }
        }
        Emptiness::Definition(definition) => {
            NeedRecord::Site(definition_site(encoding, *definition)?)
        }
        Emptiness::Table(table) => {
            reach_table(encoding, *table)?;
            NeedRecord::Table(*table)
        }
        Emptiness::Path(path) => NeedRecord::Path {
            comparison: WordSlot(u32::from(comparison_code(path.test).0)),
            regex: comparison_regex(path.test).map_or(NO_SLOT, |regex| WordSlot(regex.0)),
            constant: WordSlot(constant(encoding, path.value)?.0),
        },
        Emptiness::Bytes(ByteLiteral::Text(text)) => NeedRecord::Bytes {
            regex: NO_SLOT,
            constant: WordSlot(constant(encoding, *text)?.0),
        },
        Emptiness::Bytes(ByteLiteral::Regex(regex)) => NeedRecord::Bytes {
            regex: WordSlot(regex.0),
            constant: NO_SLOT,
        },
        Emptiness::Shape(shape) => {
            let mut slot = |word: Option<CellWord>| -> Result<WordSlot, LoweringError> {
                Ok(match word {
                    Some(word) => WordSlot(constant(encoding, word)?.0),
                    None => NO_SLOT,
                })
            };
            NeedRecord::Shape {
                parent: slot(shape.parent)?,
                field: slot(shape.field)?,
                child: slot(shape.child)?,
            }
        }
        Emptiness::All(parts) => {
            return combined(encoding, parts, |count| NeedRecord::All {
                parts: count,
                offsets: NO_SLOT,
            });
        }
        Emptiness::Any(parts) => {
            return combined(encoding, parts, |count| NeedRecord::Any {
                parts: count,
                offsets: NO_SLOT,
            });
        }
    };
    Ok(push_need(&mut encoding.need_records, record))
}

/// The need site of a formula: what a rule's branch, or a table, waits on.
pub(crate) fn need_site(
    encoding: &mut Encoding<'_>,
    emptiness: &Emptiness,
) -> Result<NeedSite, LoweringError> {
    let site = site(encoding)?;
    let place = formula(encoding, emptiness)?;
    set(encoding, site, place)?;
    Ok(site)
}

/// Writes a definition's own formula for the site its readers were given.
pub(crate) fn definition_need(
    encoding: &mut Encoding<'_>,
    definition: DefinitionId,
    site: NeedSite,
) -> Result<(), LoweringError> {
    let (pack, at) = (
        encoding.pack,
        usize::try_from(definition.0).map_err(|_| fault(encoding))?,
    );
    let emptiness = &pack
        .definitions
        .get(at)
        .ok_or_else(|| fault(encoding))?
        .emptiness;
    let place = formula(encoding, emptiness)?;
    set(encoding, site, place)
}

/// The needs as the code holds them: one place for each site, then the records, every place counted from the first slot.
pub(crate) fn assembled(encoding: &Encoding<'_>) -> Result<Vec<WordSlot>, LoweringError> {
    let sites = slot_of(
        encoding,
        Count(encoding.need_places.len()),
        PastWidth::Sites,
    )?;
    let moved = |place: WordSlot| {
        place
            .0
            .checked_add(sites.0)
            .map(WordSlot)
            .ok_or_else(|| fault(encoding))
    };
    let mut needs = Vec::with_capacity(
        encoding
            .need_places
            .len()
            .saturating_add(encoding.need_records.len()),
    );
    for place in &encoding.need_places {
        needs.push(moved(*place)?);
    }
    let mut records = encoding.need_records.clone();
    for offset in &encoding.need_offsets {
        let at = usize::try_from(offset.0).map_err(|_| fault(encoding))?;
        let Some(slot) = records.get_mut(at) else {
            return Err(fault(encoding));
        };
        *slot = moved(*slot)?;
    }
    needs.extend(records);
    Ok(needs)
}
