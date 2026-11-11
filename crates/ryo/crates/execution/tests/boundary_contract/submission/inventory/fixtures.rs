//! The support of the `inventory` cases and of the groups nested in it.
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_values::domain::runs::types::ApplicabilityRefusal;
use std::collections::BTreeSet;

/// The position of each variant in its inventory. Each match names every variant with no wildcard, so a variant added to the
/// enum is a compile error (E0004) here until it is given its place in the inventory beside it.
pub(in super::super) fn delivery_place(value: DiagnosticDelivery) -> usize {
    match value {
        DiagnosticDelivery::Disabled => 0,
        DiagnosticDelivery::Collected => 1,
        DiagnosticDelivery::Forwarded => 2,
    }
}

pub(in super::super) fn demand_place(value: ObservationDemand) -> usize {
    match value {
        ObservationDemand::Required => 0,
        ObservationDemand::NotRequired => 1,
    }
}

pub(in super::super) fn capture_place(value: CaptureIntent) -> usize {
    match value {
        CaptureIntent::Disabled => 0,
        CaptureIntent::Observations => 1,
        CaptureIntent::DiagnosticAndObservations => 2,
    }
}

pub(in super::super) fn reason_place(value: ApplicabilityReason) -> usize {
    match value {
        ApplicabilityReason::SyntaxRejected => 0,
        ApplicabilityReason::DiagnosticNotApplicable => 1,
    }
}

pub(in super::super) fn refusal_place(value: ApplicabilityRefusal) -> usize {
    match value {
        ApplicabilityRefusal::SyntaxUndetermined => 0,
        ApplicabilityRefusal::MetadataUnavailable => 1,
    }
}

/// The position of an intent in `every_intent`: the shape of each variant fixes it from the positions of its parts.
pub(in super::super) fn intent_place(value: FileRuleIntent) -> usize {
    let (reasons, refusals) = (REASONS.len(), REFUSALS.len());
    match value {
        FileRuleIntent::Undemanded => 0,
        FileRuleIntent::Skipped(why) => 1 + reason_place(why),
        FileRuleIntent::Refused(why) => 1 + reasons + refusal_place(why),
        FileRuleIntent::Run {
            diagnostic,
            observations,
            capture: captured,
        } => {
            1 + reasons
                + refusals
                + (delivery_place(diagnostic) * DEMANDS.len() + demand_place(observations))
                    * CAPTURES.len()
                + capture_place(captured)
        }
    }
}

/// An inventory lists each variant once, at the place its exhaustive match gives it.
pub(in super::super) fn assert_inventory<T: Copy + std::fmt::Debug>(
    name: &str,
    values: &[T],
    position: fn(T) -> usize,
) {
    let places: BTreeSet<usize> = values.iter().map(|&value| position(value)).collect();
    assert_eq!(
        places,
        (0..values.len()).collect::<BTreeSet<_>>(),
        "{name} lists each variant once at its place: add or remove its submission control"
    );
}

pub(in super::super) const DELIVERIES: [DiagnosticDelivery; 3] = [
    DiagnosticDelivery::Disabled,
    DiagnosticDelivery::Collected,
    DiagnosticDelivery::Forwarded,
];

pub(in super::super) const DEMANDS: [ObservationDemand; 2] =
    [ObservationDemand::Required, ObservationDemand::NotRequired];

pub(in super::super) const CAPTURES: [CaptureIntent; 3] = [
    CaptureIntent::Disabled,
    CaptureIntent::Observations,
    CaptureIntent::DiagnosticAndObservations,
];

pub(in super::super) const REASONS: [ApplicabilityReason; 2] = [
    ApplicabilityReason::SyntaxRejected,
    ApplicabilityReason::DiagnosticNotApplicable,
];

pub(in super::super) const REFUSALS: [ApplicabilityRefusal; 2] = [
    ApplicabilityRefusal::SyntaxUndetermined,
    ApplicabilityRefusal::MetadataUnavailable,
];

pub(in super::super) fn every_intent() -> Vec<FileRuleIntent> {
    let mut all = vec![FileRuleIntent::Undemanded];
    all.extend(REASONS.map(FileRuleIntent::Skipped));
    all.extend(REFUSALS.map(FileRuleIntent::Refused));
    for diagnostic in DELIVERIES {
        for observations in DEMANDS {
            for capture in CAPTURES {
                all.push(FileRuleIntent::Run {
                    diagnostic,
                    observations,
                    capture,
                });
            }
        }
    }
    all
}
