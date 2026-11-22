//! The constants of the model's `names` part: the two codes that are not a name of the file, the word of the empty text, and
//! the hash's two numbers.

use super::types::{NameCode, NameHash};
use ryo::languages::Symbol;

/// The word of the empty text: word 0 of every run, by the facts contract, and no fixed name's.
pub(crate) const EMPTY_WORD: Symbol = Symbol(0);

/// The code of no name: a node that carries none.
pub(crate) const NO_NAME: NameCode = NameCode(u32::MAX);

/// The code of the names a wildcard import binds: every name of another module, none of them known here.
pub(crate) const UNKNOWN_NAME: NameCode = NameCode(u32::MAX - 1);

/// Where the hash of a text starts (the 64-bit FNV offset basis), before its length is mixed in.
pub(crate) const HASH_START: NameHash = NameHash(0xcbf2_9ce4_8422_2325);

/// What the hash is multiplied by after each eight bytes of the text (the 64-bit golden-ratio constant, odd).
pub(crate) const HASH_STEP: NameHash = NameHash(0x9e37_79b9_7f4a_7c15);

/// The bit of a name token's code that says the code is a fixed text of the language (a keyword, by its token kind): the
/// same text in every file of a run, interned in no file's table.
pub(crate) const FIXED_TEXT: u32 = 1 << 31;
