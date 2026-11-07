//! The types of the `context` port: plain data, public fields and derives (`Clone`, `Debug`, `PartialEq`), with no constructor and no inherent impl, and no handle, file, lock or `dyn`: a handle is a trait in interface.rs.
use std::path::PathBuf;

/// Every file the corpus tracks, as paths relative to its root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusFiles<'a>(pub &'a [PathBuf]);
