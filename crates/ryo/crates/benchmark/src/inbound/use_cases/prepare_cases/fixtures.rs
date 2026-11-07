//! Independent source inputs through the actual public use case and real process boundary.
use super::interface::PrepareCasesUseCase as _;
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
pub fn exercise(scenario: Scenario) -> Result<usize, Refusal> {
    crate::inbound::use_cases::certify::fixtures::with_verification_sources(
        |suite, _reference, _candidate, _execution, rules, scratch, launches| {
            let input = match scenario {
                Scenario::Complete => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = super::use_case::PrepareCasesUseCaseImpl::new(suite)
                .prepare_cases(&input, scratch)
                .map(|cases| {
                    assert_eq!(
                        cases
                            .iter()
                            .map(|case| case.expected.diagnostics.len())
                            .collect::<Vec<_>>(),
                        [1, 0]
                    );
                    cases.len()
                });
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.unwrap_err().kind(), std::io::ErrorKind::NotFound);
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
