//! `query`: one module per entry.

#![allow(
    clippy::pub_use,
    reason = "a resource module is where the SDK exposes what its verbs use: re-exports are allowed in the resource modules only"
)]

pub mod describe;
pub mod run;
pub mod validate;

pub use describe::entry::describe;

pub use describe::types::DescribeReport;

pub use describe::types::Production;

pub use describe::types::ProductionName;

pub use describe::types::Form;

pub use describe::errors::DescribeError;

pub use run::entry::run;

pub use validate::entry::validate;

pub use crate::domain::query::errors::ValidateError;

pub use crate::domain::query::types::QuerySpan;

pub use crate::domain::query::types::ByteOffset;

pub use validate::types::StepCount;

pub use crate::domain::relation::types::RelationName;

pub use crate::domain::operator::types::OperatorName;

pub use validate::schemas::RyoQlQuerySchemaError;

pub use crate::domain::query::types::RyoQlQuery;

pub use validate::types::ValidateRequest;

pub use validate::types::ValidateReport;

pub use run::types::RunRequest;

pub use run::types::RunReport;

pub use run::types::Heading;

pub use crate::domain::cell::types::Cell;
pub use crate::domain::cell::types::Slot;

pub use run::types::Row;

pub use run::types::RowLimit;

pub use run::types::RowCount;

pub use run::types::DEFAULT_LIMIT;

pub use run::errors::RunError;
