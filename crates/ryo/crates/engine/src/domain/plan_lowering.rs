//! The `plan_lowering` concept: a pack's rule programs (as the query language's parser read them) lowered to one finish plan, then the planner's passes over it, in
//! their one order: `expansion` (every rule's query, with the fragments it imports inlined, as steps shared across
//! rules), `semijoins`, `counts`, `axes`. Each pass has the boundary of one stage of the prototype's chain
//! (`lower6d.py`, `pass_semi.py`, `pass_count.py`, `pass_axis.py`), so each can be compared with it on its own.

pub(crate) mod axes;
pub(crate) mod counts;
pub mod errors;
pub(crate) mod expansion;
pub(crate) mod filters;
#[cfg(test)]
pub(crate) mod fixtures;
pub(crate) mod given;
pub(crate) mod readers;
pub(crate) mod operators;
pub(crate) mod semijoins;
pub mod service;
pub mod types;

