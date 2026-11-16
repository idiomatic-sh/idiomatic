//! The concrete stream destination answers the publication contract.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("BMF destination honors publication contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::evidence::factory::bmf(
        Box::new(std::io::sink()),
        Box::new(std::io::sink()),
    ));
    Ok(())
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

#[test]
fn every_measure_of_one_run_is_one_document_written_at_finish() {
    use crate::outbound::evidence::fixtures::sample;
    use crate::outbound::evidence::interface::EvidencePort as _;
    let written = Shared::default();
    let adapter = crate::outbound::evidence::factory::bmf(
        Box::new(written.clone()),
        Box::new(std::io::sink()),
    );
    let language = "python".into();
    let lines = || {
        String::from_utf8(
            written
                .0
                .lock()
                .map(|bytes| bytes.clone())
                .unwrap_or_default(),
        )
        .unwrap_or_default()
        .lines()
        .count()
    };
    assert!(adapter.costs(&language, &sample::costs()).is_ok());
    assert!(adapter.verification(&language, &sample::timings()).is_ok());
    assert!(adapter.growth(&language, &sample::growth()).is_ok());
    assert_eq!(lines(), 0, "nothing is written before finish");
    assert!(adapter.finish().is_ok());
    assert_eq!(lines(), 1, "finish writes exactly one document");
    assert!(adapter.finish().is_err(), "a second document is refused");
    assert_eq!(lines(), 1, "and nothing more is written");
}
