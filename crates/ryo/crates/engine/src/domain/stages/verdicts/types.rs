//! The types of the `verdicts` part. A test of one text cell against constants holds or fails by the cell's TEXT alone,
//! and a text cell is a word: so the test is decided once for each word and kept, two bits a word. A word of the run
//! (a kind's name, a field's, a fixed name, a text of the pack) is the same text in every file, so its verdict is kept
//! for the worker's life; a file's own word is another text in the next file, so its verdict is kept for the file.

use crate::domain::code::types::PoolAt;
use crate::domain::indexes::types::FileStamp;
use crate::domain::steps::types::TestOperation;
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::patterns::types::TextGlobSet;
use ryo_relations::domain::relations::types::Count;

/// What a test of a text is against, besides the text: the constants its verdict depends on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Against {
    /// Nothing but the test itself: a regular expression is named by the test.
    Nothing,
    /// One constant cell: a glob, an affix.
    Cell(CellWord),
    /// A set of constants, by its place in the pool.
    Set(PoolAt),
}

/// A test of a text against constants: what its verdicts are kept by. Two words of the code that make the same test
/// against the same constants share them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextTestKey {
    /// The test.
    pub(crate) test: TestOperation,
    /// What it is against.
    pub(crate) against: Against,
}

/// Thirty-two words' verdicts, two bits each: nought for a word not decided yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VerdictBits(pub(crate) u64);

/// What is known of a word under one test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordKnown {
    /// Nothing yet: the test is run on the word's text, once.
    NotYet,
    /// The test fails for it.
    Fails,
    /// The test holds for it.
    Holds,
}

/// Where a word's verdict is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordPlace {
    /// Among the run's words, at the word.
    Run(Count),
    /// Among the file's own words, at the word less the run's words.
    Own(Count),
}

/// The verdicts of one test, by word.
#[derive(Debug)]
pub(crate) struct WordVerdicts {
    /// The test they are of.
    pub(crate) key: TextTestKey,
    /// Of the run's words, by the word: kept for the worker's life.
    pub(crate) base: Vec<VerdictBits>,
    /// Of the file's own words, by the word less the run's words: kept for the file `stamp` names.
    pub(crate) own: Vec<VerdictBits>,
    /// The file the own words' verdicts are of.
    pub(crate) stamp: FileStamp,
    /// Of a glob test: its globs, compiled when the test was first asked.
    pub(crate) globs: Option<TextGlobSet>,
}

/// Every text test's verdicts: a worker's, kept between files and never freed. Its size is bounded by the code: one
/// entry for each different test of a text against constants, and for each two bits a word.
#[derive(Debug, Default)]
pub(crate) struct TextVerdicts {
    /// The tests' verdicts, in the order first asked.
    pub(crate) tests: Vec<WordVerdicts>,
    /// The place of each test among them, by its key.
    pub(crate) places: hashbrown::HashTable<Count>,
}
