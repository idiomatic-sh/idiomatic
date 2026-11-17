//! Every evidence destination gathers costs, verification times and growth into one document, written once by `finish`.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::interface::EvidencePort;

/// One call of the port, in the order a bench makes it.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    /// An exact comparison of two complete reports.
    Comparison,
    /// An empty set of costs.
    EmptyCosts,
    /// A project's costs.
    Costs,
    /// A rule's verification times.
    Verification,
    /// A shape's growth.
    Growth,
    /// The times of no verification file.
    ReadNoVerification,
    /// The document written.
    Finish,
}
type Input = &'static [Step];
/// The rules whose times were read.
type Value = usize;
type Failure = ();

/// The rows every destination answers, run in order over one destination: the refusals first, then a bench's calls up
/// to its one `finish`.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an empty set of costs is refused"),
        word: Word::Refuses,
        input: || &[Step::EmptyCosts],
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an exact comparison of two complete reports is written"),
        word: Word::Succeeds,
        input: || &[Step::Comparison],
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a project's costs, a rule's times and a shape's growth are added"),
        word: Word::Succeeds,
        input: || &[Step::Costs, Step::Verification, Step::Growth],
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("no verification file gives no times"),
        word: Word::Succeeds,
        input: || &[Step::ReadNoVerification],
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the document of what was added is written"),
        word: Word::Succeeds,
        input: || &[Step::Finish],
        expect: Expect::Value(0),
    },
];

/// Make each call of `steps` on `adapter`, answering the rules whose times were read.
fn subject(adapter: &impl EvidencePort, steps: Input) -> Result<Value, Failure> {
    let language: ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage =
        "python".into();
    let mut read = 0;
    for step in steps {
        let outcome = match step {
            Step::Comparison => {
                let report = super::sample::report();
                adapter.comparison(
                    &report,
                    &report,
                    ryo_benchmark_contracts::domain::reports::types::ReportComparison {
                        diagnostics: ryo_benchmark_contracts::domain::reports::types::Comparison::Exact,
                        execution_metadata:
                            ryo_benchmark_contracts::domain::reports::types::Comparison::Exact,
                    },
                    ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics,
                )
            }
            Step::EmptyCosts => adapter.costs(
                &language,
                &ryo_benchmark_contracts::domain::costs::types::ProjectCosts(Vec::new()),
            ),
            Step::Costs => adapter.costs(&language, &super::sample::costs()),
            Step::Verification => adapter.verification(&language, &super::sample::timings()),
            Step::Growth => adapter.growth(&language, &super::sample::growth()),
            Step::ReadNoVerification => adapter
                .read_verification(
                    &crate::outbound::evidence::types::VerificationTimesFiles::default(),
                )
                .map(|timings| read += timings.0.len()),
            Step::Finish => adapter.finish(),
        };
        outcome.map_err(|_refused| ())?;
    }
    Ok(read)
}

/// Run the contract's rows against one supplied destination, the refusals first.
/// # Panics
/// Panics if the destination violates the evidence contract.
pub fn check(adapter: &impl EvidencePort) {
    run(CASES, Word::Refuses, |steps| subject(adapter, steps));
    run(CASES, Word::Succeeds, |steps| subject(adapter, steps));
}
