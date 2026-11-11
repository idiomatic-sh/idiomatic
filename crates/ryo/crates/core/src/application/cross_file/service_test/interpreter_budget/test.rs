//! The memory budget stops a later population, and a refused frontier keeps its compiled source.
use super::fixtures::{
    project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase,
    refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort,
};

#[test]
fn project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase_counted() {
    project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase()
        .expect("the counted case runs");
}

#[test]
fn refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort_counted() {
    refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort()
        .expect("the counted case runs");
}
