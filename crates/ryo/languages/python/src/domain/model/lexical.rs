//! The `lexical` part of the model: what each node of a file is to the language's name rules, the scope it is evaluated in, the
//! names it carries, and the `global` and `nonlocal` declarations and namespace hazards of the file.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

