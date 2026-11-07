//! Independent source inputs through the actual public use case and real process boundary.
use super::interface::VerifyReferenceCasesUseCase as _;
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
        |suite, reference, _candidate, execution, rules, scratch, launches| {
            let input = match scenario {
                Scenario::Complete => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let inventory = crate::outbound::inventory::factory::local();
            let result = super::use_case::VerifyReferenceCasesUseCaseImpl::new(
                suite, reference, execution, &inventory,
            )
            .verify_reference_cases(&input, scratch)
            .map(|count| count.0);
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
