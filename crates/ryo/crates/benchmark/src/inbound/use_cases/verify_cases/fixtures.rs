//! Independent source inputs through the actual public use case and real process boundary.
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

/// The public operation under test, given the ports, the rules and the scratch directory.
pub type Operate = fn(
    &dyn SuitePort,
    &dyn LinterPort,
    &dyn LinterPort,
    &dyn ExecutionPort,
    &Path,
    &Path,
) -> Result<usize, ReportsError>;
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    Complete,
    Missing,
    Unaccounted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    MissingSource,
    Unaccounted,
    Unexpected,
}
/// Run one public operation over independently authored source facts.
pub fn exercise(scenario: Scenario, operate: Operate) -> Result<usize, Refusal> {
    crate::inbound::use_cases::prepare_cases::fixtures::with_verification_sources(
        |suite, reference, candidate, execution, rules, scratch, launches| {
            if matches!(scenario, Scenario::Unaccounted) {
                let profile = rules.parent().unwrap().join("benchmark.toml");
                let text = std::fs::read_to_string(&profile)
                    .unwrap()
                    .replace("defaults=['A']", "defaults=['A','B']");
                std::fs::write(profile, text).unwrap();
                std::fs::create_dir(rules.join("B")).unwrap();
                std::fs::write(
                    rules.join("B/cases.toml"),
                    "[[cases]]\nname='oracle'\nfiles={'case.py'='x'}\nexpected=[]\n",
                )
                .unwrap();
            }
            let input = match scenario {
                Scenario::Complete | Scenario::Unaccounted => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = operate(suite, reference, candidate, execution, &input, scratch);
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.unwrap().lines().count(), 4);
                }
                Scenario::Missing | Scenario::Unaccounted => assert_eq!(
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
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(
                    message,
                ) if message.contains("defaults lack implementation or explicit deferral") => {
                    Refusal::Unaccounted
                }
                _ => Refusal::Unexpected,
            })
        },
    )
}

/// Independent authored evidence, constructed apart from either contestant response.

#[cfg(test)]
pub(crate) mod discrepancy;
#[cfg(test)]
pub(crate) mod process_capture;
#[cfg(test)]
pub(crate) mod process_identity;
#[cfg(test)]
pub(crate) mod process_replay;
#[cfg(test)]
pub(crate) mod shard;
