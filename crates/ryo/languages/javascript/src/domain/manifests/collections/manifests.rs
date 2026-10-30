//! One collection: each relation the language reads from the package manifests and the links between the files of a project, in order.

pub(crate) mod analyzed_source;
pub(crate) mod file_package;
pub(crate) mod package_dependency;
pub(crate) mod source_dependency;
pub(crate) mod workspace_member;
