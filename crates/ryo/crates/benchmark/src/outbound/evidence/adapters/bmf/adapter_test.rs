//! The concrete stream destination answers the publication contract, and writes one document per run, at finish.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::fixtures::contract::check;
use crate::outbound::evidence::fixtures::sample;
use crate::outbound::evidence::interface::EvidencePort as _;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The port contract over sinks.
    Contract,
    /// Costs, verification times and growth added, then finish called twice.
    OneDocument,
}
type Input = Scenario;
/// The lines written before finish, after the first finish and after the refused second one.
type Value = [usize; 3];
type Failure = Refusal;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// A measure was refused, or the first finish failed.
    Unexpected,
    /// A second finish wrote or was accepted.
    SecondDocument,
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("BMF destination honors publication contract"),
        word: Word::Succeeds,
        input: || Scenario::Contract,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName(
            "every measure of one run is one document: nothing before finish, one line at finish, and a second finish is refused",
        ),
        word: Word::Succeeds,
        input: || Scenario::OneDocument,
        expect: Expect::Value([0, 1, 1]),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Contract => {
            check(&crate::outbound::evidence::factory::bmf(
                Box::new(std::io::sink()),
                Box::new(std::io::sink()),
            ));
            Ok([0, 0, 0])
        }
        Scenario::OneDocument => one_document(),
    }
}

/// Add every kind of measure, then finish twice, counting the lines written at each step.
fn one_document() -> Result<Value, Failure> {
    let written = Shared::default();
    let adapter = crate::outbound::evidence::factory::bmf(
        Box::new(written.clone()),
        Box::new(std::io::sink()),
    );
    let language = "python".into();
    let lines = || {
        written
            .0
            .lock()
            .map(|bytes| bytes.iter().filter(|byte| **byte == b'\n').count())
            .map_err(|_poisoned| Refusal::Unexpected)
    };
    adapter
        .costs(&language, &sample::costs())
        .and_then(|()| adapter.verification(&language, &sample::timings()))
        .and_then(|()| adapter.growth(&language, &sample::growth()))
        .map_err(|_refused| Refusal::Unexpected)?;
    let before = lines()?;
    adapter.finish().map_err(|_refused| Refusal::Unexpected)?;
    let finished = lines()?;
    if adapter.finish().is_ok() {
        return Err(Refusal::SecondDocument);
    }
    Ok([before, finished, lines()?])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// A destination whose bytes stay readable after the adapter took it.
#[derive(Clone, Default)]
struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Shared {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_poisoned| std::io::Error::other("poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
