//! The `inputs` concept: the files each pathspec of a tool's inputs names in the repository's one listing, and the
//! digest they come to: one hash over each file's path and stamp, in path order. The stamps are what is kept: a file
//! whose size and modification time did not move keeps its stamp unread, so an unchanged input reads no file.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
