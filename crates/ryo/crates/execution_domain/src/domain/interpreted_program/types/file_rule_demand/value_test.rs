//! A rule's file demand reads the one compiled mode each demand selects.
use super::value::FileRuleDemand;
use crate::domain::interpreted_program::file_demand::types::{
    FileDemand, FileMode, FrontierSelection, NamedOutputCount, RootSelection,
};
use crate::domain::interpreted_program::types::RegionId;

#[test]
fn each_demand_selects_its_own_mode() {
    let modes = (0..12)
        .map(|at| FileMode {
            readers: Box::new([RegionId(at)]),
            roots: Box::new([]),
            observations: Box::new([]),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let demand = FileRuleDemand::compiled(NamedOutputCount(2), modes);
    let selected = |diagnostic, observations, capture| {
        demand
            .selected(FileDemand {
                diagnostic,
                observations,
                capture,
            })
            .readers[0]
    };
    use FrontierSelection::{All, Disabled, Observations};
    use RootSelection::{Required, Undemanded};
    assert_eq!(selected(Undemanded, Undemanded, Disabled), RegionId(0));
    assert_eq!(selected(Undemanded, Undemanded, All), RegionId(2));
    assert_eq!(selected(Undemanded, Required, Observations), RegionId(4));
    assert_eq!(selected(Required, Undemanded, Disabled), RegionId(6));
    assert_eq!(selected(Required, Required, All), RegionId(11));
    assert_eq!(demand.outputs.0, 2);
}
