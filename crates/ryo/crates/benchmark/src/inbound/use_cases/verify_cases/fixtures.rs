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
///
/// # Errors
/// [`Refusal::MissingSource`] when the operation refuses the missing rules directory, [`Refusal::Unaccounted`] when a default
/// rule has no implementation, and [`Refusal::Unexpected`] for any other
/// refusal or a fixture write that fails.
///
/// # Panics
/// When the contestants are launched other than the scenario promises, or a fixture source cannot be written.
pub fn exercise(scenario: Scenario, operate: Operate) -> Result<usize, Refusal> {
    crate::inbound::use_cases::prepare_cases::fixtures::with_verification_sources(
        |suite, reference, candidate, execution, rules, scratch, launches| {
            if matches!(scenario, Scenario::Unaccounted) {
                let profile = rules
                    .parent()
                    .ok_or(Refusal::Unexpected)?
                    .join("benchmark.toml");
                let text = std::fs::read_to_string(&profile)
                    .map_err(|_| Refusal::Unexpected)?
                    .replace("defaults=['A']", "defaults=['A','B']");
                std::fs::write(profile, text).map_err(|_| Refusal::Unexpected)?;
                std::fs::create_dir(rules.join("B")).map_err(|_| Refusal::Unexpected)?;
                std::fs::write(
                    rules.join("B/cases.toml"),
                    "[[cases]]\nname='oracle'\nfiles={'case.py'='x'}\nexpected=[]\n",
                )
                .map_err(|_| Refusal::Unexpected)?;
            }
            let input = match scenario {
                Scenario::Complete | Scenario::Unaccounted => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = operate(suite, reference, candidate, execution, &input, scratch);
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.ok().map(|text| text.lines().count()), Some(4));
                }
                Scenario::Missing | Scenario::Unaccounted => assert_eq!(
                    std::fs::symlink_metadata(launches)
                        .err()
                        .map(|error| error.kind()),
                    Some(std::io::ErrorKind::NotFound)
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
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(_)
                | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(_)
                | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Json(_)
                | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Toml(_) => {
                    Refusal::Unexpected
                }
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
