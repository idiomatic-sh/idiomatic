//! The python language's outbound ports: the parser library it reads with.

pub(crate) mod format_syntax;
pub(crate) mod manifest_reading;
pub(crate) mod notes;
pub(crate) mod syntax;

/// The pinned syntactic literal parser adapter.
pub(crate) mod literal_expression;
/// Project-established importable module context.
pub(crate) mod module_context;
