//! Independent source inputs through the actual public use case and real process boundary.
use super::interface::VerifyCasesUseCase as _;
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
pub fn exercise(scenario: Scenario) -> Result<usize, Refusal> {
    crate::inbound::use_cases::certify::fixtures::with_verification_sources(
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
            let result = super::use_case::VerifyCasesUseCaseImpl::new(
                suite, reference, candidate, execution,
            )
            .verify_cases(
                &input,
                scratch,
                ryo_benchmark_contracts::domain::requests::types::CaseSelection::default(),
                ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted,
            )
            .map(|count| count.0);
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
