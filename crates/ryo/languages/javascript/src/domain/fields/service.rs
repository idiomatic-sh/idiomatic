//! The `fields` service: free functions named by verb that say the role of an element under its parent. The tree is `ESTree`'s, and the
//! role of a child is the property of its parent that holds it. The parser's visitor reports no property, so a role is read from the
//! kinds of the parent and the child, from where the child stands among the nodes beside it, and, where a property may be left out, from
//! the token that comes before the child (`(` before parameters, `extends` before a superclass).

use ryo::languages::{self, Position, Tree};
use strum::IntoEnumIterator as _;

use super::constants::{ARROW, COLON, EXTENDS, FROM, OPEN_PARENTHESIS, PARENTHESES, SEMICOLON};
use super::types::{
    Field, KindRole, Lastness, Marker, MarkerRole, Operands, Ordinal, Precedence, Standing,
};
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::syntax::service::{is_token, kind, parent, spelling};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;

/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: Field) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: Field) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = Field> {
    Field::iter()
}

/// Where `at` stands among the nodes under its parent, and whether it is the last of them.
fn place(tree: &Tree<'_>, at: Position) -> Standing {
    let placed: Vec<Position> = languages::children(tree.elements, parent(tree, at))
        .filter(|each| {
            let each = *each;
            matches!(kind(tree, each), Kind::Node(_))
        })
        .collect();
    let ordinal = placed
        .iter()
        .position(|each| *each == at)
        .unwrap_or_default();
    Standing {
        ordinal: Ordinal(ordinal),
        lastness: if placed.last() == Some(&at) {
            Lastness::Last
        } else {
            Lastness::NotLast
        },
    }
}

/// The tokens among the children of the parent of `at` that come before it and are spelled `marker`.
fn markers_before<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    marker: Marker,
) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, parent(tree, at))
        .take_while(move |each| *each != at)
        .filter(move |each| {
            let each = *each;
            is_token(tree, each) == Leaf::Token && spelling(tree, each).0 == marker.0
        })
}

/// Whether a token spelled `marker` comes before `at` among the children of its parent.
fn after(tree: &Tree<'_>, at: Position, marker: Marker) -> Precedence {
    if markers_before(tree, at, marker).next().is_some() {
        Precedence::Preceded
    } else {
        Precedence::Unpreceded
    }
}

/// How many tokens spelled `marker` come before `at` among the children of its parent: the place of the clause they open.
fn count_before(tree: &Tree<'_>, at: Position, marker: Marker) -> Ordinal {
    Ordinal(markers_before(tree, at, marker).count())
}

/// The role a node held by its place has: the `ordinal`-th of `roles`, the last of them repeated.
fn nth(roles: &[Field], ordinal: Ordinal) -> Field {
    roles
        .get(ordinal.0)
        .or_else(|| roles.last())
        .copied()
        .unwrap_or_default()
}

/// The token that spells the operator of the expression at `holder`: the first token that is neither a comment nor a parenthesis,
/// after the left operand of a binary expression, anywhere in a unary or an update one; none for any other node.
fn operator_of(tree: &Tree<'_>, holder: Position) -> Option<Position> {
    let operands = match kind(tree, holder) {
        Kind::Node(
            JavaScriptNode::BinaryExpression | JavaScriptNode::LogicalExpression | JavaScriptNode::AssignmentExpression,
        ) => Operands::Binary,
        Kind::Node(JavaScriptNode::UnaryExpression | JavaScriptNode::UpdateExpression) => Operands::Unary,
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => return None,
    };
    let children: Vec<Position> = languages::children(tree.elements, holder).collect();
    // A token before the left operand of a binary expression is a parenthesis of the operand.
    let from = match operands {
        Operands::Binary => children
            .iter()
            .position(|each| matches!(kind(tree, *each), Kind::Node(_)))?
            .saturating_add(1),
        Operands::Unary => 0,
    };
    children.get(from..)?.iter().copied().find(|each| {
        let each = *each;
        matches!(kind(tree, each), Kind::Token(_))
            && !matches!(kind(tree, each), Kind::Token(Token::Block | Token::Line))
            && !PARENTHESES
                .iter()
                .any(|marker| spelling(tree, each).0 == marker.0)
    })
}

/// The role a child of the kind `node` plays.
const fn kinded(node: JavaScriptNode, field: Field) -> KindRole {
    KindRole { node, field }
}

/// The role of the children after a token spelled `marker`.
const fn marked(marker: Marker, field: Field) -> MarkerRole {
    MarkerRole { marker, field }
}

/// The role of a child of `child` kind: the role paired with its kind in `kinds`, else `otherwise`.
fn by_kind(child: Kind, kinds: &[KindRole], otherwise: Field) -> Field {
    kinds
        .iter()
        .find(|each| child == Kind::Node(each.node))
        .map_or(otherwise, |each| each.field)
}

/// The role of `at` by the tokens before it: the role paired with the first of `markers` a token before it spells, else `otherwise`.
fn by_marker(tree: &Tree<'_>, at: Position, markers: &[MarkerRole], otherwise: Field) -> Field {
    markers
        .iter()
        .find(|each| after(tree, at, each.marker) == Precedence::Preceded)
        .map_or(otherwise, |each| each.field)
}

/// The role of a child of a property: the default of a shorthand pattern (`{ a = 1 }`) is its value and the name beside it its key,
/// though the wider default comes first; any other child is the key, then the value.
fn property(tree: &Tree<'_>, at: Position, child: Kind, ordinal: Ordinal) -> Field {
    let defaulted = languages::children(tree.elements, parent(tree, at))
        .any(|each| kind(tree, each) == Kind::Node(JavaScriptNode::AssignmentPattern));
    if child == Kind::Node(JavaScriptNode::AssignmentPattern) {
        Field::Value
    } else if defaulted {
        Field::Key
    } else {
        nth(&[Field::Key, Field::Value], ordinal)
    }
}

/// The role of a node under a node of `holder` kind.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of `ESTree` node: the table of the properties is the proof, and splitting it would split it"
)]
fn held(tree: &Tree<'_>, at: Position, holder: JavaScriptNode, child: Kind) -> Field {
    let Standing { ordinal, lastness } = place(tree, at);
    match holder {
        JavaScriptNode::Program | JavaScriptNode::BlockStatement | JavaScriptNode::ClassBody | JavaScriptNode::StaticBlock => Field::Body,
        JavaScriptNode::ArrayExpression | JavaScriptNode::ArrayPattern => Field::Elements,
        JavaScriptNode::ObjectExpression | JavaScriptNode::ObjectPattern => Field::Properties,
        JavaScriptNode::SequenceExpression => Field::Expressions,
        JavaScriptNode::TemplateLiteral => by_kind(
            child,
            &[kinded(JavaScriptNode::TemplateElement, Field::Quasis)],
            Field::Expressions,
        ),
        JavaScriptNode::VariableDeclaration => Field::Declarations,
        JavaScriptNode::SwitchStatement => by_kind(
            child,
            &[kinded(JavaScriptNode::SwitchCase, Field::Cases)],
            Field::Discriminant,
        ),
        JavaScriptNode::SwitchCase => by_marker(tree, at, &[marked(COLON, Field::Consequent)], Field::Test),
        JavaScriptNode::ImportDeclaration => by_kind(
            child,
            &[
                kinded(JavaScriptNode::ImportSpecifier, Field::Specifiers),
                kinded(JavaScriptNode::ImportDefaultSpecifier, Field::Specifiers),
                kinded(JavaScriptNode::ImportNamespaceSpecifier, Field::Specifiers),
                kinded(JavaScriptNode::ImportAttribute, Field::Attributes),
            ],
            Field::Source,
        ),
        JavaScriptNode::ExportNamedDeclaration => by_kind(
            child,
            &[
                kinded(JavaScriptNode::ExportSpecifier, Field::Specifiers),
                kinded(JavaScriptNode::ImportAttribute, Field::Attributes),
            ],
            by_marker(tree, at, &[marked(FROM, Field::Source)], Field::Declaration),
        ),
        JavaScriptNode::ExportAllDeclaration => by_kind(
            child,
            &[kinded(JavaScriptNode::ImportAttribute, Field::Attributes)],
            by_marker(tree, at, &[marked(FROM, Field::Source)], Field::Exported),
        ),
        JavaScriptNode::ExportDefaultDeclaration => Field::Declaration,
        JavaScriptNode::ChainExpression
        | JavaScriptNode::ExpressionStatement
        | JavaScriptNode::JSXExpressionContainer
        | JavaScriptNode::JSXSpreadChild => Field::Expression,
        JavaScriptNode::AwaitExpression
        | JavaScriptNode::ReturnStatement
        | JavaScriptNode::ThrowStatement
        | JavaScriptNode::SpreadElement
        | JavaScriptNode::RestElement
        | JavaScriptNode::UnaryExpression
        | JavaScriptNode::UpdateExpression
        | JavaScriptNode::YieldExpression
        | JavaScriptNode::JSXSpreadAttribute => Field::Argument,
        JavaScriptNode::BreakStatement | JavaScriptNode::ContinueStatement => Field::Label,
        JavaScriptNode::JSXClosingElement => Field::Name,
        JavaScriptNode::ImportDefaultSpecifier | JavaScriptNode::ImportNamespaceSpecifier => Field::Local,
        JavaScriptNode::AssignmentExpression
        | JavaScriptNode::BinaryExpression
        | JavaScriptNode::LogicalExpression
        | JavaScriptNode::AssignmentPattern => nth(&[Field::Left, Field::Right], ordinal),
        JavaScriptNode::ConditionalExpression | JavaScriptNode::IfStatement => {
            nth(&[Field::Test, Field::Consequent, Field::Alternate], ordinal)
        }
        JavaScriptNode::MemberExpression | JavaScriptNode::JSXMemberExpression => {
            nth(&[Field::Object, Field::Property], ordinal)
        }
        JavaScriptNode::MetaProperty => nth(&[Field::Meta, Field::Property], ordinal),
        JavaScriptNode::WhileStatement => nth(&[Field::Test, Field::Body], ordinal),
        JavaScriptNode::DoWhileStatement => nth(&[Field::Body, Field::Test], ordinal),
        JavaScriptNode::WithStatement => nth(&[Field::Object, Field::Body], ordinal),
        JavaScriptNode::LabeledStatement => nth(&[Field::Label, Field::Body], ordinal),
        JavaScriptNode::ForInStatement | JavaScriptNode::ForOfStatement => {
            nth(&[Field::Left, Field::Right, Field::Body], ordinal)
        }
        JavaScriptNode::ImportSpecifier => nth(&[Field::Imported, Field::Local], ordinal),
        JavaScriptNode::ExportSpecifier => nth(&[Field::Local, Field::Exported], ordinal),
        JavaScriptNode::Property => property(tree, at, child, ordinal),
        JavaScriptNode::ImportAttribute | JavaScriptNode::PropertyDefinition => {
            nth(&[Field::Key, Field::Value], ordinal)
        }
        JavaScriptNode::JSXAttribute => nth(&[Field::Name, Field::Value], ordinal),
        JavaScriptNode::JSXNamespacedName => nth(&[Field::Namespace, Field::Name], ordinal),
        JavaScriptNode::ImportExpression => nth(&[Field::Source, Field::Options], ordinal),
        JavaScriptNode::VariableDeclarator => nth(&[Field::Id, Field::Init], ordinal),
        JavaScriptNode::CallExpression | JavaScriptNode::NewExpression => {
            nth(&[Field::Callee, Field::Arguments], ordinal)
        }
        JavaScriptNode::TaggedTemplateExpression => nth(&[Field::Tag, Field::Quasi], ordinal),
        // A method's function is its last node: what comes before is its key.
        JavaScriptNode::MethodDefinition => match lastness {
            Lastness::Last => Field::Value,
            Lastness::NotLast => Field::Key,
        },
        JavaScriptNode::CatchClause => by_kind(
            child,
            &[kinded(JavaScriptNode::BlockStatement, Field::Body)],
            Field::Param,
        ),
        JavaScriptNode::TryStatement => by_kind(
            child,
            &[kinded(JavaScriptNode::CatchClause, Field::Handler)],
            nth(&[Field::Block, Field::Finalizer], ordinal),
        ),
        // A `for` writes each of its three clauses only when it has one: the semicolons before a clause say which it is; its body is
        // its last node.
        JavaScriptNode::ForStatement => match lastness {
            Lastness::Last => Field::Body,
            Lastness::NotLast => nth(
                &[Field::Init, Field::Test, Field::Update],
                count_before(tree, at, SEMICOLON),
            ),
        },
        JavaScriptNode::ArrowFunctionExpression => {
            by_marker(tree, at, &[marked(ARROW, Field::Body)], Field::Params)
        }
        JavaScriptNode::FunctionDeclaration | JavaScriptNode::FunctionExpression => by_kind(
            child,
            &[kinded(JavaScriptNode::BlockStatement, Field::Body)],
            by_marker(
                tree,
                at,
                &[marked(OPEN_PARENTHESIS, Field::Params)],
                Field::Id,
            ),
        ),
        JavaScriptNode::ClassDeclaration | JavaScriptNode::ClassExpression => by_kind(
            child,
            &[kinded(JavaScriptNode::ClassBody, Field::Body)],
            by_marker(tree, at, &[marked(EXTENDS, Field::SuperClass)], Field::Id),
        ),
        JavaScriptNode::JSXElement => by_kind(
            child,
            &[
                kinded(JavaScriptNode::JSXOpeningElement, Field::OpeningElement),
                kinded(JavaScriptNode::JSXClosingElement, Field::ClosingElement),
            ],
            Field::Children,
        ),
        JavaScriptNode::JSXFragment => by_kind(
            child,
            &[
                kinded(JavaScriptNode::JSXOpeningFragment, Field::OpeningFragment),
                kinded(JavaScriptNode::JSXClosingFragment, Field::ClosingFragment),
            ],
            Field::Children,
        ),
        JavaScriptNode::JSXOpeningElement => nth(&[Field::Name, Field::Attributes], ordinal),

        // Nodes that hold no node in a property.
        JavaScriptNode::Identifier
        | JavaScriptNode::PrivateIdentifier
        | JavaScriptNode::Literal
        | JavaScriptNode::Super
        | JavaScriptNode::ThisExpression
        | JavaScriptNode::TemplateElement
        | JavaScriptNode::DebuggerStatement
        | JavaScriptNode::EmptyStatement
        | JavaScriptNode::JSXEmptyExpression
        | JavaScriptNode::JSXIdentifier
        | JavaScriptNode::JSXText
        | JavaScriptNode::JSXOpeningFragment
        | JavaScriptNode::JSXClosingFragment => Field::None,
    }
}

/// The role of the element at `at` under its parent; `None` for the root, for a token that spells no operator, and for an extra.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: Position) -> Field {
    let holder = parent(tree, at);
    if holder == at {
        return Field::None;
    }
    let Kind::Node(node) = kind(tree, holder) else {
        return Field::None;
    };
    let child = kind(tree, at);
    match child {
        Kind::Token(_) if operator_of(tree, holder) == Some(at) => Field::Operator,
        Kind::Token(_) | Kind::Extra(_) => Field::None,
        Kind::Node(_) => held(tree, at, node, child),
    }
}

/// The number of the role of the element at `at`, for the table to hold.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
