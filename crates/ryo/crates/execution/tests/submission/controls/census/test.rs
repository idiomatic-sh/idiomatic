//! Each entry trait method runs its refusal control, over intent inventories that list every variant.
use crate::submission::controls::fixtures::*;
use crate::submission::inventory::fixtures::{
    CAPTURES, DELIVERIES, DEMANDS, REASONS, REFUSALS, assert_inventory, capture_place,
    delivery_place, demand_place, every_intent, intent_place, reason_place, refusal_place,
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
    assert_inventory("DiagnosticDelivery", &DELIVERIES, delivery_place);
    assert_inventory("ObservationDemand", &DEMANDS, demand_place);
    assert_inventory("CaptureIntent", &CAPTURES, capture_place);
    assert_inventory("ApplicabilityRefusal", &REFUSALS, refusal_place);
    assert_inventory("ApplicabilityReason", &REASONS, reason_place);
    assert_inventory("FileRuleIntent", &every_intent(), intent_place);
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
