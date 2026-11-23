//! The `readers` part: what the planner passes ask of a plan's readers (the prototype's `plan_util.py`): whether no
//! reader of a step can tell a row from its copies, whether any reads a column of it, and which aliases an expression
//! reads.

pub(crate) mod service;
pub(crate) mod types;

