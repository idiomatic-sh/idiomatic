//! One collection: each relation the language reads from the manifests of a project and the links between its files, in order.

pub(crate) mod analyzed_source;
pub(crate) mod file_package;
pub(crate) mod package_dependency;
pub(crate) mod source_dependency;
pub(crate) mod workspace_member;

pub(crate) mod module_identity;
pub(crate) mod project_namespace_membership;

pub(crate) mod manifest_diagnostic;
