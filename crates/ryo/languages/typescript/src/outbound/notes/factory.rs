//! Builds the notes adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `postcard` notes adapter.
#[must_use]
pub(crate) fn postcard() -> impl crate::outbound::notes::interface::NotesPort {
    super::adapters::postcard::adapter::NotesPostcardAdapter
}
