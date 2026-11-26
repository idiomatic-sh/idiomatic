//! The `node` service: free functions over a walked file. The id of a typed node, the typed node of an id, the text and the row
//! cells of an element, and the field a token fills under the node that holds it.

use super::types::{Numbered, SyntaxFile};
use crate::domain::labels::service::field_bits;
use crate::domain::labels::types::Field;
use crate::domain::model::lexical::types::{EmbeddedSite, TypedTree};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyNodeRef, HasNodeIndex as _};
use ryo::languages::{
    self, CellWord, FieldCode, FileId, NameText, NodeId, Offset, Position, Span, Symbol, node_table,
};

/// The field code the node table holds for a field: the number of the field in the language's vocabulary, zero for none.
#[must_use]
pub(crate) const fn field_code(field: Field) -> FieldCode {
    FieldCode(field_bits(field).0)
}

/// The id the walk gave a typed node; none for a node the walk did not open.
#[must_use]
pub(crate) fn id_of<'t>(node: impl Into<AnyNodeRef<'t>>) -> Option<Position> {
    node.into().node_index().load().as_u32().map(Position)
}

/// The id the table gave the node at `place`: the place itself in the file's own tree, counted from its segment's root in an
/// embedded segment.
#[must_use]
pub(crate) fn table_id(segments: &[EmbeddedSite], place: Position) -> Position {
    let after = segments.partition_point(|segment| segment.first <= place);
    after
        .checked_sub(1)
        .and_then(|within| segments.get(within))
        .map_or(place, |segment| {
            Position(
                segment
                    .root
                    .0
                    .saturating_add(place.0.saturating_sub(segment.first.0)),
            )
        })
}

/// The place of the node the table gave the id `id`: what `table_id` is undone by. None for a token and for an id past the
/// nodes.
#[must_use]
pub(crate) fn place_of_id(tree: TypedTree<'_, '_>, id: Position) -> Option<Position> {
    let after = tree.segments.partition_point(|segment| segment.root <= id);
    let place = after
        .checked_sub(1)
        .and_then(|within| tree.segments.get(within))
        .map_or(id, |segment| {
            Position(
                segment
                    .first
                    .0
                    .saturating_add(id.0.saturating_sub(segment.root.0)),
            )
        });
    (usize::from(place) < tree.nodes.len() && table_id(tree.segments, place) == id).then_some(place)
}

/// The id a key cell names in the file `file`: none for a cell that is no element's id of this file (the node of another
/// file, a number, a word). The id may still be past the file's elements: no row is written for it.
#[must_use]
pub(crate) fn keyed_id(file: FileId, key: CellWord) -> Option<Position> {
    let id = languages::node_position(NodeId(key.0));
    (languages::node_id(file, id).0 == key.0).then_some(id)
}

/// The node of the file a key cell names, by its place: none for a cell that is no node of this file (another file's node, a
/// token, an id past the nodes).
#[must_use]
pub(crate) fn keyed_place(
    file: FileId,
    tree: TypedTree<'_, '_>,
    key: CellWord,
) -> Option<Position> {
    keyed_id(file, key).and_then(|id| place_of_id(tree, id))
}

/// The syntax node of the file's own tree a key cell names: none for a cell that is no node of this file's own tree.
#[must_use]
pub(crate) fn keyed_node(file: &SyntaxFile<'_>, key: CellWord) -> Option<Position> {
    keyed_id(file.file, key).filter(|id| usize::from(*id) < file.nodes.len())
}

/// The id after the last element of the file's own tree: its nodes, then its tokens. The embedded segments follow.
#[must_use]
pub(crate) fn own_end(file: &SyntaxFile<'_>) -> Position {
    file.tree.segments.first().map_or_else(
        || Position(u32::try_from(node_table::element_count(file.table).0).unwrap_or(u32::MAX)),
        |segment| segment.root,
    )
}

/// The typed node of a syntax node id; none for a token and for an id the file does not have.
#[must_use]
pub(crate) fn node_at<'f>(file: &SyntaxFile<'f>, id: Position) -> Option<AnyNodeRef<'f>> {
    file.nodes.get(usize::from(id)).copied()
}

#[cfg(test)]
/// The syntax nodes of the file, in id order.
pub(crate) fn typed_nodes<'f>(
    file: &SyntaxFile<'f>,
) -> impl Iterator<Item = Numbered<'f>> + use<'f> {
    file.nodes.iter().enumerate().map(|(index, node)| Numbered {
        id: Position(u32::try_from(index).unwrap_or(u32::MAX)),
        node: *node,
    })
}

/// The syntax nodes of one kind of the language, in id order.
pub(crate) fn nodes_of_kind<'f>(
    file: &SyntaxFile<'f>,
    kind: Kind,
) -> impl Iterator<Item = Numbered<'f>> + use<'f> {
    let nodes = file.nodes;
    node_table::ids_of_kind(file.table, kind_bits(kind))
        .iter()
        .filter_map(move |id| {
            nodes.get(usize::from(*id)).map(|node| Numbered {
                id: *id,
                node: *node,
            })
        })
}

/// A semantic fact uses the public ID even when its typed node lives after a
/// physical token segment in the table.
pub(crate) fn semantic_node<'f>(file: &SyntaxFile<'f>, id: Position) -> Option<AnyNodeRef<'f>> {
    let place = place_of_id(file.tree, id)?;
    file.tree.nodes.get(usize::from(place)).copied()
}

pub(crate) fn semantic_nodes_of_kind<'f>(
    file: &SyntaxFile<'f>, kind: Kind,
) -> impl Iterator<Item = Numbered<'f>> + use<'f> {
    let syntax = *file;
    node_table::ids_of_kind(syntax.table, kind_bits(kind)).iter().filter_map(move |&id| {
        semantic_node(&syntax, id).map(|node| Numbered { id, node })
    })
}

/// Where an element is, as a row's span.
#[must_use]
pub(crate) fn span_at(file: &SyntaxFile<'_>, id: Position) -> Span {
    let span = node_table::span_of(file.table, id);
    Span {
        start: span.start,
        end: span.end,
    }
}

/// An element as a row's node cell.
#[must_use]
pub(crate) fn node_cell(file: &SyntaxFile<'_>, id: Position) -> NodeId {
    languages::node_id(file.file, id)
}

/// The text between two offsets of the file, as it is written; empty where the offsets are no text of the file.
#[must_use]
pub(crate) fn text_between<'f>(file: &SyntaxFile<'f>, start: Offset, end: Offset) -> NameText<'f> {
    let from = usize::try_from(start.0).unwrap_or(usize::MAX);
    let to = usize::try_from(end.0).unwrap_or(usize::MAX);
    NameText(file.text.0.get(from..to).unwrap_or_default())
}

/// The text an element spans, as it is written.
#[must_use]
pub(crate) fn written<'f>(file: &SyntaxFile<'f>, id: Position) -> NameText<'f> {
    let span = node_table::span_of(file.table, id);
    text_between(file, span.start, span.end)
}

/// A text as a row's text cell: a word of the file.
pub(crate) fn word(words: &mut languages::FileWords, text: NameText<'_>) -> Symbol {
    Symbol(u32::try_from(languages::facts::intern(words, text).0).unwrap_or(u32::MAX))
}

/// `field` for a token that is an operator of an expression, none for a bracket, a comma and trivia.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the brackets, the comma and the trivia are named; every other token is an operator here"
)]
const fn operator_field(named: Token, field: Field) -> Field {
    match named {
        Token::Lpar
        | Token::Rpar
        | Token::Lsqb
        | Token::Rsqb
        | Token::Lbrace
        | Token::Rbrace
        | Token::Comma
        | Token::Comment
        | Token::Newline
        | Token::Nl
        | Token::Indent
        | Token::Dedent => Field::None,
        _ => field,
    }
}

/// The field a token fills under the node that holds it: a name's identifier, an operation's operator, a constant's literal, an
/// interpolation's conversion, the dots of a relative import and the `async` of a comprehension. Every other token fills none.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the kinds whose tokens fill a field of the standard are named; a token under any other kind fills none"
)]
#[must_use]
pub(crate) fn token_field(parent: AnyNodeRef<'_>, kind: TokenKind, named: Token) -> Field {
    match parent {
        AnyNodeRef::ExprName(_) if named == Token::Name => Field::Id,
        AnyNodeRef::ExprUnaryOp(_)
        | AnyNodeRef::ExprBinOp(_)
        | AnyNodeRef::StmtAugAssign(_)
        | AnyNodeRef::ExprBoolOp(_) => operator_field(named, Field::Op),
        AnyNodeRef::ExprCompare(_) => operator_field(named, Field::Ops),
        AnyNodeRef::ExprStringLiteral(_)
        | AnyNodeRef::ExprBytesLiteral(_)
        | AnyNodeRef::ExprNumberLiteral(_)
        | AnyNodeRef::ExprBooleanLiteral(_)
        | AnyNodeRef::ExprNoneLiteral(_)
        | AnyNodeRef::ExprEllipsisLiteral(_)
        | AnyNodeRef::InterpolatedStringLiteralElement(_)
            if !matches!(
                named,
                Token::Lpar | Token::Rpar | Token::Nl | Token::Comment
            ) =>
        {
            Field::Value
        }
        AnyNodeRef::PatternMatchSingleton(_) if named == Token::Name => Field::Value,
        AnyNodeRef::InterpolatedElement(_) if named == Token::Name => Field::Conversion,
        AnyNodeRef::StmtImportFrom(_) if matches!(named, Token::Dot | Token::Ellipsis) => {
            Field::Level
        }
        AnyNodeRef::Comprehension(_) if kind == TokenKind::Async => Field::IsAsync,
        _ => Field::None,
    }
}

/// The expressions that are children of the node `id`, in the order the table lists them: the first is what `nth(.., 0)` named
/// in the element table.
pub(crate) fn expressions_of<'f>(file: &SyntaxFile<'f>, id: Position) -> impl Iterator<Item = Position> + use<'f> {
    let table = file.table;
    node_table::children_of(table, id).iter().copied().filter(move |&child| {
        crate::domain::syntax::service::category_of(crate::domain::syntax::service::kind_of(node_table::kind_of(table, child)))
            == crate::domain::nodes::types::Category::Expression
    })
}

/// The scope the node `id` is evaluated in, as a table id: the model's `scope_of` over the place the id numbers, the module when
/// the model holds none.
pub(crate) fn scope_id(file: &SyntaxFile<'_>, scope_of: &[Position], id: Position) -> Position {
    crate::domain::node::service::place_of_id(file.tree, id)
        .and_then(|place| scope_of.get(usize::from(place)))
        .map_or(Position(0), |&place| table_id(file.tree.segments, place))
}
