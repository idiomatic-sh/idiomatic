//! Independent source inputs through the actual public use case and real process boundary.
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

/// The public operation under test, given the ports, the rules and the scratch directory.
pub type Operate =
    fn(&dyn SuitePort, &dyn LinterPort, &dyn ExecutionPort, &Path, &Path) -> Result<usize, ReportsError>;
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    Complete,
    Missing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    MissingSource,
    Unexpected,
}
/// Run one public operation over independently authored source facts.
pub fn exercise(scenario: Scenario, operate: Operate) -> Result<usize, Refusal> {
    crate::inbound::use_cases::prepare_cases::fixtures::with_verification_sources(
        |suite, reference, _candidate, execution, rules, scratch, launches| {
            let input = match scenario {
                Scenario::Complete => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = operate(suite, reference, execution, &input, scratch);
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.unwrap().lines().count(), 2);
                }
                Scenario::Missing => assert_eq!(
                    std::fs::symlink_metadata(launches).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                ),
            }
            result.map_err(|error| match error {
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Refusal::MissingSource
                }
                _ => Refusal::Unexpected,
            })
        },
    )
}
