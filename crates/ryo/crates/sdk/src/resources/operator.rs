//! `operator`: one module per entry.

#![allow(
    clippy::pub_use,
    reason = "a resource module is where the SDK exposes what its verbs use: re-exports are allowed in the resource modules only"
)]

pub mod describe;
pub mod list;

pub use describe::entry::describe;

pub use list::entry::list;

pub use list::types::ListReport;

pub use list::types::OperatorSummary;

pub use crate::domain::operator::types::Purpose;

pub use list::errors::ListError;

pub use describe::types::DescribeRequest;

pub use describe::types::DescribeReport;

pub use describe::types::OperatorDescription;

pub use describe::types::FieldDescription;

pub use describe::types::FieldName;

pub use describe::types::Requirement;

pub use describe::types::Choice;

pub use crate::domain::operator::types::Example;

pub use describe::errors::DescribeError;
