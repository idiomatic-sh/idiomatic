//! The `extras` service: free functions named by verb over the domain's types, with no IO and no ports. It says which extras carry
//! text of the stylesheet a relation reads.

#[cfg(any(test, feature = "fixtures"))]
use super::types::{Content, Extra};

/// What `extra` is to the readers.
#[cfg(any(test, feature = "fixtures"))]
#[must_use]
pub(crate) const fn content(extra: Extra) -> Content {
    match extra {
        Extra::Comment => Content::Read,
        Extra::ByteOrderMark | Extra::Error => Content::Kept,
    }
}
