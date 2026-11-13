//! One file's session: the one walk that serves every root, then the steps the demanded rules reach, each the first time it is
//! needed, reading relations through the facts port only when a step still has rows to join them with.

pub mod service;
pub mod types;
