//! The macros of `languages`: `reads!`, which declares how a language reads a relation; the statements it is made of; `link!`, which
//! links a language; and `catalog_names`, which names the catalog in the SDK.

pub(super) mod catalog;
/// `link!` and the function it calls, which build a language from its parts.
pub(super) mod link;
/// `reads!`, which declares how a language reads a relation.
mod reads;
mod statements;
