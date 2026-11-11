//! The source partition proof cases and the names they run under.
pub(super) mod phase;
use super::super::*;
use crate::application::file_dag::program::admission::{
    service::FileGeneration, service::Partition, service::contributed_proof,
};
use crate::application::file_dag::program::context::service::RunContext;
use crate::application::file_dag::program::facade_runtime::service::required_partitions;
use crate::application::file_dag::program::kernel_session::service::KernelSession;
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

/// The source a row runs its project phase over, or the scope it admits.
#[derive(Clone, Copy, Debug)]
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
