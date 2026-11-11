//! The support of the `distinct_integration` cases.
use super::super::fixtures::{Scenario, exercise};

pub(super) fn run(
    scenario: Scenario,
) -> Result<Vec<u64>, ryo_execution_domain::domain::operating::errors::OperatingError> {
    exercise(
        scenario,
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        std::sync::Arc::new(crate::application::file_dag::fixtures::operators()),
    )
}
