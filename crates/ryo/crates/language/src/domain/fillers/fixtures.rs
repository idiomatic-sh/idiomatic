//! The test support of the `fillers` concept: a language over files of words, its per-file view (the finished table, the bytes,
//! the file's number, how many fills it answered and how its fillers are weaker), and its fillers: `syntax-error` (only in part when the file holds a `?`),
//! `token-link` with a body for each of its declared keys (a word's link to the next, the kind written by a fixed word),
//! `language-target` (one row for a file), and every other relation said not read yet. The walk use case's fixture walks it.
//! A file that holds one of four bytes makes a filler weaker than the catalog declares (`weakness`), which is what the growth
//! harness (`growth`) rejects in its own cases: `%` (a link for every pair of words), `^` (`token-link` has no body for a key),
//! `&` (its key body writes every row), `+` (two targets for the file).

/// The growth harness every language's contract test holds its fillers to.
pub mod growth;

use crate::domain::facts::service::{fixed, syntax_error_rows};
use crate::domain::facts::types::{CellWord, FileWords, FixedNames, FixedWord, Wanted};
use crate::domain::fillers::service::{filled, filled_keyed};
use crate::domain::fillers::types::{
    FieldNames, Fill, FillItem, Fillers, KeyedFillItem, WalkKinds, WalkVocabulary,
};
use crate::domain::node_table::service::{node_count, own_end, span_of};
use crate::domain::node_table::types::NodeTable;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::relations::collections::relations::language_target::item::{
    LanguageTarget, LanguageTargetRow,
};
use ryo_relations::domain::relations::collections::relations::syntax_error::item::{
    SyntaxError, SyntaxErrorRow,
};
use ryo_relations::domain::relations::collections::relations::token_link::item::{
    TokenLink, TokenLinkColumn, TokenLinkRow,
};
use ryo_relations::domain::relations::service::{node_id, node_position};
use ryo_relations::domain::relations::types::{
    Column, Count, FileId, Integer, Missing, NodeId, Position, ReportText, Rows, Span,
};
use ryo_relations::domain::terms::types::{NameText, Symbol};

/// The language that walks files of words.
#[derive(Clone, Copy, Debug)]
pub struct Walking;

/// Its view of one file: the finished table, the file's number, how many fills it answered and how its fillers are weaker.
#[derive(Debug)]
pub struct WalkingFile<'f> {
    /// The file's table.
    pub table: &'f NodeTable,
    /// The file's bytes.
    pub source: &'f [u8],
    /// The file's number.
    pub file: FileId,
    /// How many relations it filled.
    pub fills: Count,
    /// How the file's fillers are weaker than the catalog declares: read from its bytes once, when the view is made.
    pub weakness: Weakness,
}

/// How a file's fillers are weaker than the catalog declares of their relations: what the growth harness's own cases reject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weakness {
    /// They are as the catalog declares.
    Sound,
    /// `token-link` writes a link for every pair of words: a row per pair, declared `PerElement`.
    Pairs,
    /// `token-link` has no body for a key: asked for one, it writes every row and says so.
    Unkeyed,
    /// `token-link`'s key body writes every row, though it answers that it wrote the key's.
    WholeByKey,
    /// `language-target` writes two rows: more than one for a file, declared `PerFile`.
    Twice,
}

/// How the fillers of a file of these bytes are weaker: by the first of the four bytes it holds, and sound with none.
#[must_use]
pub fn weakness(source: &[u8]) -> Weakness {
    let asked = source.iter().find_map(|byte| match *byte {
        PAIRS => Some(Weakness::Pairs),
        UNKEYED => Some(Weakness::Unkeyed),
        WHOLE_BY_KEY => Some(Weakness::WholeByKey),
        TWICE => Some(Weakness::Twice),
        _ => None,
    });
    asked.unwrap_or(Weakness::Sound)
}

/// The fixed name the fixture writes by handle: the kind of every link it writes.
pub const NEXT_TOKEN: FixedWord = FixedWord(0);

/// The byte of a file whose `token-link` filler writes a link for every pair of words: a row per pair, declared `PerElement`.
pub const PAIRS: u8 = b'%';

/// The byte of a file whose `token-link` filler has no body for a key: asked for one, it writes every row and says so.
pub const UNKEYED: u8 = b'^';

/// The byte of a file whose `token-link` key body writes every row, though it answers that it wrote the key's.
pub const WHOLE_BY_KEY: u8 = b'&';

/// The byte of a file whose `language-target` filler writes two rows: more than one for a file, declared `PerFile`.
pub const TWICE: u8 = b'+';

/// The fixture's `syntax-error` filler.
#[derive(Clone, Copy, Debug)]
pub struct SyntaxErrorFill;

impl FillItem for SyntaxErrorFill {
    type Walk = Walking;
    type Relation = SyntaxError;

    /// Writes the file's diagnostics as `syntax-error` rows, counting the fill.
    fn fill(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        rows: &mut dyn Rows<SyntaxErrorRow>,
    ) {
        file.fills = Count(file.fills.0.saturating_add(1));
        syntax_error_rows(file.table, file.file, words, rows);
        if file.source.contains(&b'?') {
            rows.partial(
                ReportText("a question the fixture cannot read"),
                Missing::Any,
            );
        }
    }
}

/// The fixture's `token-link` filler: each word of the file's own tree links to the word after it, and it has a body for both
/// of the relation's declared keys.
#[derive(Clone, Copy, Debug)]
pub struct TokenLinkFill;

/// The link from the word at `source` to the word at `target`; none when either is no word of the file's own tree.
fn link(
    file: &WalkingFile<'_>,
    words: &FileWords,
    source: Position,
    target: Position,
) -> Option<TokenLinkRow> {
    let first = Position(u32::try_from(node_count(file.table).0).ok()?);
    let own = first..own_end(file.table);
    if !own.contains(&source) || !own.contains(&target) {
        return None;
    }
    let span = span_of(file.table, source);
    Some(TokenLinkRow {
        source: node_id(file.file, source),
        target: node_id(file.file, target),
        link_kind: Symbol(u32::try_from(fixed(words, NEXT_TOKEN).0).unwrap_or(u32::MAX)),
        file: file.file,
        span: Span {
            start: span.start,
            end: span.end,
        },
    })
}

/// The word after `source`.
const fn after(source: Position) -> Position {
    Position(source.0.saturating_add(1))
}

impl FillItem for TokenLinkFill {
    type Walk = Walking;
    type Relation = TokenLink;

    /// Writes the link of every word of the file's own tree to the word after it, in source order; in a file of pairs, a link
    /// for every pair of words.
    fn fill(file: &mut WalkingFile<'_>, words: &mut FileWords, rows: &mut dyn Rows<TokenLinkRow>) {
        let first = u32::try_from(node_count(file.table).0).unwrap_or(u32::MAX);
        let end = own_end(file.table).0;
        for source in (first..end).map(Position) {
            let targets = if file.weakness == Weakness::Pairs {
                first..end
            } else {
                after(source).0..after(after(source)).0
            };
            for target in targets.map(Position) {
                if let Some(row) = link(file, words, source, target) {
                    rows.write(row);
                }
            }
        }
    }
}

impl KeyedFillItem for TokenLinkFill {
    const KEYS: &'static [Column] = &[
        Column::TokenLink(TokenLinkColumn::Source),
        Column::TokenLink(TokenLinkColumn::Target),
    ];

    /// Writes the one link from the word `key` names, or the one link to it: nothing else of the file is read. In a file
    /// whose key body is weak it writes every link.
    fn fill_key(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        column: Column,
        key: CellWord,
        rows: &mut dyn Rows<TokenLinkRow>,
    ) {
        if file.weakness == Weakness::WholeByKey {
            return <Self as FillItem>::fill(file, words, rows);
        }
        let at = node_position(NodeId(key.0));
        let source = if column == Column::TokenLink(TokenLinkColumn::Target) {
            at.0.checked_sub(1).map(Position)
        } else {
            Some(at)
        };
        if let Some(row) = source.and_then(|source| link(file, words, source, after(source))) {
            rows.write(row);
        }
    }
}

/// The fixture's `language-target` filler: one row for a file, whatever its size.
#[derive(Clone, Copy, Debug)]
pub struct LanguageTargetFill;

impl FillItem for LanguageTargetFill {
    type Walk = Walking;
    type Relation = LanguageTarget;

    /// Writes the file's one target; in a file of two, a second.
    fn fill(
        file: &mut WalkingFile<'_>,
        _words: &mut FileWords,
        rows: &mut dyn Rows<LanguageTargetRow>,
    ) {
        for minor in 0..=i64::from(file.weakness == Weakness::Twice) {
            rows.write(LanguageTargetRow {
                file: file.file,
                major: Integer(1),
                minor: Integer(minor),
            });
        }
    }
}

/// The fixture's filler of a relation it has not read: absent, for that reason. `syntax_error`, `token_link` and
/// `language_target` are written by hand; a `NodeTable` relation has no filler.
macro_rules! absent_filler {
    (syntax_error $( $rest:tt )*) => {};
    (token_link $( $rest:tt )*) => {};
    (language_target $( $rest:tt )*) => {};
    ($module:ident item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column $column:tt, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt) => {};
    ($module:ident item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column $column:tt, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, size $size:ident, keys $keys:tt,
        columns $columns:tt) => {
        fn $module(
            _file: &mut WalkingFile<'_>,
            _words: &mut FileWords,
            _wanted: Wanted,
            _rows: &mut dyn Rows<
                ryo_relations::domain::relations::collections::relations::$module::item::$row,
            >,
        ) -> Fill {
            Fill::Absent(ryo_relations::domain::answers::types::AbsenceCause::NotYetRead)
        }
    };
}

/// Every relation's filler of the fixture.
macro_rules! absent_fillers {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        $( $( ryo_relations::$module!(absent_filler, $module); )* )*
    };
}

impl Fillers for Walking {
    type File<'f> = WalkingFile<'f>;

    ryo_relations::each_relation!(absent_fillers);

    fn syntax_error(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        wanted: Wanted,
        rows: &mut dyn Rows<SyntaxErrorRow>,
    ) -> Fill {
        filled::<SyntaxErrorFill>(file, words, wanted, rows)
    }

    fn token_link(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        wanted: Wanted,
        rows: &mut dyn Rows<TokenLinkRow>,
    ) -> Fill {
        if file.weakness == Weakness::Unkeyed {
            return filled::<TokenLinkFill>(file, words, wanted, rows);
        }
        filled_keyed::<TokenLinkFill>(file, words, wanted, rows)
    }

    fn language_target(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        wanted: Wanted,
        rows: &mut dyn Rows<LanguageTargetRow>,
    ) -> Fill {
        filled::<LanguageTargetFill>(file, words, wanted, rows)
    }

    fn render(_file: &mut WalkingFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }

    /// The fixture's names, each at its code: the kind no element has, the sentence, the word, the quote; no field and the
    /// field a label fills; the kind of a link.
    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(vec![
                NameText("blank"),
                NameText("sentence"),
                NameText("word"),
                NameText("quote"),
            ]),
            fields: FieldNames(vec![NameText(""), NameText("label")]),
            fixed: FixedNames(vec![NameText("next-token")]),
        }
    }
}
