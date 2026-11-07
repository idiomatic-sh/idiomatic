//! `relation`: one module per entry.

#![allow(
    clippy::pub_use,
    reason = "a resource module is where the SDK exposes what its verbs use: re-exports are allowed in the resource modules only"
)]

pub mod describe;
pub mod list;

pub use describe::entry::describe;

pub use list::entry::list;

pub use list::types::ListReport;

pub use list::types::RelationSummary;

pub use crate::domain::relation::types::Holds;

pub use list::errors::ListError;

pub use describe::types::DescribeRequest;

pub use describe::types::DescribeReport;

pub use describe::types::RelationDescription;

pub use describe::types::ColumnDescription;

pub use describe::types::LanguageDescription;

pub use crate::domain::language::types::LanguageName;

pub use describe::types::Reading;

pub use crate::domain::relation::types::ColumnName;

pub use describe::types::ColumnKind;

pub use describe::types::Reason;

pub use describe::types::KindWord;

pub use describe::types::UnknownName;

pub use describe::errors::DescribeError;
