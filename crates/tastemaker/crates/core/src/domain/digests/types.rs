//! The types of the `digests` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a list of byte strings comes to: the hex of the first sixteen bytes of their hash.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(pub Box<str>);
