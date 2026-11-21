//! The rows of the semantic model for a source, read through the test view: each row shown one way (a node as `kind@start:end`,
//! a word as its text, a span as `start:end`), for the tests that hold the fillers to today's readers.

use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::syntax::fixtures::model_view::{FixtureView, ModelFixture, viewed};
use ryo::languages::node_table::{kind_of, span_of};
use ryo::languages::{
    self, Bits, CatalogRelation, CellKind, CellWord, DeclarationKind, FileId, FileWords, Fill, KindBits, Missing,
    NodeTable, Position, ReportText, Rows, Symbol, Term as _, facts, node_position, term_symbol,
};
use strum::IntoEnumIterator as _;

/// The first number of the words that mean the same in every file: shown by number, the same on both sides.
const FIXED_WORDS: u32 = 0xFFFF_0000;

/// A word that means the same in every file, shown: a declaration kind by its name, an unknown value as `\\N`.
fn fixed_word(symbol: Symbol) -> String {
    DeclarationKind::iter()
        .find(|kind| term_symbol(*kind) == symbol)
        .map_or_else(|| "\\N".to_owned(), |kind| kind.text().0.to_owned())
}

/// A text with its backslashes, tabs and line ends written out, so that a row stays one line of tab-separated cells.
fn escaped(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => shown.push_str("\\\\"),
            '\t' => shown.push_str("\\t"),
            '\n' => shown.push_str("\\n"),
            '\r' => shown.push_str("\\r"),
            other => shown.push(other),
        }
    }
    shown
}

/// The rows a filler writes, kept as cells until its words can be read.
pub(crate) struct Gathered<R: CatalogRelation> {
    /// The rows.
    pub rows: Vec<R::Words>,
    /// The causes.
    pub causes: Vec<String>,
}

impl<R: CatalogRelation> Rows<R::Row> for Gathered<R> {
    fn write(&mut self, row: R::Row) {
        self.rows.push(R::words(row));
    }

    fn partial(&mut self, cause: ReportText<'_>, _missing: Missing) {
        self.causes.push(cause.0.to_owned());
    }

    fn partial_at(&mut self, _file: FileId, cause: ReportText<'_>, _missing: Missing) {
        self.causes.push(cause.0.to_owned());
    }
}

/// The rows a filler writes, counted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counted {
    /// How many rows.
    pub rows: usize,
    /// The causes it gave for a relation read only in part.
    pub causes: Vec<String>,
}

impl<Row> Rows<Row> for Counted {
    fn write(&mut self, _row: Row) {
        self.rows += 1;
    }

    fn partial(&mut self, cause: ReportText<'_>, _missing: Missing) {
        self.causes.push(cause.0.to_owned());
    }

    fn partial_at(&mut self, _file: FileId, cause: ReportText<'_>, _missing: Missing) {
        self.causes.push(cause.0.to_owned());
    }
}

/// How many rows a filler writes for `source`, read through the test view, with the causes it gives.
pub(crate) fn counted<Row>(
    source: &str,
    fill: impl Fn(&mut FixtureView<'_>, &mut FileWords, &mut dyn Rows<Row>) -> Fill,
) -> Counted {
    let mut counted = Counted::default();
    viewed(&mut ModelFixture::default(), source, FileId(0), &mut |view| {
        let base = facts::base_words(languages::CheckedNames::default(), &[]);
        let mut words = facts::file_words(&base);
        if let Fill::Absent(cause) = fill(view, &mut words, &mut counted) {
            counted.causes.push(format!("absent: {cause:?}"));
        }
    });
    counted
}

/// The name of the kind `bits`.
pub(crate) fn kind_shown(bits: KindBits) -> &'static str {
    kinds()
        .find(|kind| kind_bits(*kind) == bits)
        .map_or("?", |kind| kind_name(kind).0)
}

/// One row shown: `node` shows a node, `word` a word of the file.
pub(crate) fn shown_row<R: CatalogRelation>(
    cells: &[Bits],
    node: &dyn Fn(Position) -> String,
    word: &dyn Fn(Symbol) -> String,
) -> String {
    let texts: Vec<String> = R::kinds()
        .iter()
        .zip(cells)
        .map(|(kind, bits)| match kind {
            CellKind::Node => node(node_position(languages::NodeId(bits.0))),
            CellKind::Symbol => {
                let symbol = Symbol(u32::try_from(bits.0).unwrap_or(u32::MAX));
                if symbol.0 >= FIXED_WORDS { fixed_word(symbol) } else { escaped(&word(symbol)) }
            }
            CellKind::File => bits.0.to_string(),
            CellKind::Span => format!("{}:{}", bits.0 >> 32, bits.0 & 0xffff_ffff),
            CellKind::Number => bits.0.cast_signed().to_string(),
            CellKind::Flag => format!("flag{}", bits.0),
        })
        .collect();
    texts.join("\t")
}

/// One row a filler wrote, shown with the nodes of `table` and the words of `words`.
pub(crate) fn shown_filled<R: CatalogRelation>(cells: &[Bits], table: &NodeTable, words: &FileWords) -> String {
    let node = |at: Position| {
        let span = span_of(table, at);
        format!("{}@{}:{}", kind_shown(kind_of(table, at)), span.start.0, span.end.0)
    };
    let word = |symbol: Symbol| facts::text(words, CellWord(u64::from(symbol.0))).0.to_owned();
    shown_row::<R>(cells, &node, &word)
}
