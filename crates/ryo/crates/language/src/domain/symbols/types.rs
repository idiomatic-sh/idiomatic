// Symbols is the engine's table of one file's names, handed to every language: its one field is a private, crate-private `SymbolsEngine` the crate reaches through `AsRef`/`AsMut`, so a language reads and writes names only through the SDK's functions and cannot corrupt the index; a public field would hand it the table's invariants.
//! The types of the `symbols` concept: the names of a file and what readers note while reading it.

use hashbrown::HashTable;

use ryo_relations::domain::relations::types::{
    Bytes, NameNote, NamePlace, Offset, TextHash, Words,
};
use ryo_relations::domain::terms::types::Symbol;

/// What readers keep between the visits of one file, so that a row which depends on elements
/// visited after it is written once the file is read: values indexed by the number of a name, and
/// a list of words. Cleared with the names.
#[derive(Debug, Default)]
pub(crate) struct Notes {
    /// One number for each name that a reader has noted something about, by the name's number.
    pub indexed: Vec<NameNote>,
    /// Words a reader appended, in groups it chooses.
    pub listed: Words,
}

/// The names of one file, each stored once: a symbol is the place of the name in the file's bytes
/// and a hash of its text, so reading a name allocates nothing. The names are cleared with the
/// element table. A name that is not in the file (a piece of a path, a joined list) is derived: its
/// text is kept in `derived`, after the names of the file.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::symbols::types::Symbols(Default::default());
/// ```
#[derive(Debug, Default)]
pub struct Symbols(SymbolsEngine);

/// The symbols a query operator may read and the derived names it may add. The
/// source-name boundary belongs to language fact readers, not query operators.
/// This borrow has no mutable access to `Symbols` and cannot call `intern`.
///
/// ```compile_fail,E0603
/// let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
/// let forged = ryo_language::domain::symbols::types::QuerySymbols(&mut symbols);
/// ```
#[derive(Debug)]
pub struct QuerySymbols<'a>(pub(super) &'a mut Symbols);

/// Engine-held authority to admit transported name places into exactly one
/// live symbol generation. A query only receives QuerySymbols, never this key.
///
/// ```compile_fail,E0423
/// use ryo_language::domain::symbols::types::TransportPermit;
/// let forged = TransportPermit(std::sync::Arc::new(()));
/// ```
///
/// ```compile_fail,E0308
/// use ryo_language::domain::symbols::{service::transport_permit, types::{QuerySymbols, Symbols}};
/// let mut symbols = Symbols::default();
/// let query = QuerySymbols::from(&mut symbols);
/// let stolen = transport_permit(query.as_ref());
/// ```
#[derive(Debug)]
pub struct TransportPermit(pub(super) std::sync::Arc<()>);

impl<'a> From<&'a mut Symbols> for QuerySymbols<'a> {
    fn from(symbols: &'a mut Symbols) -> Self {
        Self(symbols)
    }
}

impl AsRef<Symbols> for QuerySymbols<'_> {
    fn as_ref(&self) -> &Symbols {
        &*self.0
    }
}

/// The immutable byte owner of one interned symbol, independent of insertion order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SymbolStorage {
    /// Offsets refer to the file's source bytes.
    Source,
    /// Offsets refer to the symbol pool's derived bytes.
    Derived,
}

/// The names of one file as the engine keeps them: where each is, its hash, the index that finds it, the derived text, the notes, the
/// vocabulary and the places of the names that left their file.
#[derive(Debug, Default)]
pub(crate) struct SymbolsEngine {
    /// Changes on clear, so a transport key cannot outlive a file generation.
    pub transport_generation: std::sync::Arc<()>,
    /// Where each name starts, interpreted using its corresponding storage owner.
    pub start: Vec<Offset>,
    /// Where each name ends.
    pub end: Vec<Offset>,
    /// The hash of each name's text.
    pub hash: Vec<TextHash>,
    /// The symbols by the hash of their text, so a name is found without a string key.
    pub index: HashTable<Symbol>,
    /// The byte owner for each symbol; source and derived names may be interleaved.
    pub storage: Vec<SymbolStorage>,
    /// The text of the derived names.
    pub derived: Bytes,
    /// What readers noted while reading the file.
    pub notes: Notes,
    /// The name of each number a language gives names by (its kinds, its refinements), made the
    /// first time it is asked for in a file; [`NO_SYMBOL`] before.
    pub vocabulary: Vec<Symbol>,
    /// Where the text of each name that left its file is, by the number of the place: the places of every table the steps after
    /// the files were read work on.
    pub places: Vec<NamePlace>,
}

impl AsRef<SymbolsEngine> for Symbols {
    /// The names as the engine keeps them, for the crate to read.
    fn as_ref(&self) -> &SymbolsEngine {
        &self.0
    }
}

impl AsMut<SymbolsEngine> for Symbols {
    /// The names as the engine keeps them, for the crate to write.
    fn as_mut(&mut self) -> &mut SymbolsEngine {
        &mut self.0
    }
}

impl From<SymbolsEngine> for Symbols {
    /// The names `engine` keeps.
    fn from(engine: SymbolsEngine) -> Self {
        Self(engine)
    }
}

/// Whether two frame permits, or a permit and a pool, name the same live symbol generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    /// They name the same live generation.
    Same,
    /// They name different generations: one is stale or foreign.
    Different,
}

/// A borrowed run of bytes: the bytes of a file, the text of a name, a message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ByteSlice<'a>(pub &'a [u8]);
