//! The current interpreter's own tests: what it counts while it runs, read through the runtime's fixture accessors. No test
//! outside this directory names it, and the rewrite deletes it whole.
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::cell::Cell;
use crate::boundary_fixture::{
    corpus, demand, engine, prepared, source,
};
mod activation;
mod file_intents;
mod project_phase;
mod publication;
mod scheduler;
mod shared_walk;
