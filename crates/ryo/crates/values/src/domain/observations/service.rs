//! Canonical identity of declared named inputs, independent of formatting and arena state.
use super::errors::ObservationsError;
use super::types::{ObservationBinding, ObservationIdentityBytes, ObservationProducer};
use ryo_relations::domain::relations::types::CellKind;

/// Versioned, typed, length-delimited bytes. Empty declarations preserve ordinary request identity.
#[must_use]
pub fn identity(bindings: &[ObservationBinding]) -> ObservationIdentityBytes {
    if bindings.is_empty() {
        return ObservationIdentityBytes::default();
    }
    let mut bytes = ObservationIdentityBytes(super::constants::BINDINGS_TAG.to_vec());
    tally(&mut bytes, &bindings);
    for binding in bindings {
        text(&mut bytes, &binding.name.0);
        tally(&mut bytes, &binding.columns);
        for column in &binding.columns {
            text(&mut bytes, &column.name.0);
            bytes.0.push(match column.kind {
                CellKind::Node => 0,
                CellKind::Symbol => 1,
                CellKind::File => 2,
                CellKind::Span => 3,
                CellKind::Number => 4,
                CellKind::Flag => 5,
            });
        }
        tally(&mut bytes, &binding.producers);
        for producer in &binding.producers {
            match producer {
                ObservationProducer::Output { rule, output } => {
                    bytes.0.push(0);
                    text(&mut bytes, &rule.0);
                    text(&mut bytes, &output.0);
                }
                ObservationProducer::NoQuery { rule } => {
                    bytes.0.push(1);
                    text(&mut bytes, &rule.0);
                }
            }
        }
    }
    bytes
}
fn tally<I: IntoIterator>(bytes: &mut ObservationIdentityBytes, items: I)
where
    I::IntoIter: ExactSizeIterator,
{
    bytes.0.extend_from_slice(
        &u64::try_from(items.into_iter().len())
            .expect(super::constants::COUNT_FITS_MESSAGE)
            .to_le_bytes(),
    );
}
fn text(bytes: &mut ObservationIdentityBytes, value: &str) {
    tally(bytes, value.as_bytes());
    bytes.0.extend_from_slice(value.as_bytes());
}

/// Allocate compact state only when a request has demanded persistent frontier slots.
pub fn new_frontiers(
    layout: super::types::ObservationReadinessLayout,
    slots: super::types::ObservationReadinessCount,
) -> crate::domain::records::types::FrontierReadiness {
    crate::domain::records::types::FrontierReadiness {
        version: super::constants::READINESS_VERSION,
        layout,
        slots,
        complete: crate::domain::records::types::ReadinessBits(vec![
            0;
            (slots.0 as usize).div_ceil(64)
        ]),
        skipped: crate::domain::records::types::ReadinessBits(Vec::new()),
        refusals: Vec::new(),
    }
}
/// Record an actual state; an unset slot remains Unknown without allocating a boxed reason per root.
pub fn set_frontier(
    map: &mut crate::domain::records::types::FrontierReadiness,
    slot: super::types::ObservationReadinessSlot,
    readiness: super::types::ProducerReadiness,
) -> Result<(), ObservationsError> {
    use super::types::ProducerReadiness;
    use crate::domain::records::types::{FrontierRefusal, FrontierRefusalReason};
    if slot.0 >= map.slots.0 {
        return Err(ObservationsError::Invalid);
    }
    let index = slot.0 as usize / 64;
    let mask = 1_u64 << (slot.0 % 64);
    *map.complete
        .0
        .get_mut(index)
        .ok_or(ObservationsError::Invalid)? &= !mask;
    if let Some(word) = map.skipped.0.get_mut(index) {
        *word &= !mask;
    }
    if let Ok(index) = map
        .refusals
        .binary_search_by_key(&slot, |refusal| refusal.slot)
    {
        let _previous = map.refusals.remove(index);
    }
    match readiness {
        ProducerReadiness::Complete => map.complete.0[index] |= mask,
        ProducerReadiness::Skipped => {
            if map.skipped.0.is_empty() {
                map.skipped.0.resize(map.complete.0.len(), 0);
            }
            *map.skipped
                .0
                .get_mut(index)
                .ok_or(ObservationsError::Invalid)? |= mask;
        }
        ProducerReadiness::Unknown(reason) => {
            let index = map.refusals.partition_point(|refusal| refusal.slot < slot);
            map.refusals.insert(
                index,
                FrontierRefusal {
                    slot,
                    reason: FrontierRefusalReason::Unknown(reason),
                },
            );
        }
        ProducerReadiness::NoQuery => {
            let index = map.refusals.partition_point(|refusal| refusal.slot < slot);
            map.refusals.insert(
                index,
                FrontierRefusal {
                    slot,
                    reason: FrontierRefusalReason::NoQuery,
                },
            );
        }
    }
    Ok(())
}
/// Validate once against the current stable semantic layout before any bit can establish completion.
pub fn validate_frontiers<'a>(
    map: &'a crate::domain::records::types::FrontierReadiness,
    layout: super::types::ObservationReadinessLayout,
    slots: super::types::ObservationReadinessCount,
) -> Result<super::types::ValidatedFrontiers<'a>, ObservationsError> {
    use ObservationsError::Invalid;
    let words = (slots.0 as usize).div_ceil(64);
    if map.version != super::constants::READINESS_VERSION
        || map.layout != layout
        || map.slots != slots
        || map.complete.0.len() != words
        || (!map.skipped.0.is_empty() && map.skipped.0.len() != words)
    {
        return Err(Invalid);
    }
    for (index, complete) in map.complete.0.iter().copied().enumerate() {
        let skipped = map.skipped.0.get(index).copied().unwrap_or(0);
        if complete & skipped != 0 {
            return Err(Invalid);
        }
        let remainder = slots.0 % 64;
        if index + 1 == words && remainder != 0 && (complete | skipped) >> remainder != 0 {
            return Err(Invalid);
        }
    }
    let mut previous = None;
    for refusal in &map.refusals {
        if refusal.slot.0 >= slots.0
            || previous.is_some_and(|old| old >= refusal.slot)
            || bit(&map.complete, refusal.slot)
            || bit(&map.skipped, refusal.slot)
        {
            return Err(Invalid);
        }
        previous = Some(refusal.slot);
    }
    Ok(super::types::ValidatedFrontiers(map))
}
/// Read the already-validated generation. Missing state is an explicit Unknown, not complete-empty.
pub fn frontier_state(
    map: super::types::ValidatedFrontiers<'_>,
    slot: super::types::ObservationReadinessSlot,
) -> Result<super::types::ProducerReadiness, ObservationsError> {
    use super::types::{ObservationReason, ProducerReadiness};
    use crate::domain::records::types::FrontierRefusalReason;
    let map = map.0;
    if slot.0 >= map.slots.0 {
        return Err(ObservationsError::Invalid);
    }
    if bit(&map.complete, slot) {
        return Ok(ProducerReadiness::Complete);
    }
    if bit(&map.skipped, slot) {
        return Ok(ProducerReadiness::Skipped);
    }
    Ok(
        match match map
            .refusals
            .binary_search_by_key(&slot, |refusal| refusal.slot)
        {
            Ok(at) => map.refusals.get(at),
            Err(_) => None,
        } {
            Some(refusal) => match &refusal.reason {
                FrontierRefusalReason::Unknown(reason) => {
                    ProducerReadiness::Unknown(reason.clone())
                }
                FrontierRefusalReason::NoQuery => ProducerReadiness::NoQuery,
            },
            None => ProducerReadiness::Unknown(ObservationReason(
                super::constants::UNPUBLISHED_REASON.into(),
            )),
        },
    )
}
fn bit(
    words: &crate::domain::records::types::ReadinessBits,
    slot: super::types::ObservationReadinessSlot,
) -> bool {
    words
        .0
        .get(slot.0 as usize / 64)
        .is_some_and(|word| word & (1_u64 << (slot.0 % 64)) != 0)
}
/// Digest one original producer's slot range without walking unrelated roots or allocating rows.
pub fn frontier_identity(
    validated: super::types::ValidatedFrontiers<'_>,
    range: super::types::ObservationReadinessRange,
) -> Result<ObservationIdentityBytes, ObservationsError> {
    use super::types::{ObservationReadinessSlot, ProducerReadiness};
    let map = validated.0;
    let end = range
        .start
        .0
        .checked_add(range.length.0)
        .ok_or(ObservationsError::Invalid)?;
    if end > map.slots.0 {
        return Err(ObservationsError::Invalid);
    }
    if range.length.0 == 0 {
        return Ok(ObservationIdentityBytes::default());
    }
    let mut bytes = ObservationIdentityBytes(super::constants::FRONTIER_READINESS_TAG.to_vec());
    bytes.0.extend_from_slice(&map.layout.0.0);
    bytes.0.extend_from_slice(&map.version.0.to_le_bytes());
    bytes.0.extend_from_slice(&range.start.0.to_le_bytes());
    bytes.0.extend_from_slice(&range.length.0.to_le_bytes());
    for index in range.start.0..end {
        match frontier_state(validated, ObservationReadinessSlot(index))? {
            ProducerReadiness::Complete => bytes.0.push(0),
            ProducerReadiness::Unknown(reason) => {
                bytes.0.push(1);
                text(&mut bytes, &reason.0);
            }
            ProducerReadiness::Skipped => bytes.0.push(2),
            ProducerReadiness::NoQuery => bytes.0.push(3),
        }
    }
    Ok(bytes)
}

/// Identity of the actually demanded output set, distinct from all authored exports in a cached plan.
#[must_use]
pub fn output_identity(outputs: &[super::types::PlanObservation]) -> ObservationIdentityBytes {
    use crate::domain::plan_parts::types::StepScope;
    if outputs.is_empty() {
        return ObservationIdentityBytes::default();
    }
    let mut bytes = ObservationIdentityBytes(super::constants::DEMANDED_ROOTS_TAG.to_vec());
    tally(&mut bytes, &outputs);
    for output in outputs {
        text(&mut bytes, &output.name.0);
        bytes.0.push(match output.scope {
            StepScope::PerFile => 0,
            StepScope::CrossFile => 1,
        });
        tally(&mut bytes, &output.columns);
        for column in &output.columns {
            text(&mut bytes, &column.name.0);
            bytes.0.push(match column.kind {
                CellKind::Node => 0,
                CellKind::Symbol => 1,
                CellKind::File => 2,
                CellKind::Span => 3,
                CellKind::Number => 4,
                CellKind::Flag => 5,
            });
        }
    }
    bytes
}
/// Named requests bind original rule positions and their persistent bucket layout before reuse.
/// Reordering/adding rules or changing demanded frontier counts cannot reinterpret old bucket IDs.
#[must_use]
pub fn layout_identity(layout: &[super::types::ObservationRuleLayout]) -> ObservationIdentityBytes {
    let mut bytes = ObservationIdentityBytes(super::constants::REQUEST_LAYOUT_TAG.to_vec());
    tally(&mut bytes, &layout);
    for rule in layout {
        text(&mut bytes, &rule.rule.0);
        bytes.0.extend_from_slice(&rule.frontiers.0.to_le_bytes());
        match &rule.outputs {
            super::types::ObservationOutputLayout::NoPersistentRoots => bytes.0.push(0),
            super::types::ObservationOutputLayout::Persistent {
                configuration,
                outputs,
            } => {
                bytes.0.push(1);
                bytes.0.extend_from_slice(&configuration.0);
                let outputs = output_identity(outputs).0;
                tally(&mut bytes, &outputs);
                bytes.0.extend_from_slice(&outputs);
            }
        }
    }
    bytes
}
