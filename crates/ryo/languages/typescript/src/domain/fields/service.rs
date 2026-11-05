// one arm for each kind of TypeScript-ESTree node, each naming the property that holds the child: the table of the properties is the proof, and splitting it would split that table
//! The `fields` service: free functions named by verb that say the role of an element under its parent. The tree is TypeScript-ESTree's,
//! and the role of a child is the property of its parent that holds it (`@typescript-eslint/visitor-keys`). The parser's visitor reports
//! no property, so a role is read from the kinds of the parent and the child, from where the child stands among the nodes beside it, and,
//! where a property may be left out, from the token that comes before the child (`(` before parameters, `extends` before a superclass).

use ryo::languages::{self, Position, Tree};
use strum::IntoEnumIterator as _;

use super::constants::{
    ARROW, AS, COLON, DOT, EQUALS, EXTENDS, FROM, IN, OPEN_PARENTHESIS, PARENTHESES, SEMICOLON,
};
use super::types::{
    Field, KindRole, Lastness, Marker, MarkerRole, Operands, Ordinal, Precedence, Returning,
    Setting, Sharing, Standing,
};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::service::{is_token, kind, parent};
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

/// The bytes of the element at `at`, as the file spells them.
fn spelled<'a>(tree: &Tree<'a>, at: Position) -> languages::NameText<'a> {
    let from = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.source
            .0
            .get(from..to)
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .unwrap_or_default(),
    )
}

/// Whether a child of this kind is held in a property of its own whatever its place: a decorator, an annotation, a type parameter list
/// or a type argument list.
const fn setting(child: Kind) -> Setting {
    if matches!(
        child,
        Kind::Node(
            TypeScriptNode::Decorator
                | TypeScriptNode::TSTypeAnnotation
                | TypeScriptNode::TSTypeParameterDeclaration
                | TypeScriptNode::TSTypeParameterInstantiation
        )
    ) {
        Setting::Aside
    } else {
        Setting::Placed
    }
}

/// Where `at` stands among the nodes under its parent, `aside` ones left out, and whether it is the last of them.
fn place(tree: &Tree<'_>, at: Position) -> Standing {
    let placed: Vec<Position> = languages::children(tree.elements, parent(tree, at))
        .filter(|each| {
            let each = *each;
            matches!(kind(tree, each), Kind::Node(_))
                && setting(kind(tree, each)) == Setting::Placed
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
            is_token(tree, each) == Leaf::Token && spelled(tree, each).0 == marker.0
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

/// Whether the nodes under the parent of `at` all start where it does: a shorthand that holds one name in two properties
/// (`export { a }`).
fn sharing(tree: &Tree<'_>, at: Position) -> Sharing {
    let start = languages::start_at(tree.elements, at);
    if languages::children(tree.elements, parent(tree, at))
        .filter(|each| matches!(kind(tree, *each), Kind::Node(_)))
        .all(|each| languages::start_at(tree.elements, each) == start)
    {
        Sharing::Shared
    } else {
        Sharing::Apart
    }
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
            TypeScriptNode::BinaryExpression | TypeScriptNode::LogicalExpression | TypeScriptNode::AssignmentExpression,
        ) => Operands::Binary,
        Kind::Node(TypeScriptNode::UnaryExpression | TypeScriptNode::UpdateExpression) => Operands::Unary,
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
                .any(|marker| spelled(tree, each).0 == marker.0)
    })
}

/// Whether a type annotation under a node of `holder` kind says what it returns.
const fn returning(holder: TypeScriptNode) -> Returning {
    if matches!(
        holder,
        TypeScriptNode::FunctionDeclaration
            | TypeScriptNode::FunctionExpression
            | TypeScriptNode::ArrowFunctionExpression
            | TypeScriptNode::TSDeclareFunction
            | TypeScriptNode::TSEmptyBodyFunctionExpression
            | TypeScriptNode::TSCallSignatureDeclaration
            | TypeScriptNode::TSConstructSignatureDeclaration
            | TypeScriptNode::TSFunctionType
            | TypeScriptNode::TSConstructorType
            | TypeScriptNode::TSMethodSignature
    ) {
        Returning::Returns
    } else {
        Returning::Holds
    }
}

/// The role a child of the kind `node` plays.
const fn kinded(node: TypeScriptNode, field: Field) -> KindRole {
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
        .any(|each| kind(tree, each) == Kind::Node(TypeScriptNode::AssignmentPattern));
    if child == Kind::Node(TypeScriptNode::AssignmentPattern) {
        Field::Value
    } else if defaulted {
        Field::Key
    } else {
        nth(&[Field::Key, Field::Value], ordinal)
    }
}

/// The role of a child of an export specifier: `export { a }` holds one name in both properties, the two nodes of one range, which
/// TypeScript-ESTree orders `exported` first; `export { a as b }` writes the local name first.
fn export_specifier(tree: &Tree<'_>, at: Position, ordinal: Ordinal) -> Field {
    match sharing(tree, at) {
        Sharing::Shared => nth(&[Field::Exported, Field::Local], ordinal),
        Sharing::Apart => nth(&[Field::Local, Field::Exported], ordinal),
    }
}

/// The role of a node under a node of `holder` kind that `aside` does not settle.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of TypeScript-ESTree node: the table of the properties is the proof, and splitting it would split it"
)]
fn held(tree: &Tree<'_>, at: Position, holder: TypeScriptNode, child: Kind) -> Field {
    let Standing { ordinal, lastness } = place(tree, at);
    match holder {
        TypeScriptNode::Program
        | TypeScriptNode::BlockStatement
        | TypeScriptNode::ClassBody
        | TypeScriptNode::StaticBlock
        | TypeScriptNode::TSInterfaceBody
        | TypeScriptNode::TSModuleBlock => Field::Body,
        TypeScriptNode::ArrayExpression | TypeScriptNode::ArrayPattern => Field::Elements,
        TypeScriptNode::ObjectExpression | TypeScriptNode::ObjectPattern => Field::Properties,
        TypeScriptNode::SequenceExpression => Field::Expressions,
        TypeScriptNode::TSEnumBody | TypeScriptNode::TSTypeLiteral => Field::Members,
        TypeScriptNode::TSTupleType => Field::ElementTypes,
        TypeScriptNode::TSUnionType | TypeScriptNode::TSIntersectionType => Field::Types,
        TypeScriptNode::TemplateLiteral => by_kind(
            child,
            &[kinded(TypeScriptNode::TemplateElement, Field::Quasis)],
            Field::Expressions,
        ),
        TypeScriptNode::TSTemplateLiteralType => by_kind(
            child,
            &[kinded(TypeScriptNode::TemplateElement, Field::Quasis)],
            Field::Types,
        ),
        TypeScriptNode::VariableDeclaration => Field::Declarations,
        TypeScriptNode::SwitchStatement => by_kind(
            child,
            &[kinded(TypeScriptNode::SwitchCase, Field::Cases)],
            Field::Discriminant,
        ),
        TypeScriptNode::SwitchCase => by_marker(tree, at, &[marked(COLON, Field::Consequent)], Field::Test),
        TypeScriptNode::ImportDeclaration => by_kind(
            child,
            &[
                kinded(TypeScriptNode::ImportSpecifier, Field::Specifiers),
                kinded(TypeScriptNode::ImportDefaultSpecifier, Field::Specifiers),
                kinded(TypeScriptNode::ImportNamespaceSpecifier, Field::Specifiers),
                kinded(TypeScriptNode::ImportAttribute, Field::Attributes),
            ],
            Field::Source,
        ),
        TypeScriptNode::ExportNamedDeclaration => by_kind(
            child,
            &[
                kinded(TypeScriptNode::ExportSpecifier, Field::Specifiers),
                kinded(TypeScriptNode::ImportAttribute, Field::Attributes),
            ],
            by_marker(tree, at, &[marked(FROM, Field::Source)], Field::Declaration),
        ),
        TypeScriptNode::ExportAllDeclaration => by_kind(
            child,
            &[kinded(TypeScriptNode::ImportAttribute, Field::Attributes)],
            by_marker(tree, at, &[marked(FROM, Field::Source)], Field::Exported),
        ),
        TypeScriptNode::ExportDefaultDeclaration => Field::Declaration,
        TypeScriptNode::ChainExpression
        | TypeScriptNode::Decorator
        | TypeScriptNode::ExpressionStatement
        | TypeScriptNode::JSXExpressionContainer
        | TypeScriptNode::JSXSpreadChild
        | TypeScriptNode::TSExportAssignment
        | TypeScriptNode::TSExternalModuleReference
        | TypeScriptNode::TSNonNullExpression
        | TypeScriptNode::TSClassImplements
        | TypeScriptNode::TSInterfaceHeritage
        | TypeScriptNode::TSInstantiationExpression => Field::Expression,
        TypeScriptNode::AwaitExpression
        | TypeScriptNode::ReturnStatement
        | TypeScriptNode::ThrowStatement
        | TypeScriptNode::SpreadElement
        | TypeScriptNode::RestElement
        | TypeScriptNode::UnaryExpression
        | TypeScriptNode::UpdateExpression
        | TypeScriptNode::YieldExpression
        | TypeScriptNode::JSXSpreadAttribute => Field::Argument,
        TypeScriptNode::BreakStatement | TypeScriptNode::ContinueStatement => Field::Label,
        TypeScriptNode::TSArrayType => Field::ElementType,
        TypeScriptNode::TSOptionalType | TypeScriptNode::TSRestType | TypeScriptNode::TSTypeOperator | TypeScriptNode::TSTypeAnnotation => {
            Field::TypeAnnotation
        }
        TypeScriptNode::TSLiteralType => Field::Literal,
        TypeScriptNode::TSInferType => Field::TypeParameter,
        TypeScriptNode::TSNamespaceExportDeclaration => Field::Id,
        TypeScriptNode::TSParameterProperty => Field::Parameter,
        TypeScriptNode::TSTypeReference => Field::TypeName,
        TypeScriptNode::TSTypeQuery => Field::ExprName,
        TypeScriptNode::JSXClosingElement => Field::Name,
        TypeScriptNode::ImportDefaultSpecifier | TypeScriptNode::ImportNamespaceSpecifier => Field::Local,
        TypeScriptNode::TSIndexSignature => Field::Parameters,
        TypeScriptNode::TSPropertySignature => Field::Key,
        TypeScriptNode::TSTypePredicate => Field::ParameterName,
        TypeScriptNode::AssignmentExpression
        | TypeScriptNode::BinaryExpression
        | TypeScriptNode::LogicalExpression
        | TypeScriptNode::AssignmentPattern
        | TypeScriptNode::TSQualifiedName => nth(&[Field::Left, Field::Right], ordinal),
        TypeScriptNode::ConditionalExpression | TypeScriptNode::IfStatement => {
            nth(&[Field::Test, Field::Consequent, Field::Alternate], ordinal)
        }
        TypeScriptNode::MemberExpression | TypeScriptNode::JSXMemberExpression => {
            nth(&[Field::Object, Field::Property], ordinal)
        }
        TypeScriptNode::WhileStatement => nth(&[Field::Test, Field::Body], ordinal),
        TypeScriptNode::DoWhileStatement => nth(&[Field::Body, Field::Test], ordinal),
        TypeScriptNode::WithStatement => nth(&[Field::Object, Field::Body], ordinal),
        TypeScriptNode::LabeledStatement => nth(&[Field::Label, Field::Body], ordinal),
        TypeScriptNode::ForInStatement | TypeScriptNode::ForOfStatement => {
            nth(&[Field::Left, Field::Right, Field::Body], ordinal)
        }
        TypeScriptNode::ImportSpecifier => nth(&[Field::Imported, Field::Local], ordinal),
        TypeScriptNode::ExportSpecifier => export_specifier(tree, at, ordinal),
        TypeScriptNode::Property => property(tree, at, child, ordinal),
        TypeScriptNode::ImportAttribute
        | TypeScriptNode::PropertyDefinition
        | TypeScriptNode::AccessorProperty
        | TypeScriptNode::TSAbstractPropertyDefinition
        | TypeScriptNode::TSAbstractAccessorProperty => nth(&[Field::Key, Field::Value], ordinal),
        TypeScriptNode::JSXAttribute => nth(&[Field::Name, Field::Value], ordinal),
        TypeScriptNode::JSXNamespacedName => nth(&[Field::Namespace, Field::Name], ordinal),
        TypeScriptNode::ImportExpression => nth(&[Field::Source, Field::Options], ordinal),
        TypeScriptNode::VariableDeclarator => nth(&[Field::Id, Field::Init], ordinal),
        TypeScriptNode::TSEnumMember => nth(&[Field::Id, Field::Initializer], ordinal),
        TypeScriptNode::TSImportEqualsDeclaration => nth(&[Field::Id, Field::ModuleReference], ordinal),
        TypeScriptNode::TSIndexedAccessType => nth(&[Field::ObjectType, Field::IndexType], ordinal),
        TypeScriptNode::TSNamedTupleMember => nth(&[Field::Label, Field::ElementType], ordinal),
        TypeScriptNode::TSConditionalType => nth(
            &[
                Field::CheckType,
                Field::ExtendsType,
                Field::TrueType,
                Field::FalseType,
            ],
            ordinal,
        ),
        TypeScriptNode::TSAsExpression | TypeScriptNode::TSSatisfiesExpression => {
            nth(&[Field::Expression, Field::TypeAnnotation], ordinal)
        }
        TypeScriptNode::TSTypeAssertion => nth(&[Field::TypeAnnotation, Field::Expression], ordinal),
        TypeScriptNode::TSTypeAliasDeclaration => nth(&[Field::Id, Field::TypeAnnotation], ordinal),
        TypeScriptNode::CallExpression | TypeScriptNode::NewExpression => {
            nth(&[Field::Callee, Field::Arguments], ordinal)
        }
        TypeScriptNode::TaggedTemplateExpression => nth(&[Field::Tag, Field::Quasi], ordinal),
        // A method's function is its last node: what comes before is its key.
        TypeScriptNode::MethodDefinition | TypeScriptNode::TSAbstractMethodDefinition => match lastness {
            Lastness::Last => Field::Value,
            Lastness::NotLast => Field::Key,
        },
        TypeScriptNode::TSModuleDeclaration => by_kind(
            child,
            &[kinded(TypeScriptNode::TSModuleBlock, Field::Body)],
            Field::Id,
        ),
        TypeScriptNode::TSEnumDeclaration => {
            by_kind(child, &[kinded(TypeScriptNode::TSEnumBody, Field::Body)], Field::Id)
        }
        TypeScriptNode::CatchClause => by_kind(
            child,
            &[kinded(TypeScriptNode::BlockStatement, Field::Body)],
            Field::Param,
        ),
        TypeScriptNode::TryStatement => by_kind(
            child,
            &[kinded(TypeScriptNode::CatchClause, Field::Handler)],
            nth(&[Field::Block, Field::Finalizer], ordinal),
        ),
        // A `for` writes each of its three clauses only when it has one: the semicolons before a clause say which it is; its body is
        // its last node.
        TypeScriptNode::ForStatement => match lastness {
            Lastness::Last => Field::Body,
            Lastness::NotLast => nth(
                &[Field::Init, Field::Test, Field::Update],
                count_before(tree, at, SEMICOLON),
            ),
        },
        TypeScriptNode::ArrowFunctionExpression => {
            by_marker(tree, at, &[marked(ARROW, Field::Body)], Field::Params)
        }
        TypeScriptNode::FunctionDeclaration
        | TypeScriptNode::FunctionExpression
        | TypeScriptNode::TSDeclareFunction
        | TypeScriptNode::TSEmptyBodyFunctionExpression => by_kind(
            child,
            &[kinded(TypeScriptNode::BlockStatement, Field::Body)],
            by_marker(
                tree,
                at,
                &[marked(OPEN_PARENTHESIS, Field::Params)],
                Field::Id,
            ),
        ),
        TypeScriptNode::TSMethodSignature => by_marker(
            tree,
            at,
            &[marked(OPEN_PARENTHESIS, Field::Params)],
            Field::Key,
        ),
        TypeScriptNode::TSCallSignatureDeclaration
        | TypeScriptNode::TSConstructSignatureDeclaration
        | TypeScriptNode::TSFunctionType
        | TypeScriptNode::TSConstructorType
        | TypeScriptNode::TSTypeParameterDeclaration
        | TypeScriptNode::TSTypeParameterInstantiation => Field::Params,
        TypeScriptNode::ClassDeclaration | TypeScriptNode::ClassExpression => by_kind(
            child,
            &[
                kinded(TypeScriptNode::ClassBody, Field::Body),
                kinded(TypeScriptNode::TSClassImplements, Field::Implements),
            ],
            by_marker(tree, at, &[marked(EXTENDS, Field::SuperClass)], Field::Id),
        ),
        TypeScriptNode::TSInterfaceDeclaration => by_kind(
            child,
            &[
                kinded(TypeScriptNode::TSInterfaceBody, Field::Body),
                kinded(TypeScriptNode::TSInterfaceHeritage, Field::Extends),
            ],
            Field::Id,
        ),
        TypeScriptNode::TSImportType => by_kind(
            child,
            &[kinded(TypeScriptNode::ObjectExpression, Field::Options)],
            by_marker(tree, at, &[marked(DOT, Field::Qualifier)], Field::Source),
        ),
        TypeScriptNode::TSMappedType => by_marker(
            tree,
            at,
            &[
                marked(COLON, Field::TypeAnnotation),
                marked(AS, Field::NameType),
                marked(IN, Field::Constraint),
            ],
            Field::Key,
        ),
        TypeScriptNode::TSTypeParameter => by_marker(
            tree,
            at,
            &[
                marked(EQUALS, Field::Default),
                marked(EXTENDS, Field::Constraint),
            ],
            Field::Name,
        ),
        TypeScriptNode::JSXElement => by_kind(
            child,
            &[
                kinded(TypeScriptNode::JSXOpeningElement, Field::OpeningElement),
                kinded(TypeScriptNode::JSXClosingElement, Field::ClosingElement),
            ],
            Field::Children,
        ),
        TypeScriptNode::JSXFragment => by_kind(
            child,
            &[
                kinded(TypeScriptNode::JSXOpeningFragment, Field::OpeningFragment),
                kinded(TypeScriptNode::JSXClosingFragment, Field::ClosingFragment),
            ],
            Field::Children,
        ),
        TypeScriptNode::JSXOpeningElement => nth(&[Field::Name, Field::Attributes], ordinal),
        TypeScriptNode::MetaProperty => nth(&[Field::Meta, Field::Property], ordinal),
        // Nodes with no child node a property holds besides the decorators and annotations `aside` settles.
        TypeScriptNode::Identifier
        | TypeScriptNode::PrivateIdentifier
        | TypeScriptNode::Literal
        | TypeScriptNode::Super
        | TypeScriptNode::ThisExpression
        | TypeScriptNode::TemplateElement
        | TypeScriptNode::DebuggerStatement
        | TypeScriptNode::EmptyStatement
        | TypeScriptNode::JSXEmptyExpression
        | TypeScriptNode::JSXIdentifier
        | TypeScriptNode::JSXText
        | TypeScriptNode::JSXOpeningFragment
        | TypeScriptNode::JSXClosingFragment
        | TypeScriptNode::TSThisType
        | TypeScriptNode::TSAbstractKeyword
        | TypeScriptNode::TSAnyKeyword
        | TypeScriptNode::TSAsyncKeyword
        | TypeScriptNode::TSBigIntKeyword
        | TypeScriptNode::TSBooleanKeyword
        | TypeScriptNode::TSDeclareKeyword
        | TypeScriptNode::TSExportKeyword
        | TypeScriptNode::TSIntrinsicKeyword
        | TypeScriptNode::TSNeverKeyword
        | TypeScriptNode::TSNullKeyword
        | TypeScriptNode::TSNumberKeyword
        | TypeScriptNode::TSObjectKeyword
        | TypeScriptNode::TSPrivateKeyword
        | TypeScriptNode::TSProtectedKeyword
        | TypeScriptNode::TSPublicKeyword
        | TypeScriptNode::TSReadonlyKeyword
        | TypeScriptNode::TSStaticKeyword
        | TypeScriptNode::TSStringKeyword
        | TypeScriptNode::TSSymbolKeyword
        | TypeScriptNode::TSUndefinedKeyword
        | TypeScriptNode::TSUnknownKeyword
        | TypeScriptNode::TSVoidKeyword => Field::None,
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
        Kind::Node(TypeScriptNode::Decorator) => Field::Decorators,
        Kind::Node(TypeScriptNode::TSTypeAnnotation) if returning(node) == Returning::Returns => {
            Field::ReturnType
        }
        Kind::Node(TypeScriptNode::TSTypeAnnotation) => Field::TypeAnnotation,
        Kind::Node(TypeScriptNode::TSTypeParameterDeclaration) => Field::TypeParameters,
        Kind::Node(TypeScriptNode::TSTypeParameterInstantiation)
            if matches!(node, TypeScriptNode::ClassDeclaration | TypeScriptNode::ClassExpression) =>
        {
            Field::SuperTypeArguments
        }
        Kind::Node(TypeScriptNode::TSTypeParameterInstantiation) => Field::TypeArguments,
        Kind::Node(_) => held(tree, at, node, child),
    }
}

/// The number of the role of the element at `at`, for the table to hold.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
