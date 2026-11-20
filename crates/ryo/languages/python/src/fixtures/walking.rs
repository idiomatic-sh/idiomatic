//! Walking a source with the language's walk, for its tests and its row-equality bench: the tree the walk writes and the rows its
//! fillers write, each compared with what today's reading of the same source gives. The two number their elements differently
//! (today's table numbers nodes and tokens together in pre-order, the walk's numbers the nodes first), so an element is compared
//! by what it is and where it is: its kind, its span, and its place among the elements that share both.

use ryo::languages::{
    self, ByteSlice, CatalogRelation, CellKind, CellWord, Column, FactAnswer, FileFacts, FileId,
    FileVisit, FilledColumns, KindBits, NodeTable, Offset, Position, Relation, RelationNeeds, Walk,
    WalkRequest, facts, node_table,
};
use std::collections::HashMap;

/// An element by what it is and where: its kind, its span, and its place among the elements of the file that share both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementKey {
    /// Its kind.
    pub kind: KindBits,
    /// Its first byte.
    pub start: Offset,
    /// The byte after its last.
    pub end: Offset,
    /// Its place among the elements of the same kind and span, in the order of the file.
    pub nth: u32,
}

/// One cell of a row, in a form two readings of a file can be compared in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Seen {
    /// An element of the tree.
    Element(ElementKey),
    /// A node cell that names no element of the tree: a report's id.
    Other(u64),
    /// A text.
    Text(String),
    /// A value the language cannot know, by the number of its reason.
    Unknown(u8),
    /// No text.
    NoText,
    /// The file.
    File,
    /// A span of the file.
    Span(Offset, Offset),
    /// A number.
    Number(i64),
    /// A flag.
    Flag(bool),
}

/// One element of a tree where it stands: its parent, its place among the parent's children and the field it fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Placement {
    /// The element.
    pub element: ElementKey,
    /// Its parent; itself for the root.
    pub parent: ElementKey,
    /// Its place among the parent's children.
    pub ordinal: u32,
    /// The number of the field it fills; zero for none.
    pub field: u16,
}

/// The elements of one reading of a file: the key of each id.
#[derive(Clone, Debug, Default)]
pub struct Keys(pub Vec<ElementKey>);

/// The key of each element, numbered among the elements that share its kind and span in id order.
pub(crate) fn keyed(elements: impl Iterator<Item = (KindBits, Offset, Offset)>) -> Keys {
    let mut seen: HashMap<(KindBits, Offset, Offset), u32> = HashMap::new();
    Keys(
        elements
            .map(|(kind, start, end)| {
                let nth = seen.entry((kind, start, end)).or_insert(0);
                let key = ElementKey {
                    kind,
                    start,
                    end,
                    nth: *nth,
                };
                *nth += 1;
                key
            })
            .collect(),
    )
}

/// The key of each element of the walk's table. Tokens follow the nodes there, so the elements of one kind and span are numbered
/// in the order of the file as today's table has them: the two kinds of element never share a kind.
#[must_use]
pub fn walked_keys(table: &NodeTable) -> Keys {
    let count = u32::try_from(node_table::element_count(table).0).unwrap_or(u32::MAX);
    keyed((0..count).map(|id| {
        let span = node_table::span_of(table, Position(id));
        (
            node_table::kind_of(table, Position(id)),
            span.start,
            span.end,
        )
    }))
}

/// How many elements the file's own tree has in the walk's table: its nodes, then its tokens; the embedded segments follow.
#[must_use]
pub fn walked_own_count(table: &NodeTable) -> usize {
    usize::from(node_table::own_end(table))
}

/// The key of each element of the file's own tree in the walk's table.
#[must_use]
pub fn walked_own_keys(table: &NodeTable) -> Keys {
    let mut keys = walked_keys(table);
    keys.0.truncate(walked_own_count(table));
    keys
}

/// Where each element of the walk's table stands, sorted.
#[must_use]
pub fn walked_placements(table: &NodeTable, keys: &Keys) -> Vec<Placement> {
    let mut placements: Vec<Placement> = keys
        .0
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let id = Position(u32::try_from(index).unwrap_or(u32::MAX));
            let parent = node_table::parent_of(table, id);
            Placement {
                element: *element,
                parent: keys.0.get(usize::from(parent)).copied().unwrap_or(*element),
                ordinal: node_table::ordinal_of(table, id).0,
                field: node_table::field_of(table, id).0,
            }
        })
        .collect();
    placements.sort_unstable();
    placements
}

/// The kinds of the columns of a relation Python fills or reads; none for any other relation.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the relations Python reads are compared; any other relation has no row to compare"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each relation Python reads: the length is the catalog's"
)]
#[must_use]
pub fn column_kinds(relation: Relation) -> &'static [CellKind] {
    /// The column kinds of each relation named.
    macro_rules! kinds {
        ($( $variant:ident ),* $(,)?) => {
            match relation {
                $( Relation::$variant => <languages::$variant as CatalogRelation>::kinds(), )*
                _ => &[],
            }
        };
    }
    kinds!(
        Annotates,
        Assigns,
        BindingEvent,
        BindingRead,
        BindingResolution,
        BindingShadow,
        BindingUse,
        BraceFormatField,
        BraceFormatParse,
        BytesLiteral,
        CallArgument,
        Comment,
        CompilerFeature,
        ConstantKey,
        Decorates,
        Defines,
        EmbeddedExpression,
        ExpressionShape,
        ExpressionType,
        FormatField,
        FormatParse,
        FormatSpecification,
        Function,
        FunctionTerminal,
        Identifier,
        ImplicitSymbol,
        IndentRegion,
        Inherits,
        InterpolatedStringLiteral,
        LanguageTarget,
        LiteralExpressionParse,
        LiteralProperties,
        NumericLiteral,
        Parameter,
        QualifiedName,
        RawFormatField,
        RawFormatSegment,
        Reference,
        Resolves,
        Scope,
        SourceLocation,
        StandardLibraryImport,
        StaticCall,
        StaticImport,
        StringLiteral,
        SyntaxError,
        TokenLink,
    )
}

/// The key columns the catalog declares of a relation, in its order: each with its place in a row and what it holds. Empty for
/// a relation with no declared key.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the relations the catalog declares a key of have one; any other relation has none to ask"
)]
#[must_use]
pub fn declared_keys(relation: Relation) -> Vec<(Column, usize, CellKind)> {
    /// The declared keys of each relation named, each found among the relation's own columns.
    macro_rules! keys {
        ($( $variant:ident ),* $(,)?) => {
            match relation {
                $( Relation::$variant => <languages::$variant as CatalogRelation>::KEYS
                    .iter()
                    .filter_map(|key| {
                        let place = <languages::$variant as CatalogRelation>::columns()
                            .iter()
                            .position(|column| Column::$variant(*column) == *key)?;
                        let kind = <languages::$variant as CatalogRelation>::kinds().get(place)?;
                        Some((*key, place, *kind))
                    })
                    .collect(), )*
                _ => Vec::new(),
            }
        };
    }
    keys!(
        BindingRead,
        BindingShadow,
        ConstantKey,
        ExpressionShape,
        ExpressionType,
        Function,
        Inherits,
        RawFormatSegment,
        Reference,
        Resolves,
        Scope,
        TokenLink,
    )
}

/// A cell that is no node and no text, by the kind of its column.
pub(crate) fn plain(kind: CellKind, word: u64) -> Seen {
    match kind {
        CellKind::Node => Seen::Other(word),
        CellKind::Symbol => Seen::NoText,
        CellKind::File => Seen::File,
        CellKind::Span => Seen::Span(
            Offset(u32::try_from(word >> 32).unwrap_or(u32::MAX)),
            Offset(u32::try_from(word & 0xffff_ffff).unwrap_or(u32::MAX)),
        ),
        CellKind::Number => Seen::Number(word.cast_signed()),
        CellKind::Flag => Seen::Flag(word != 0),
    }
}

/// A node cell: the element at its position, or the id itself when it names no element of the tree.
pub(crate) fn element(keys: &Keys, word: u64) -> Seen {
    let position = usize::try_from(word & 0xffff_ffff).unwrap_or(usize::MAX);
    keys.0
        .get(position)
        .map_or(Seen::Other(word & 0xffff_ffff), |key| Seen::Element(*key))
}

/// A text cell that holds no text of the file: no text, the unknown value of its reason, or a word of a closed vocabulary (a
/// declaration's or a parameter's kind).
pub(crate) fn special(word: u64) -> Option<Seen> {
    use languages::{DeclarationKind, ParameterKind, Term as _, UnknownReason};
    use strum::IntoEnumIterator as _;
    let symbol = languages::Symbol(u32::try_from(word).unwrap_or(u32::MAX));
    if symbol == languages::NO_SYMBOL {
        return Some(Seen::NoText);
    }
    // The reserved words stand in blocks after every word a file interns, the unknown values last: a symbol below the first
    // block is a text.
    let first_unknown = languages::unknown_symbol(UnknownReason(0)).0;
    if symbol.0 >= first_unknown {
        return u8::try_from(symbol.0 - first_unknown)
            .ok()
            .map(Seen::Unknown);
    }
    let first_term = DeclarationKind::iter()
        .map(languages::term_symbol)
        .chain(ParameterKind::iter().map(languages::term_symbol))
        .min()?;
    if symbol < first_term {
        return None;
    }
    DeclarationKind::iter()
        .find(|kind| languages::term_symbol(*kind) == symbol)
        .map(|kind| kind.text())
        .or_else(|| {
            ParameterKind::iter()
                .find(|kind| languages::term_symbol(*kind) == symbol)
                .map(|kind| kind.text())
        })
        .map(|text| Seen::Text(text.0.to_owned()))
}

/// The rows of `relation` the walk's fillers write for a lent file, sorted; and how the fill answered.
pub fn walked_rows(
    file: &mut FileFacts<'_>,
    keys: &Keys,
    relation: Relation,
    words: &mut languages::FileWords,
) -> (Vec<Vec<Seen>>, FactAnswer) {
    let mut columns = FilledColumns::default();
    let answer = file
        .facts
        .fill(relation, languages::Wanted::Whole, words, &mut columns);
    let kinds = column_kinds(relation);
    let count = columns.0.first().map_or(0, |column| column.0.len());
    let mut rows: Vec<Vec<Seen>> = (0..count)
        .map(|row| {
            kinds
                .iter()
                .zip(&columns.0)
                .map(|(kind, column)| {
                    let word = column.0.get(row).copied().unwrap_or(CellWord(0)).0;
                    match kind {
                        CellKind::Node => element(keys, word),
                        CellKind::Symbol => special(word).unwrap_or_else(|| {
                            Seen::Text(facts::text(words, CellWord(word)).0.to_owned())
                        }),
                        CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => {
                            plain(*kind, word)
                        }
                    }
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    (rows, answer)
}

/// A worker's words for the files `walk` lends: over the run's base words, which hold the fixed names the walk's language
/// states and no constant of a program. The fillers of a lent file write their fixed names by handle, so they are filled into
/// these words and no others.
#[must_use]
pub fn walk_words<W: Walk>(_walk: &W) -> languages::FileWords {
    facts::file_words(&facts::base_words(&W::vocabulary().fixed, &[]))
}

/// A visit of a lent file by a function.
struct Visit<'v>(&'v mut dyn FnMut(FileFacts<'_>));

impl FileVisit for Visit<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        (self.0)(file);
    }
}

/// Walks `bytes` with `walk` as the file numbered 0, `needs` named as the relations read, and hands the lent file to `visit`.
///
/// # Errors
///
/// When the language does not walk the file.
pub fn walked<W: Walk>(
    walk: &W,
    path: &str,
    bytes: &[u8],
    needs: &[Relation],
    table: &mut NodeTable,
    scratch: &mut W::Scratch,
    visit: &mut dyn FnMut(FileFacts<'_>),
) -> Result<(), languages::WalkError> {
    let mut builder = node_table::builder(table, W::KIND_COUNT);
    walk.walk(
        WalkRequest {
            path: languages::SourcePath(path),
            file: FileId(0),
            source: ByteSlice(bytes),
        },
        RelationNeeds(needs),
        &mut builder,
        scratch,
        &mut Visit(visit),
    )
}
