//! The types of the `key_facts` concept: what the keys of a file are built through.

use crate::domain::constant_keys::types::{KeyClass, KeyWord};
use crate::domain::literal_facts::types::TextBuffer;
use ryo::languages::{FileWords, RetainedKeyIndex};

/// The keys of a file being built: the classes found so far, the file's words and a text buffer.
#[derive(Debug)]
pub(crate) struct KeyBuild<'b> {
    /// The classes by node id, and the key in hand.
    pub index: &'b mut RetainedKeyIndex<KeyClass, KeyWord>,
    /// The words of the file: a literal's value is one.
    pub words: &'b mut FileWords,
    /// A text built for one literal.
    pub text: &'b mut TextBuffer,
}
