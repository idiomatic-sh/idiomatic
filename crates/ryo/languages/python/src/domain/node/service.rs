//! The `node` service: free functions over a walked file. The id of a typed node, the typed node of an id, the text and the row
//! cells of an element, and the field a token fills under the node that holds it.

use super::types::{FixedWords, Numbered, SyntaxFile};
use crate::domain::labels::service::field_bits;
use crate::domain::labels::types::Field;
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyNodeRef, HasNodeIndex as _};
use ryo::languages::{
    self, FieldCode, NameText, NodeId, Offset, Position, Span, Symbol, node_table,
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

/// The file as the semantic model reads it: the typed nodes with their parents and the ends of their subtrees.
#[must_use]
pub(crate) const fn typed_tree<'f>(file: &SyntaxFile<'f>) -> TypedTree<'f, 'f> {
    TypedTree {
        parents: file.parents,
        ends: file.ends,
        nodes: file.nodes,
        annotations: file.annotations,
    }
}

/// The typed node of a syntax node id; none for a token and for an id the file does not have.
#[must_use]
pub(crate) fn node_at<'f>(file: &SyntaxFile<'f>, id: Position) -> Option<AnyNodeRef<'f>> {
    file.nodes.get(usize::from(id)).copied()
}

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

/// A fixed word of the language as a row's text cell: interned the first time a filler of the file writes it, read from `kept`
/// every time after. The words of a file are forgotten with the file, so `kept` is emptied when a file begins.
pub(crate) fn fixed(
    words: &mut languages::FileWords,
    kept: &mut FixedWords,
    text: NameText<'static>,
) -> Symbol {
    // The same constant is the same place in the program: the address is compared before the text.
    let found = kept
        .texts
        .iter()
        .position(|each| std::ptr::eq(each.0, text.0) || each.0 == text.0);
    if let Some(word) = found.and_then(|at| kept.words.get(at)) {
        return *word;
    }
    let interned = word(words, text);
    kept.texts.push(text);
    kept.words.push(interned);
    interned
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
