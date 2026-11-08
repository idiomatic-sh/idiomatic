//! The source partition proof cases and the names they run under.
use super::super::*;

#[derive(Clone, Copy)]
pub(super) enum SourceCase {
    File,
    ProjectRefusal,
    Project,
    FileInvocation,
    ProjectInvocation,
    FileAsProject,
    ProjectAsFile,
    ForeignWalkGeneration,
    Partial,
    Hosted,
    Parameter,
    FrontierRefusal,
    FrontierScope,
}
pub(super) const CASES: &[Case<SourceCase, &'static str, OperatingError>] = &[
    Case {
        name: CaseName("a_file_invocation_accepts_its_required_generation_owner"),
        word: Word::Succeeds,
        input: || SourceCase::FileInvocation,
        expect: Expect::Value("scope-admitted"),
    },
    Case {
        name: CaseName("a_project_invocation_accepts_its_explicit_project_stage"),
        word: Word::Succeeds,
        input: || SourceCase::ProjectInvocation,
        expect: Expect::Value("scope-admitted"),
    },
    Case {
        name: CaseName("a_file_invocation_cannot_select_the_project_walk_stage"),
        word: Word::Refuses,
        input: || SourceCase::FileAsProject,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_project_invocation_cannot_borrow_a_file_walk_owner"),
        word: Word::Refuses,
        input: || SourceCase::ProjectAsFile,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_file_walk_from_another_generation_cannot_execute"),
        word: Word::Refuses,
        input: || SourceCase::ForeignWalkGeneration,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("one_file_frontier_certificate_cannot_supply_two_file_request"),
        word: Word::Refuses,
        input: || SourceCase::FrontierRefusal,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("matching_frontier_certificate_retains_actual_request"),
        word: Word::Succeeds,
        input: || SourceCase::FrontierScope,
        expect: Expect::Value("frontier-scope"),
    },
    Case {
        name: CaseName("exact_file_source_is_admitted"),
        word: Word::Succeeds,
        input: || SourceCase::File,
        expect: Expect::Value("complete"),
    },
    Case {
        name: CaseName("one_file_source_cannot_be_retagged_as_two_file_project"),
        word: Word::Refuses,
        input: || SourceCase::ProjectRefusal,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("declared_project_source_uses_actual_request"),
        word: Word::Succeeds,
        input: || SourceCase::Project,
        expect: Expect::Value("complete"),
    },
    Case {
        name: CaseName("physical_partial_source_never_becomes_complete"),
        word: Word::Succeeds,
        input: || SourceCase::Partial,
        expect: Expect::Value("partial"),
    },
    Case {
        name: CaseName("hosted_output_broadcasts_from_request_owned_population"),
        word: Word::Succeeds,
        input: || SourceCase::Hosted,
        expect: Expect::Value("invariant"),
    },
    Case {
        name: CaseName("rows_output_broadcasts_from_request_owned_parameter"),
        word: Word::Succeeds,
        input: || SourceCase::Parameter,
        expect: Expect::Value("invariant"),
    },
];
