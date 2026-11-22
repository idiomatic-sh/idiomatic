//! The `fact_tables` concept's `keying` part: a table's indexes, one for each key column its relation's catalog item
//! declares (the column it is filled by, then the columns it is read by), built once when the table is finished.

pub(crate) mod service;
