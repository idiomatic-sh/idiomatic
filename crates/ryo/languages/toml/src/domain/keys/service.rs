//! The `keys` service: how the pass knows a key of a document again. A key is found by the hash of its text and is the same key only
//! when its text is the same, compared in place in the file or in what the library decoded: a key of the same hash and another text
//! takes the next probe. The pass keeps no copy of a key, and allocates nothing once its tables have room.

use super::types::{Child, ChildKey, KeyHash, KeyId, KeyText, Named, Slot};
use ryo::languages::Position;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

/// The hash of a key's text, as `how` hashes it.
#[must_use]
pub(crate) fn hashed(how: KeyHash, text: KeyText<'_>) -> KeyId {
    match how {
        KeyHash::Standard => {
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            KeyId(hasher.finish())
        }
    }
}

/// Says that the key of `slot` names `child`; the part that first said the key stays the one its text is compared with.
pub(crate) fn put(
    children: &mut HashMap<ChildKey, Named>,
    slot: Slot,
    part: Position,
    child: Child,
) {
    let at = children.get(&slot.key).map_or(part, |named| named.at);
    // What the key named before is replaced: the key now names `child`, at the part that first said it.
    let _: Option<Named> = children.insert(slot.key, Named { at, child });
}
