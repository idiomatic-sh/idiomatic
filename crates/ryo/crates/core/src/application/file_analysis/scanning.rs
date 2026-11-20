//! The files of a run scanned by the new engine: its rule sets loaded once, each file scanned in the worker that took its job,
//! its rows rendered as the old engine's are and streamed in the order of the files, or compared with the old engine's.
pub(crate) mod service;
