//! The `file_view` service: free functions named by verb over the walked file's view: the borrows a filler reads it by, the
//! positions it visits, and the words its names come out as.

use super::types::RustFile;
use crate::domain::model::types::ReadIndexes;
use ryo::languages::{
    self, ByteSlice, Elements, FileWords, Flag, FileId, Integer, NodeId, NodeTable, ParameterKind, Position, Span, Symbol, Symbols, Tree,
};

/// The borrows a filler reads the file by: its tree, the names it interns, the indexes it builds, each from a different part of the
/// view, so a reader holds them together.
pub(crate) fn split<'a>(file: &'a mut RustFile<'_>) -> (Tree<'a>, &'a mut Symbols, &'a mut ReadIndexes, &'a NodeTable) {
    (
        Tree { elements: &*file.elements, source: file.source, file: file.file },
        &mut *file.symbols,
        &mut *file.model,
        file.table,
    )
}

/// The positions a reader of the file visits, in order: the file's own tree, then the trees its macro calls expand to when the
/// reader reads them.
pub(crate) fn visited(elements: &Elements, expanded: bool) -> impl Iterator<Item = Position> + use<> {
    let own = u32::try_from(languages::positions(elements).count()).unwrap_or(u32::MAX);
    let end = if expanded { languages::end_of(elements).0 } else { own };
    (0..end).map(Position)
}

/// What a row's cell is made of when the file's words are the words of the run: a name the readers interned is the word of its text;
/// a term of the catalog, an unknown reason and the absent name stay what they are.
pub(crate) trait Cell {
    /// The cell for the value, as the file's words give it.
    fn cell(self, symbols: &Symbols, source: ByteSlice<'_>, words: &mut FileWords) -> Self;
}

impl Cell for Symbol {
    fn cell(self, symbols: &Symbols, source: ByteSlice<'_>, words: &mut FileWords) -> Self {
        match languages::text_of(symbols, source, self) {
            Some(text) => Symbol(u32::try_from(languages::facts::intern(words, text).0).unwrap_or(u32::MAX)),
            // A term of the catalog, an unknown reason and the absent name are the same word in every file.
            None => self,
        }
    }
}

/// The values of a row that no word stands for.
macro_rules! plain_cells {
    ($($kind:ty),* $(,)?) => {$(
        impl Cell for $kind {
            fn cell(self, _symbols: &Symbols, _source: ByteSlice<'_>, _words: &mut FileWords) -> Self {
                self
            }
        }
    )*};
}
plain_cells!(NodeId, Span, FileId, Flag, Integer, ParameterKind, languages::DeclarationKind);

/// The cell of a value written for a row.
pub(crate) fn cell<T: Cell>(value: T, symbols: &Symbols, source: ByteSlice<'_>, words: &mut FileWords) -> T {
    value.cell(symbols, source, words)
}
