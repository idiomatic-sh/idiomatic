//! The typescript language's outbound ports: the parser library it reads with, the blocks other formats carry, the bytes the notes of a
//! file are written as, and the manifests.

pub(crate) mod embedded;
pub(crate) mod manifest_reading;
pub(crate) mod notes;
pub(crate) mod syntax;
