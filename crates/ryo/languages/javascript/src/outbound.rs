//! The javascript language's outbound ports: the parser library it reads with, the readers of its package manifests, and the bytes its
//! notes of a file are written as.

pub(crate) mod embedded;
pub(crate) mod manifest_reading;
pub(crate) mod notes;
pub(crate) mod syntax;
