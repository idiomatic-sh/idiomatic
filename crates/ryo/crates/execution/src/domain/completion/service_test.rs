//! Physical completeness survives merging and file-frontier conversion.
use super::{
    service::{file_frontier_coverage, merge_coverage},
    types::CoverageState,
};
use crate::domain::operating::types::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run, selects},
    domain::records::types::{FrontierAffected, FrontierCoverage},
};
struct Input {
    coverage: CoverageState,
    expected: FrontierCoverage,
}
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete"),
        word: Word::Succeeds,
        input: || Input {
            coverage: CoverageState::Complete,
            expected: FrontierCoverage::Complete,
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("current_file"),
        word: Word::Succeeds,
        input: || Input {
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(0))],
            },
            expected: FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile]),
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("repository"),
        word: Word::Succeeds,
        input: || Input {
            coverage: CoverageState::Partial { files: vec![None] },
            expected: FrontierCoverage::Partial(vec![FrontierAffected::Repository]),
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("duplicate_causes"),
        word: Word::Succeeds,
        input: || {
            let mut causes = vec![FrontierAffected::CurrentFile, FrontierAffected::Repository];
            causes.sort_unstable();
            Input {
                coverage: merge_coverage(
                    CoverageState::Partial {
                        files: vec![Some(FileId(0)), Some(FileId(0))],
                    },
                    CoverageState::Partial { files: vec![None] },
                ),
                expected: FrontierCoverage::Partial(causes),
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("foreign_file"),
        word: Word::Refuses,
        input: || Input {
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(1))],
            },
            expected: FrontierCoverage::Complete,
        },
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_ref() == "file frontier carries a foreign physical partition"),
            )
        }),
    },
    Case {
        name: CaseName("empty_partial"),
        word: Word::Refuses,
        input: || Input {
            coverage: CoverageState::Partial { files: vec![] },
            expected: FrontierCoverage::Complete,
        },
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_ref() == "partial frontier has no affected physical partition"),
            )
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let actual = file_frontier_coverage(input.coverage, FileId(0))?;
    assert_eq!(actual, input.expected);
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
