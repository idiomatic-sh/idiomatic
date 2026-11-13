//! Plain data of the profile tools port.
use std::path::PathBuf;

/// The heap dump files one run wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeapDumps(pub Vec<PathBuf>);

/// The bytes one file of a profile run holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileContents(pub Vec<u8>);
