//! What the readers of a relation ask of a file's syntax tree, so one reader serves the old element table and the numbered node table:
//! the kind, the parent, the children, the extent and the decoded text of an element, and what interns a text as a cell.

use crate::domain::file_view::types::TypeScriptFile;
use ryo::languages::{self, ByteSlice, FileId, FileWords, KindBits, NameText, Offset, Position, Symbol, Symbols, Tree, node_table};

/// A file's syntax tree as the readers see it.
pub(crate) trait Syntax {
    /// The kind of the element at `at`, as the number the table holds.
    fn kind_bits(&self, at: Position) -> KindBits;
    /// The parent of the element at `at`; the root is its own.
    fn parent_of(&self, at: Position) -> Position;
    /// The children of the element at `at`, nodes and tokens in source order.
    fn children_of(&self, at: Position) -> impl Iterator<Item = Position> + '_;
    /// Where the element at `at` starts.
    fn start_of(&self, at: Position) -> Offset;
    /// Where the element at `at` ends.
    fn end_of(&self, at: Position) -> Offset;
    /// The text the library decoded for the token at `at`.
    fn decoded_of(&self, at: Position) -> Option<NameText<'_>>;
    /// The scalar the parser gave the element at `at` under `property`.
    fn property_of(&self, at: Position, property: languages::PropertyId) -> Option<NameText<'_>>;
    /// Whether the element at `at` is a token, a leaf, and not a node.
    fn is_leaf(&self, at: Position) -> bool;
    /// The last element below the element at `at`, in pre-order; `at` itself when it has none.
    fn last_of(&self, at: Position) -> Position;
    /// The place of the element at `at` among its parent's children.
    fn ordinal_of(&self, at: Position) -> languages::ChildOrdinal;
    /// Every element of the file.
    fn positions(&self) -> impl Iterator<Item = Position> + '_;
    /// The bytes of the file.
    fn bytes(&self) -> ByteSlice<'_>;
    /// The number the run gave the file.
    fn file_number(&self) -> FileId;
    /// The cell of the text of the element at `at`, as it is written.
    fn spelling(&self, symbols: &mut impl Interner, at: Position) -> Symbol;
}

/// What interns a text as the cell of a row, and builds a name from parts.
pub(crate) trait Interner {
    /// The cell of a text derived from the file.
    fn derived(&mut self, source: ByteSlice<'_>, text: NameText<'_>) -> Symbol;
    /// The text of a cell this interned, when it has one.
    fn text_of<'a>(&'a self, source: ByteSlice<'a>, symbol: Symbol) -> Option<NameText<'a>>;
    /// The cell of the name of a kind.
    fn vocabulary(&mut self, source: ByteSlice<'_>, kind: KindBits, name: NameText<'_>) -> Symbol;
    /// Begins a name made of parts: where its parts start.
    fn begin_name(&mut self) -> usize;
    /// Adds a text to the name being made.
    fn push_text(&mut self, text: NameText<'_>);
    /// Adds the bytes `start..end` of the file to the name being made.
    fn push_span(&mut self, source: ByteSlice<'_>, start: Offset, end: Offset);
    /// Adds the text of a cell to the name being made.
    fn push_name(&mut self, source: ByteSlice<'_>, symbol: Symbol);
    /// The cell of the name made from the parts since `from`.
    fn end_name(&mut self, source: ByteSlice<'_>, from: usize) -> Symbol;
}

impl Interner for Symbols {
    fn derived(&mut self, source: ByteSlice<'_>, text: NameText<'_>) -> Symbol {
        languages::intern_derived(self, source, text)
    }
    fn text_of<'a>(&'a self, source: ByteSlice<'a>, symbol: Symbol) -> Option<NameText<'a>> {
        languages::text_of(self, source, symbol)
    }
    fn vocabulary(&mut self, source: ByteSlice<'_>, kind: KindBits, name: NameText<'_>) -> Symbol {
        languages::vocabulary_symbol(self, source, kind, name)
    }
    fn begin_name(&mut self) -> usize {
        languages::begin_name(self).0
    }
    fn push_text(&mut self, text: NameText<'_>) {
        languages::push_text(self, text);
    }
    fn push_span(&mut self, source: ByteSlice<'_>, start: Offset, end: Offset) {
        languages::push_span(self, source, start, end);
    }
    fn push_name(&mut self, source: ByteSlice<'_>, symbol: Symbol) {
        languages::push_name(self, source, symbol);
    }
    fn end_name(&mut self, source: ByteSlice<'_>, from: usize) -> Symbol {
        languages::end_name(self, source, languages::NameStart(from))
    }
}

/// The words of a file the fillers write cells into, and the name being made from parts.
#[derive(Debug)]
pub(crate) struct WordSink<'w> {
    /// The file's words.
    pub(crate) words: &'w mut FileWords,
    /// The parts of the names being made.
    built: String,
}

impl<'w> WordSink<'w> {
    /// A sink over the file's words.
    pub(crate) fn new(words: &'w mut FileWords) -> Self {
        Self { words, built: String::new() }
    }
}

impl Interner for WordSink<'_> {
    fn derived(&mut self, _source: ByteSlice<'_>, text: NameText<'_>) -> Symbol {
        Symbol(u32::try_from(languages::facts::intern(self.words, text).0).unwrap_or(u32::MAX))
    }
    fn text_of<'a>(&'a self, _source: ByteSlice<'a>, symbol: Symbol) -> Option<NameText<'a>> {
        Some(languages::facts::text(self.words, languages::CellWord(u64::from(symbol.0))))
    }
    fn vocabulary(&mut self, source: ByteSlice<'_>, _kind: KindBits, name: NameText<'_>) -> Symbol {
        self.derived(source, name)
    }
    fn begin_name(&mut self) -> usize {
        self.built.len()
    }
    fn push_text(&mut self, text: NameText<'_>) {
        self.built.push_str(text.0);
    }
    fn push_span(&mut self, source: ByteSlice<'_>, start: Offset, end: Offset) {
        let from = usize::try_from(start.0).unwrap_or(usize::MAX);
        let to = usize::try_from(end.0).unwrap_or(usize::MAX);
        self.built.push_str(source.0.get(from..to).map(|bytes| std::str::from_utf8(bytes).unwrap_or_default()).unwrap_or_default());
    }
    fn push_name(&mut self, _source: ByteSlice<'_>, symbol: Symbol) {
        let text = languages::facts::text(self.words, languages::CellWord(u64::from(symbol.0)));
        self.built.push_str(text.0);
    }
    fn end_name(&mut self, source: ByteSlice<'_>, from: usize) -> Symbol {
        let name = self.built.split_off(from);
        self.derived(source, NameText(&name))
    }
}

impl Syntax for Tree<'_> {
    fn kind_bits(&self, at: Position) -> KindBits {
        languages::kind_at(self.elements, at)
    }
    fn parent_of(&self, at: Position) -> Position {
        languages::parent_at(self.elements, at)
    }
    fn children_of(&self, at: Position) -> impl Iterator<Item = Position> + '_ {
        languages::children(self.elements, at)
    }
    fn start_of(&self, at: Position) -> Offset {
        languages::start_at(self.elements, at)
    }
    fn end_of(&self, at: Position) -> Offset {
        languages::end_at(self.elements, at)
    }
    fn decoded_of(&self, at: Position) -> Option<NameText<'_>> {
        languages::decoded_at(self.elements, at)
    }
    fn property_of(&self, at: Position, property: languages::PropertyId) -> Option<NameText<'_>> {
        languages::property_at(self.elements, at, property)
    }
    fn is_leaf(&self, at: Position) -> bool {
        languages::flags_at(self.elements, at).0 & languages::TOKEN.0 != 0
    }
    fn last_of(&self, at: Position) -> Position {
        languages::last_at(self.elements, at)
    }
    fn ordinal_of(&self, at: Position) -> languages::ChildOrdinal {
        languages::ordinal_at(self.elements, at)
    }
    fn positions(&self) -> impl Iterator<Item = Position> + '_ {
        languages::positions(self.elements)
    }
    fn bytes(&self) -> ByteSlice<'_> {
        self.source
    }
    fn file_number(&self) -> FileId {
        self.file
    }
    fn spelling(&self, _symbols: &mut impl Interner, at: Position) -> Symbol {
        languages::text_at(self.elements, at)
    }
}

impl Syntax for TypeScriptFile<'_> {
    fn kind_bits(&self, at: Position) -> KindBits {
        node_table::kind_of(self.table, at)
    }
    fn parent_of(&self, at: Position) -> Position {
        node_table::parent_of(self.table, at)
    }
    fn children_of(&self, at: Position) -> impl Iterator<Item = Position> + '_ {
        node_table::children_of(self.table, at).iter().copied()
    }
    fn start_of(&self, at: Position) -> Offset {
        node_table::span_of(self.table, at).start
    }
    fn end_of(&self, at: Position) -> Offset {
        node_table::span_of(self.table, at).end
    }
    fn decoded_of(&self, at: Position) -> Option<NameText<'_>> {
        // The tokens follow the nodes in the table, in the order they were written.
        let token = usize::from(at).checked_sub(node_table::node_count(self.table).0)?;
        let found = self.decoded.places.binary_search_by_key(&u32::try_from(token).ok()?, |place| place.0).ok()?;
        let (_, from, to) = *self.decoded.places.get(found)?;
        self.decoded.texts.get(usize::try_from(from).ok()?..usize::try_from(to).ok()?).map(NameText)
    }
    fn property_of(&self, at: Position, property: languages::PropertyId) -> Option<NameText<'_>> {
        node_table::property_of(self.table, at, property)
    }
    fn is_leaf(&self, at: Position) -> bool {
        usize::from(at) >= node_table::node_count(self.table).0
    }
    fn last_of(&self, at: Position) -> Position {
        Position(node_table::subtree_end_of(self.table, at).0.saturating_sub(1).max(at.0))
    }
    fn ordinal_of(&self, at: Position) -> languages::ChildOrdinal {
        node_table::ordinal_of(self.table, at)
    }
    fn positions(&self) -> impl Iterator<Item = Position> + '_ {
        (0..node_table::element_count(self.table).0).map(|index| Position(u32::try_from(index).unwrap_or(u32::MAX)))
    }
    fn bytes(&self) -> ByteSlice<'_> {
        self.source
    }
    fn file_number(&self) -> FileId {
        self.file
    }
    fn spelling(&self, symbols: &mut impl Interner, at: Position) -> Symbol {
        let from = usize::try_from(self.start_of(at).0).unwrap_or(usize::MAX);
        let to = usize::try_from(self.end_of(at).0).unwrap_or(usize::MAX);
        let text = self.source.0.get(from..to).map(|bytes| std::str::from_utf8(bytes).unwrap_or_default()).unwrap_or_default();
        symbols.derived(self.source, NameText(text))
    }
}
