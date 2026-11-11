//! A set of project roots prepares once and delivers in order.
use super::fixtures::{
    project_root_set_prepares_shared_publications_once_before_ordered_delivery,
    project_root_set_preserves_distinct_actual_population_policy_in_both_orders,
};

#[test]
fn project_root_set_prepares_shared_publications_once_before_ordered_delivery_counted() {
    project_root_set_prepares_shared_publications_once_before_ordered_delivery()
        .expect("the counted case runs");
}

#[test]
fn project_root_set_preserves_distinct_actual_population_policy_in_both_orders_counted() {
    project_root_set_preserves_distinct_actual_population_policy_in_both_orders()
        .expect("the counted case runs");
}
