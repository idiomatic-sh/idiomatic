//! The `defines` relation of the language: every name a file binds: functions and methods, classes, imported names, assigned names,
//! parameters, the names of `except ... as` and of `case` captures, type aliases and type parameters, each in the scope that binds it
//! once `global` and `nonlocal` are read. `from m import *` defines names it cannot list: its name is the unknown value of a
//! wildcard import.








/// Reads the `defines` rows of one Python file from its syntax tree.
pub(crate) struct DefinesItem;
