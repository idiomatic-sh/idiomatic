//! `linter`: one module per entry.

#![allow(
    clippy::pub_use,
    reason = "a resource module is where the SDK exposes what its verbs use: re-exports are allowed in the resource modules only"
)]

pub mod lint;

pub use lint::entry::lint;

pub use lint::entry::lint_with;

pub use lint::types::Linter;

pub use crate::domain::capacity::types::WorkerPool;

pub use lint::errors::LintError;

pub use lint::errors::ProviderCode;

pub use lint::schemas::GlobSchemaError;

pub use lint::schemas::RuleIdSchemaError;

pub use lint::types::Cache;

pub use lint::types::CaseSource;
pub use lint::types::ExecutablePermission;

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

pub use lint::entry::dependency_fingerprint;
pub use lint::types::LintReport;
pub use lint::types::Skipped;
pub use lint::types::{DependencyFingerprint, ProjectDependency, ProjectDependencyState};

pub use lint::types::LintRequest;

pub use crate::domain::capacity::types::MemoryBudget;

pub use lint::types::ParameterValue;

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

pub use lint::types::Column;

pub use lint::types::FindingKey;

pub use lint::types::Line;

pub use lint::types::Place;

pub use lint::types::Workers;

pub use lint::types::FlagState;

pub use lint::types::ParameterNumber;

pub use lint::types::ParameterNumbers;

pub use lint::types::Message;

pub use lint::types::ParameterName;

pub use lint::types::ParameterText;

pub use lint::types::ParameterCell;

pub use lint::types::ParameterTable;

pub use crate::domain::file::types::ReadFailure;

pub use crate::domain::file::types::RelativePath;

pub use crate::domain::cell::types::HostText;

pub use crate::domain::cell::types::HostNumber;

pub use lint::types::Corpus;

pub use lint::types::FindingColumn;

pub use crate::domain::relation::types::ColumnName;
