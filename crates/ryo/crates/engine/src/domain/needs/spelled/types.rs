//! The types of the `spelled` part: the texts a rule set's needs ask a file to spell, and a file's answers.

/// The texts the needs ask for, numbered as they are met, found once a worker.
#[derive(Debug, Default)]
pub(crate) struct SpelledTexts {
    /// The number of each text.
    pub(crate) numbers: hashbrown::HashMap<Box<str>, u32>,
    /// The shortest text's length in bytes, and the longest's: a token of another length is no text asked.
    pub(crate) lengths: (usize, usize),
}

/// What a file spells of the texts asked, as bits by the texts' numbers.
#[derive(Debug, Default)]
pub(crate) struct SpelledFile {
    /// How many of the texts the bits answer: none at the file's start.
    pub(crate) known: usize,
    /// The texts a token of the file's own tree spells.
    pub(crate) own: Vec<u64>,
    /// The texts a token of the file spells, the embedded segments' too.
    pub(crate) every: Vec<u64>,
    /// Whether a token of the file holds a byte past ASCII: a language may read such a name as another text (a
    /// normalised one), so the file is taken to spell every text asked.
    pub(crate) unsure: bool,
}
