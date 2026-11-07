//! `linter`: one module per entry.

#![allow(
    clippy::pub_use,
    reason = "a resource module is where the SDK exposes what its verbs use: re-exports are allowed in the resource modules only"
)]

pub mod lint;

pub use lint::entry::lint;

pub use lint::entry::lint_with;

pub use lint::types::Linter;

/// Which execution strategies a linter may use, for a differential run against the generic evaluation.
#[cfg(feature = "fixtures")]
pub use ryo_execution::inbound::use_cases::prepare::interface::RoutePolicy;
pub use ryo_execution::inbound::use_cases::prepare::interface::RouteWiring;

pub use crate::domain::capacity::types::WorkerPool;

pub use lint::errors::LintError;

pub use lint::errors::ProviderCode;

pub use lint::schemas::GlobSchemaError;

pub use lint::schemas::RuleIdSchemaError;

pub use lint::types::Cache;

pub use lint::types::CaseSource;
/// Edge-observed POSIX executable-bit knowledge, independent of file content.
pub use ryo_values::domain::files::types::ExecutablePermission;

pub use lint::types::SourceText;

pub use crate::domain::cell::types::Cell;
pub use crate::domain::cell::types::Slot;

pub use crate::domain::host::types::HostColumn;

pub use crate::domain::host::types::HostColumnName;

pub use crate::domain::host::types::HostDeclaration;

pub use crate::domain::host::types::HostKind;

pub use lint::types::HostRelation;

pub use crate::domain::host::types::HostRelationName;

pub use lint::types::HostRow;

pub use lint::types::RowNumber;

pub use lint::types::Finding;

pub use lint::types::Glob;

pub use crate::domain::project::types::{
    DependencyBytes, DependencyFingerprint, ProjectDependency, ProjectDependencyState,
};
pub use lint::types::LintReport;
pub use lint::types::Skipped;

pub use crate::domain::language::types::SourceLanguage;
pub use crate::domain::project::types::{
    GivenProjectInventory, ProjectEntryObservation, ProjectEntryState,
    ProjectInventoryCompleteness, ProjectSearchScope,
};
pub use lint::types::LintRequest;

pub use crate::domain::capacity::types::MemoryBudget;

pub use crate::domain::parameter::types::ParameterValue;

pub use lint::types::Port;

pub use lint::types::ProviderName;

pub use lint::types::Rule;

pub use lint::types::RuleId;

pub use lint::types::RuleApplicability;
pub use lint::types::SkipReason;

pub use lint::types::Unknown;

pub use crate::domain::language::types::Languages;

pub use crate::domain::file::types::Unread;

pub use crate::domain::file::types::Unreadable;

pub use crate::domain::place::types::Column;

pub use lint::types::FindingKey;

pub use crate::domain::place::types::Line;

pub use crate::domain::place::types::Place;

pub use lint::types::Workers;

pub use crate::domain::parameter::types::FlagState;

pub use crate::domain::parameter::types::ParameterNumber;

pub use crate::domain::parameter::types::ParameterNumbers;

pub use lint::types::Message;

pub use crate::domain::parameter::types::ParameterName;

pub use crate::domain::parameter::types::ParameterText;

pub use crate::domain::parameter::types::ParameterCell;

pub use crate::domain::parameter::types::ParameterTable;

pub use crate::domain::file::types::ReadFailure;

pub use crate::domain::file::types::RelativePath;

pub use crate::domain::cell::types::HostText;

pub use crate::domain::cell::types::HostNumber;

pub use lint::types::Corpus;

pub use lint::types::FindingColumn;

pub use crate::domain::relation::types::ColumnName;

/// Local typed query input name.
pub use crate::domain::observation::types::ObservationBindingName;
/// Ordered semantic column required by a bound input.
pub use crate::domain::observation::types::ObservationColumn;
/// Name of one bound input column.
pub use crate::domain::observation::types::ObservationColumnName;
/// Name of an explicitly authored producer output.
pub use crate::domain::observation::types::ObservationName;
/// Explicit authored query outputs consumed by another configured rule.
pub use lint::types::ObservationBinding;
/// A selected producer output or an explicitly unavailable query.
pub use lint::types::ObservationProducer;

pub use crate::domain::observation::types::ObservationKind;
