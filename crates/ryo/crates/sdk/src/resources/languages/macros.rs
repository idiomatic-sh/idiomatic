//! The macros of `languages`: `reads!`, which declares how a language reads a relation; the statements it is made of; `fills!`,
//! which declares how a language fills a relation for the engine; `link!`, which links a language; and `catalog_names`, which names
//! the catalog in the SDK.

pub(super) mod catalog;
mod constants;
/// `fills!`, which declares how a language fills a relation for the engine.
mod fills;
/// `link!` and the function it calls, which build a language from its parts.
pub(super) mod link;
/// `reads!`, which declares how a language reads a relation.
mod reads;
mod statements;
