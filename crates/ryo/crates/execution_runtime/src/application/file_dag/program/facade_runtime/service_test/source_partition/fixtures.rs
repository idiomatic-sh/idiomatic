//! The source partition proof cases and the names they run under.
pub(super) mod phase;
use super::super::*;
use crate::application::file_dag::program::admission::{
    FileGeneration, Partition, contributed_proof,
};
use crate::application::file_dag::program::context::RunContext;
use crate::application::file_dag::program::facade_runtime::service::required_partitions;
use crate::application::file_dag::program::kernel_session::KernelSession;
use crate::application::file_dag::program::runtime::service::{
    InvocationOutcome, ProgramExecution,
};
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial, Partials, RuleIndex};
use ryo_execution_domain::domain::table_model::service::count_of;
use ryo_execution_store::application::file_dag::service::open_program_owner;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{HostedNumber, ParameterId, ParameterName, TableKey},
    query_bindings::types::{ParameterType, PlanParameter},
    stored_facts::types::PartialCause,
};
use std::{cell::Cell as Counter, rc::Rc};

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
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
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

pub(crate) type Input = SourceCase;

pub(crate) type Value = &'static str;

pub(crate) type Failure = OperatingError;
