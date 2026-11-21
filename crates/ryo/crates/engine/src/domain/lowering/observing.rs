//! The `observing` part of the lowering: the rows a plan reads that are not a file's relations, lowered as rows of the file
//! the engine scans. A relation the host declares is a constant table (a row that names a file is a row of that file
//! alone); a bound observation is one definition whose branches are its producers' observed steps; and a step after the
//! files whose every input is the file's rows, or the same rows for every file, keyed by the file where two of the file's
//! row sets meet, is a step of the file. Nothing here knows a rule, a rule's text or a relation's name.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
