//! Each entry trait method runs its refusal control, over intent inventories that list every variant.
use crate::submission::controls::fixtures::*;
use crate::submission::inventory::fixtures::{
    CAPTURES, DELIVERIES, DEMANDS, REASONS, REFUSALS, capture_place, delivery_place, demand_place,
    every_intent, intent_place, places, reason_place, refusal_place,
};
use crate::submission::witness::fixtures::{ALL, Witness, implements_every_entry_trait};
use std::collections::BTreeSet;

#[test]
pub(super) fn every_entry_method_refuses_an_incomplete_extra_or_undeclared_root() {
    implements_every_entry_trait(&Witness);
    assert_eq!(
        ALL.iter().collect::<BTreeSet<_>>().len(),
        ALL.len(),
        "each entry method has exactly one control"
    );
    assert_eq!(
        places(&DELIVERIES, delivery_place),
        (0..DELIVERIES.len()).collect::<BTreeSet<_>>(),
        "DiagnosticDelivery lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&DEMANDS, demand_place),
        (0..DEMANDS.len()).collect::<BTreeSet<_>>(),
        "ObservationDemand lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&CAPTURES, capture_place),
        (0..CAPTURES.len()).collect::<BTreeSet<_>>(),
        "CaptureIntent lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&REFUSALS, refusal_place),
        (0..REFUSALS.len()).collect::<BTreeSet<_>>(),
        "ApplicabilityRefusal lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&REASONS, reason_place),
        (0..REASONS.len()).collect::<BTreeSet<_>>(),
        "ApplicabilityReason lists each variant once at its place: add or remove its submission control"
    );
    assert_eq!(
        places(&every_intent(), intent_place),
        (0..every_intent().len()).collect::<BTreeSet<_>>(),
        "FileRuleIntent lists each variant once at its place: add or remove its submission control"
    );
    let (mut refused, mut exempt) = (0, 0);
    for &method in ALL {
        match control(method) {
            Control::Refuses(refuses) => {
                refuses();
                refused += 1;
            }
            Control::AcceptsNoRoot(reason) => {
                assert!(!reason.is_empty(), "{method:?} states why it takes no root");
                exempt += 1;
            }
        }
    }
    assert_eq!(
        refused + exempt,
        ALL.len(),
        "every entry method was classified"
    );
}
