//! Neither contestant can certify a case by silently dropping its expected finding.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::verify_cases::fixtures::shard::{
    Refusal, Shape, category, history_len, prepared, report_scope, verify_shard,
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    receipts::fixtures::fake::ReceiptsFake,
};
use std::path::Path;

type Input = Shape;
type Value = usize;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("full report preserves distinct frontend and lint identities"),
        word: Word::Succeeds,
        input: || Shape::FullAgreement,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("matching lint cannot conceal a missing frontend diagnostic"),
        word: Word::Refuses,
        input: || Shape::MissingFrontend,
        expect: Expect::Error(Refusal::CandidateMismatch),
    },
    Case {
        name: CaseName("rule scope preserves parser diagnostics and excludes unrelated rules"),
        word: Word::Succeeds,
        input: || Shape::RuleProjection,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("candidate startup failure executes once before fixture comparisons"),
        word: Word::Refuses,
        input: || Shape::StartupFailure,
        expect: Expect::Error(Refusal::CandidateStartup),
    },
    Case {
        name: CaseName("both contestants establish the independent expectation"),
        word: Word::Succeeds,
        input: || Shape::Agreement,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("candidate omissions cannot certify"),
        word: Word::Refuses,
        input: || Shape::MissingCandidate,
        expect: Expect::Error(Refusal::CandidateMismatch),
    },
    Case {
        name: CaseName("reference omissions cannot certify"),
        word: Word::Refuses,
        input: || Shape::MissingReference,
        expect: Expect::Error(Refusal::ReferenceMismatch),
    },
    Case {
        name: CaseName("empty case sources refuse certification"),
        word: Word::Refuses,
        input: || Shape::NoCases,
        expect: Expect::Error(Refusal::NoCases),
    },
];

fn subject(shape: Input) -> Result<Value, Failure> {
    shard(shape).map_err(|error| category(&error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The paired verification of the prepared case, which executes both contestants.
fn shard(shape: Shape) -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let (mut suite, expected) = prepared(shape);
    let mut reference = LinterFake {
        diagnostics: if matches!(shape, Shape::MissingReference) {
            Vec::new()
        } else {
            vec![expected.clone()]
        },
        ..Default::default()
    };
    let mut candidate = LinterFake {
        diagnostics: if matches!(shape, Shape::MissingCandidate) {
            Vec::new()
        } else {
            vec![expected]
        },
        ..Default::default()
    };
    report_scope(shape, &mut suite, &mut reference, &mut candidate);
    let execution = ExecutionFake {
        status: if matches!(shape, Shape::StartupFailure) {
            2
        } else {
            0
        },
        ..Default::default()
    };
    let result = verify_shard(
        &suite,
        &ReceiptsFake::default(),
        &reference,
        &candidate,
        &execution,
        Path::new("rules"),
    );
    if matches!(shape, Shape::StartupFailure) {
        assert_eq!(history_len(&execution), Some(1));
    } else if !matches!(shape, Shape::NoCases) {
        assert_eq!(history_len(&execution), Some(2));
    } else {
        assert_eq!(
            history_len(&execution),
            Some(0),
            "no cases must launch no contestants"
        );
    }
    result
}
