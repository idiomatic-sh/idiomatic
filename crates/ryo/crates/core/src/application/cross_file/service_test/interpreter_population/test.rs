//! Project populations stay separate by rule, payload and reached region.
use super::fixtures::many_distinct_project_populations_allocate_only_reached_region_state;

#[test]
fn many_distinct_project_populations_allocate_only_reached_region_state_counted() {
    many_distinct_project_populations_allocate_only_reached_region_state()
        .expect("the counted case runs");
}
