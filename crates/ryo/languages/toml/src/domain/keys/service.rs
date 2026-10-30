//! The `keys` service: how the pass knows a key of a document again. A key is found by the hash of its text and is the same key only
//! when its text is the same, compared in place in the file or in what the library decoded: a key of the same hash and another text
//! takes the next probe. The pass keeps no copy of a key, and allocates nothing once its tables have room.

use super::types::{Child, ChildKey, KeyHash, KeyId, KeyText, Named, Probe, Slot};
use crate::domain::syntax::service::{kind, written};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Tree};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

/// The text of the key the part at `at` says: a bare key is itself, a quoted one is what the library decoded.
fn key_text<'a>(tree: &Tree<'a>, at: Position) -> KeyText<'a> {
    KeyText(if kind(tree, at) == Kind::Token(Token::UnquotedKey) {
        written(tree, at).0
    } else {
        languages::decoded_at(tree.elements, at).map_or_else(|| written(tree, at).0, |text| text.0)
    })
}

/// The key the part at `at` says, as a name.
pub(crate) fn key_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> languages::Symbol {
    if kind(tree, at) == Kind::Token(Token::UnquotedKey) {
        return languages::text_at(tree.elements, at);
    }
    let name = languages::begin_name(symbols);
    languages::push_text(symbols, languages::NameText(key_text(tree, at).0));
    languages::end_name(symbols, tree.source, name)
}

/// The hash of a key's text, as `how` hashes it.
#[must_use]
pub(crate) fn hashed(how: KeyHash, text: KeyText<'_>) -> KeyId {
    match how {
        KeyHash::Standard => {
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            KeyId(hasher.finish())
        }
        #[cfg(test)]
        KeyHash::Same => KeyId(0),
    }
}

/// The slot of the key the part at `part` says within the mapping `owner`. A key is found by the hash of its text, and a key of the
/// same hash is the same key only when its text is the same, compared in place: a key of the same hash and another text takes the
/// next probe. The pass keeps no copy of a key, and allocates nothing once its tables have room.
pub(crate) fn slot(
    tree: &Tree<'_>,
    children: &HashMap<ChildKey, Named>,
    how: KeyHash,
    owner: Position,
    part: Position,
) -> Slot {
    let text = key_text(tree, part);
    let key = hashed(how, text);
    let mut probe = Probe(0);
    loop {
        let at = ChildKey { owner, key, probe };
        match children.get(&at) {
            None => {
                return Slot {
                    key: at,
                    child: None,
                };
            }
            Some(named) if key_text(tree, named.at) == text => {
                return Slot {
                    key: at,
                    child: Some(named.child),
                };
            }
            Some(_) => probe.0 += 1,
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
