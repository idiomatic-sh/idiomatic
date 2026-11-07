//! The `repository` port's types: what the port hands back.

use crate::domain::paths::types::{Entry, Look};

/// What is in a directory, in no order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entries(pub Vec<Entry>);

/// What was looked at for a name in each directory from one up to the top, nearest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Looks(pub Vec<Look>);
