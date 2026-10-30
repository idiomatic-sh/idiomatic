//! The types of the `keys` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{Count, Position};

/// What a key names within its mapping, once the document has said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Child {
    /// A mapping, by the element that first opens it.
    Table(Position),
    /// An array of tables: its entry, how many members it has, and its last member.
    Sequence {
        /// The part of a key that opened it.
        entry: Position,
        /// How many members it has.
        members: Count,
        /// The last member, which a longer header extends.
        last: Position,
    },
    /// A value, which nothing extends.
    Value,
}

/// A key within the mapping that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ChildKey {
    /// The mapping.
    pub owner: Position,
    /// The key, decoded, by its hash.
    pub key: KeyId,
    /// Which of the keys of the same hash it is: the first is 0, the next whose text differs 1, and so on.
    pub probe: Probe,
}

/// Which of the keys of one hash a key is, in the order the file says them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Probe(pub u32);

/// What a key names within its mapping, and the part of a key that first said it, whose text a key of the same hash is compared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Named {
    /// The part of a key that first said it.
    pub at: Position,
    /// What it names.
    pub child: Child,
}

/// The key of a mapping a part of a key names, and what it names there when the file has said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    /// The key.
    pub key: ChildKey,
    /// What it names; nothing yet.
    pub child: Option<Child>,
}

/// How a key's text is hashed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum KeyHash {
    /// By the standard hasher.
    #[default]
    Standard,
    /// So that every key has the same hash, for the test that two keys of one hash and two texts are two keys.
    #[cfg(test)]
    Same,
}

/// A key's number in one file, the hash of its text: the same text, the same number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KeyId(pub u64);

/// The text of a key of one file, decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KeyText<'a>(pub &'a str);
