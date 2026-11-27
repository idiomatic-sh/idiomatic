//! What the executor reads of a numbered file besides its columns: the words the reader declares public, and the dictionary
//! of a file's symbols.
use super::finish::Run;
use std::collections::HashMap;
use crate::languages::{self as l, ByteSlice};

pub(crate) struct ReaderFacts {
    pub(crate) file: u32,
    pub(crate) fills_made: u64,
    pub text: Vec<u32>,
    pub(crate) projects_words: bool,
    pub(crate) projects_fields: bool,
}

impl ReaderFacts {
    pub fn new(file: u32) -> Self {
        Self {
            file,
            fills_made: 0,
            text: Vec::new(),
            projects_words: true,
            projects_fields: true,
        }
    }
}

/// Reserve distinct identities for missing text and unresolved names. They are
/// outside the interner, so an ordinary string can have the same displayed bytes.
pub(crate) fn public_words(run: &mut Run, reasons: &[l::UnknownName]) -> HashMap<u32, u32> {
    let mut unknown = HashMap::new();
    if run.absent_word.is_none() {
        let absent = u32::try_from(run.texts.len()).expect("word id");
        run.texts.push(Default::default());
        run.absent_word = Some(absent);
    }
    for reason in reasons {
        let number = u32::try_from(run.texts.len()).expect("word id");
        run.texts.push(format!("unknown: {}", reason.text.0).into_bytes().into());
        run.unresolved_words.push(number);
        unknown.insert(l::unknown_symbol(reason.reason).0, number);
    }
    let mut public: rustc_hash::FxHashMap<_, _> = unknown.iter().map(|(&raw, &id)| (raw, id)).collect();
    for (raw, text) in ryo_relations::domain::terms::service::declared_terms() {
        public.insert(raw.0, run.sym(text.0.as_bytes()));
    }
    run.public_word_ids = std::rc::Rc::new(public);
    unknown
}

pub(crate) fn symbol(run: &mut Run, unknown: &HashMap<u32, u32>, symbols: &l::Symbols, bytes: &[u8], word: l::Symbol) -> u32 {
    if let Some(text) = l::text_of(symbols, ByteSlice(bytes), word) {
        return run.sym(text.0.as_bytes());
    }
    if let Some(&published) = unknown.get(&word.0) {
        return published;
    }
    run.absent_word.expect("reader word table")
}
