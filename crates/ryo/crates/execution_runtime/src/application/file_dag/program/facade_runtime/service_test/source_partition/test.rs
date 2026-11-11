//! Producer declarations and actual source scope govern ordinary admission.
use super::fixtures::SourceCase;
use super::fixtures::phase::{Admission, Refused, admit};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use std::borrow::Cow;

type Input = SourceCase;
type Value = Admission;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_invocation_accepts_its_required_generation_owner"),
        word: Word::Succeeds,
        input: || SourceCase::FileInvocation,
        expect: Expect::Value(Admission {
            how: "scope-admitted",
            rows: None,
            reads: None,
            partial: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("a_project_invocation_accepts_its_explicit_project_stage"),
        word: Word::Succeeds,
        input: || SourceCase::ProjectInvocation,
        expect: Expect::Value(Admission {
            how: "scope-admitted",
            rows: None,
            reads: None,
            partial: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("a_file_invocation_cannot_select_the_project_walk_stage"),
        word: Word::Refuses,
        input: || SourceCase::FileAsProject,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            reads_before: None,
        }),
    },
    Case {
        name: CaseName("a_project_invocation_cannot_borrow_a_file_walk_owner"),
        word: Word::Refuses,
        input: || SourceCase::ProjectAsFile,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            reads_before: None,
        }),
    },
    Case {
        name: CaseName("a_file_walk_from_another_generation_cannot_execute"),
        word: Word::Refuses,
        input: || SourceCase::ForeignWalkGeneration,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            reads_before: None,
        }),
    },
    Case {
        name: CaseName("one_file_frontier_certificate_cannot_supply_two_file_request"),
        word: Word::Refuses,
        input: || SourceCase::FrontierRefusal,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            reads_before: None,
        }),
    },
    Case {
        name: CaseName("matching_frontier_certificate_retains_actual_request"),
        word: Word::Succeeds,
        input: || SourceCase::FrontierScope,
        expect: Expect::Value(Admission {
            how: "frontier-scope",
            rows: None,
            reads: None,
            partial: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("exact_file_source_is_admitted"),
        word: Word::Succeeds,
        input: || SourceCase::File,
        expect: Expect::Value(Admission {
            how: "complete",
            rows: None,
            reads: Some(1),
            partial: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("one_file_source_cannot_be_retagged_as_two_file_project"),
        word: Word::Refuses,
        input: || SourceCase::ProjectRefusal,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            reads_before: Some(0),
        }),
    },
    Case {
        name: CaseName("declared_project_source_uses_actual_request"),
        word: Word::Succeeds,
        input: || SourceCase::Project,
        expect: Expect::Value(Admission {
            how: "complete",
            rows: None,
            reads: Some(1),
            partial: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("physical_partial_source_never_becomes_complete"),
        word: Word::Succeeds,
        input: || SourceCase::Partial,
        expect: Expect::Value(Admission {
            how: "partial",
            rows: None,
            reads: Some(1),
            partial: Cow::Borrowed(&[Some(FileId(0))]),
        }),
    },
    Case {
        name: CaseName("hosted_output_broadcasts_from_request_owned_population"),
        word: Word::Succeeds,
        input: || SourceCase::Hosted,
        expect: Expect::Value(Admission {
            how: "invariant",
            rows: Some(1),
            reads: Some(0),
            partial: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("rows_output_broadcasts_from_request_owned_parameter"),
        word: Word::Succeeds,
        input: || SourceCase::Parameter,
        expect: Expect::Value(Admission {
            how: "invariant",
            rows: Some(1),
            reads: Some(0),
            partial: Cow::Borrowed(&[]),
        }),
    },
];

/// Runs the row's project phase over its sources.
fn subject(input: Input) -> Result<Value, Failure> {
    admit(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
