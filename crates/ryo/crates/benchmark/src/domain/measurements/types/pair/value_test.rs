//! Naming preserves each original certified observation and contestant order.
use super::value::Pair;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::{Measurement, ResourceObservation, SampleName};

const CASES: &[Case<(), [u128; 2], ()>] = &[Case {
    name: CaseName("naming preserves both observations"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([3, 7]),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        let observation = |milliseconds| Measurement {
            wall: std::time::Duration::from_millis(milliseconds),
            resources: ResourceObservation::WallOnly,
            report: crate::domain::diagnostics::types::RunReport {
                files: vec![],
                skipped: Vec::new(),
                coverage: None,
                diagnostics: vec![],
            },
        };
        let pair = Pair {
            reference: observation(3),
            candidate: observation(7),
        };
        assert_eq!(pair.reference().wall.as_millis(), 3);
        assert_eq!(pair.candidate().wall.as_millis(), 7);
        let [reference, candidate] =
            pair.samples([SampleName(String::new()), SampleName(String::new())]);
        assert!(reference.name().0.is_empty());
        assert!(candidate.name().0.is_empty());
        Ok([
            reference.measurement().wall.as_millis(),
            candidate.measurement().wall.as_millis(),
        ])
    });
}
