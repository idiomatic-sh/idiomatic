//! The holding of the adapter: property, export_specifier, held.

use super::super::constants::{
    ARROW, AS, COLON, DOT, EQUALS, EXTENDS, FROM, IN, OPEN_PARENTHESIS, SEMICOLON,
};
use super::super::placing::service::{
    by_kind, by_marker, count_before, kinded, marked, nth, place, sharing,
};
use super::super::types::{Field, Lastness, Ordinal, Sharing, Standing};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, Position, Tree};

/// The role of a child of a property: the default of a shorthand pattern (`{ a = 1 }`) is its value and the name beside it its key,
/// though the wider default comes first; any other child is the key, then the value.
pub(in super::super) fn property(
    tree: &Tree<'_>,
    at: Position,
    child: Kind,
    ordinal: Ordinal,
) -> Field {
    let defaulted = languages::children(tree.elements, parent(tree, at))
        .any(|each| kind(tree, each) == Kind::Node(Node::AssignmentPattern));
    if child == Kind::Node(Node::AssignmentPattern) {
        Field::Value
    } else if defaulted {
        Field::Key
    } else {
        nth(&[Field::Key, Field::Value], ordinal)
    }
}

/// The role of a child of an export specifier: `export { a }` holds one name in both properties, the two nodes of one range, which
/// TypeScript-ESTree orders `exported` first; `export { a as b }` writes the local name first.
pub(in super::super) fn export_specifier(tree: &Tree<'_>, at: Position, ordinal: Ordinal) -> Field {
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
pub(in super::super) fn held(tree: &Tree<'_>, at: Position, holder: Node, child: Kind) -> Field {
    let Standing { ordinal, lastness } = place(tree, at);
    match holder {
        Node::Program
        | Node::BlockStatement
        | Node::ClassBody
        | Node::StaticBlock
        | Node::TSInterfaceBody
        | Node::TSModuleBlock => Field::Body,
        Node::ArrayExpression | Node::ArrayPattern => Field::Elements,
        Node::ObjectExpression | Node::ObjectPattern => Field::Properties,
        Node::SequenceExpression => Field::Expressions,
        Node::TSEnumBody | Node::TSTypeLiteral => Field::Members,
        Node::TSTupleType => Field::ElementTypes,
        Node::TSUnionType | Node::TSIntersectionType => Field::Types,
        Node::TemplateLiteral => by_kind(
            child,
            &[kinded(Node::TemplateElement, Field::Quasis)],
            Field::Expressions,
        ),
        Node::TSTemplateLiteralType => by_kind(
            child,
            &[kinded(Node::TemplateElement, Field::Quasis)],
            Field::Types,
        ),
        Node::VariableDeclaration => Field::Declarations,
        Node::SwitchStatement => by_kind(
            child,
            &[kinded(Node::SwitchCase, Field::Cases)],
            Field::Discriminant,
        ),
        Node::SwitchCase => by_marker(tree, at, &[marked(COLON, Field::Consequent)], Field::Test),
        Node::ImportDeclaration => by_kind(
            child,
            &[
                kinded(Node::ImportSpecifier, Field::Specifiers),
                kinded(Node::ImportDefaultSpecifier, Field::Specifiers),
                kinded(Node::ImportNamespaceSpecifier, Field::Specifiers),
                kinded(Node::ImportAttribute, Field::Attributes),
            ],
            Field::Source,
        ),
        Node::ExportNamedDeclaration => by_kind(
            child,
            &[
                kinded(Node::ExportSpecifier, Field::Specifiers),
                kinded(Node::ImportAttribute, Field::Attributes),
            ],
            by_marker(tree, at, &[marked(FROM, Field::Source)], Field::Declaration),
        ),
        Node::ExportAllDeclaration => by_kind(
            child,
            &[kinded(Node::ImportAttribute, Field::Attributes)],
            by_marker(tree, at, &[marked(FROM, Field::Source)], Field::Exported),
        ),
        Node::ExportDefaultDeclaration => Field::Declaration,
        Node::ChainExpression
        | Node::Decorator
        | Node::ExpressionStatement
        | Node::JSXExpressionContainer
        | Node::JSXSpreadChild
        | Node::TSExportAssignment
        | Node::TSExternalModuleReference
        | Node::TSNonNullExpression
        | Node::TSClassImplements
        | Node::TSInterfaceHeritage
        | Node::TSInstantiationExpression => Field::Expression,
        Node::AwaitExpression
        | Node::ReturnStatement
        | Node::ThrowStatement
        | Node::SpreadElement
        | Node::RestElement
        | Node::UnaryExpression
        | Node::UpdateExpression
        | Node::YieldExpression
        | Node::JSXSpreadAttribute => Field::Argument,
        Node::BreakStatement | Node::ContinueStatement => Field::Label,
        Node::TSArrayType => Field::ElementType,
        Node::TSOptionalType | Node::TSRestType | Node::TSTypeOperator | Node::TSTypeAnnotation => {
            Field::TypeAnnotation
        }
        Node::TSLiteralType => Field::Literal,
        Node::TSInferType => Field::TypeParameter,
        Node::TSNamespaceExportDeclaration => Field::Id,
        Node::TSParameterProperty => Field::Parameter,
        Node::TSTypeReference => Field::TypeName,
        Node::TSTypeQuery => Field::ExprName,
        Node::JSXClosingElement => Field::Name,
        Node::ImportDefaultSpecifier | Node::ImportNamespaceSpecifier => Field::Local,
        Node::TSIndexSignature => Field::Parameters,
        Node::TSPropertySignature => Field::Key,
        Node::TSTypePredicate => Field::ParameterName,
        Node::AssignmentExpression
        | Node::BinaryExpression
        | Node::LogicalExpression
        | Node::AssignmentPattern
        | Node::TSQualifiedName => nth(&[Field::Left, Field::Right], ordinal),
        Node::ConditionalExpression | Node::IfStatement => {
            nth(&[Field::Test, Field::Consequent, Field::Alternate], ordinal)
        }
        Node::MemberExpression | Node::JSXMemberExpression => {
            nth(&[Field::Object, Field::Property], ordinal)
        }
        Node::WhileStatement => nth(&[Field::Test, Field::Body], ordinal),
        Node::DoWhileStatement => nth(&[Field::Body, Field::Test], ordinal),
        Node::WithStatement => nth(&[Field::Object, Field::Body], ordinal),
        Node::LabeledStatement => nth(&[Field::Label, Field::Body], ordinal),
        Node::ForInStatement | Node::ForOfStatement => {
            nth(&[Field::Left, Field::Right, Field::Body], ordinal)
        }
        Node::ImportSpecifier => nth(&[Field::Imported, Field::Local], ordinal),
        Node::ExportSpecifier => export_specifier(tree, at, ordinal),
        Node::Property => property(tree, at, child, ordinal),
        Node::ImportAttribute
        | Node::PropertyDefinition
        | Node::AccessorProperty
        | Node::TSAbstractPropertyDefinition
        | Node::TSAbstractAccessorProperty => nth(&[Field::Key, Field::Value], ordinal),
        Node::JSXAttribute => nth(&[Field::Name, Field::Value], ordinal),
        Node::JSXNamespacedName => nth(&[Field::Namespace, Field::Name], ordinal),
        Node::ImportExpression => nth(&[Field::Source, Field::Options], ordinal),
        Node::VariableDeclarator => nth(&[Field::Id, Field::Init], ordinal),
        Node::TSEnumMember => nth(&[Field::Id, Field::Initializer], ordinal),
        Node::TSImportEqualsDeclaration => nth(&[Field::Id, Field::ModuleReference], ordinal),
        Node::TSIndexedAccessType => nth(&[Field::ObjectType, Field::IndexType], ordinal),
        Node::TSNamedTupleMember => nth(&[Field::Label, Field::ElementType], ordinal),
        Node::TSConditionalType => nth(
            &[
                Field::CheckType,
                Field::ExtendsType,
                Field::TrueType,
                Field::FalseType,
            ],
            ordinal,
        ),
        Node::TSAsExpression | Node::TSSatisfiesExpression => {
            nth(&[Field::Expression, Field::TypeAnnotation], ordinal)
        }
        Node::TSTypeAssertion => nth(&[Field::TypeAnnotation, Field::Expression], ordinal),
        Node::TSTypeAliasDeclaration => nth(&[Field::Id, Field::TypeAnnotation], ordinal),
        Node::CallExpression | Node::NewExpression => {
            nth(&[Field::Callee, Field::Arguments], ordinal)
        }
        Node::TaggedTemplateExpression => nth(&[Field::Tag, Field::Quasi], ordinal),
        // A method's function is its last node: what comes before is its key.
        Node::MethodDefinition | Node::TSAbstractMethodDefinition => match lastness {
            Lastness::Last => Field::Value,
            Lastness::NotLast => Field::Key,
        },
        Node::TSModuleDeclaration => by_kind(
            child,
            &[kinded(Node::TSModuleBlock, Field::Body)],
            Field::Id,
        ),
        Node::TSEnumDeclaration => {
            by_kind(child, &[kinded(Node::TSEnumBody, Field::Body)], Field::Id)
        }
        Node::CatchClause => by_kind(
            child,
            &[kinded(Node::BlockStatement, Field::Body)],
            Field::Param,
        ),
        Node::TryStatement => by_kind(
            child,
            &[kinded(Node::CatchClause, Field::Handler)],
            nth(&[Field::Block, Field::Finalizer], ordinal),
        ),
        // A `for` writes each of its three clauses only when it has one: the semicolons before a clause say which it is; its body is
        // its last node.
        Node::ForStatement => match lastness {
            Lastness::Last => Field::Body,
            Lastness::NotLast => nth(
                &[Field::Init, Field::Test, Field::Update],
                count_before(tree, at, SEMICOLON),
            ),
        },
        Node::ArrowFunctionExpression => {
            by_marker(tree, at, &[marked(ARROW, Field::Body)], Field::Params)
        }
        Node::FunctionDeclaration
        | Node::FunctionExpression
        | Node::TSDeclareFunction
        | Node::TSEmptyBodyFunctionExpression => by_kind(
            child,
            &[kinded(Node::BlockStatement, Field::Body)],
            by_marker(
                tree,
                at,
                &[marked(OPEN_PARENTHESIS, Field::Params)],
                Field::Id,
            ),
        ),
        Node::TSMethodSignature => by_marker(
            tree,
            at,
            &[marked(OPEN_PARENTHESIS, Field::Params)],
            Field::Key,
        ),
        Node::TSCallSignatureDeclaration
        | Node::TSConstructSignatureDeclaration
        | Node::TSFunctionType
        | Node::TSConstructorType
        | Node::TSTypeParameterDeclaration
        | Node::TSTypeParameterInstantiation => Field::Params,
        Node::ClassDeclaration | Node::ClassExpression => by_kind(
            child,
            &[
                kinded(Node::ClassBody, Field::Body),
                kinded(Node::TSClassImplements, Field::Implements),
            ],
            by_marker(tree, at, &[marked(EXTENDS, Field::SuperClass)], Field::Id),
        ),
        Node::TSInterfaceDeclaration => by_kind(
            child,
            &[
                kinded(Node::TSInterfaceBody, Field::Body),
                kinded(Node::TSInterfaceHeritage, Field::Extends),
            ],
            Field::Id,
        ),
        Node::TSImportType => by_kind(
            child,
            &[kinded(Node::ObjectExpression, Field::Options)],
            by_marker(tree, at, &[marked(DOT, Field::Qualifier)], Field::Source),
        ),
        Node::TSMappedType => by_marker(
            tree,
            at,
            &[
                marked(COLON, Field::TypeAnnotation),
                marked(AS, Field::NameType),
                marked(IN, Field::Constraint),
            ],
            Field::Key,
        ),
        Node::TSTypeParameter => by_marker(
            tree,
            at,
            &[
                marked(EQUALS, Field::Default),
                marked(EXTENDS, Field::Constraint),
            ],
            Field::Name,
        ),
        Node::JSXElement => by_kind(
            child,
            &[
                kinded(Node::JSXOpeningElement, Field::OpeningElement),
                kinded(Node::JSXClosingElement, Field::ClosingElement),
            ],
            Field::Children,
        ),
        Node::JSXFragment => by_kind(
            child,
            &[
                kinded(Node::JSXOpeningFragment, Field::OpeningFragment),
                kinded(Node::JSXClosingFragment, Field::ClosingFragment),
            ],
            Field::Children,
        ),
        Node::JSXOpeningElement => nth(&[Field::Name, Field::Attributes], ordinal),
        Node::MetaProperty => nth(&[Field::Meta, Field::Property], ordinal),
        // Nodes with no child node a property holds besides the decorators and annotations `aside` settles.
        Node::Identifier
        | Node::PrivateIdentifier
        | Node::Literal
        | Node::Super
        | Node::ThisExpression
        | Node::TemplateElement
        | Node::DebuggerStatement
        | Node::EmptyStatement
        | Node::JSXEmptyExpression
        | Node::JSXIdentifier
        | Node::JSXText
        | Node::JSXOpeningFragment
        | Node::JSXClosingFragment
        | Node::TSThisType
        | Node::TSAbstractKeyword
        | Node::TSAnyKeyword
        | Node::TSAsyncKeyword
        | Node::TSBigIntKeyword
        | Node::TSBooleanKeyword
        | Node::TSDeclareKeyword
        | Node::TSExportKeyword
        | Node::TSIntrinsicKeyword
        | Node::TSNeverKeyword
        | Node::TSNullKeyword
        | Node::TSNumberKeyword
        | Node::TSObjectKeyword
        | Node::TSPrivateKeyword
        | Node::TSProtectedKeyword
        | Node::TSPublicKeyword
        | Node::TSReadonlyKeyword
        | Node::TSStaticKeyword
        | Node::TSStringKeyword
        | Node::TSSymbolKeyword
        | Node::TSUndefinedKeyword
        | Node::TSUnknownKeyword
        | Node::TSVoidKeyword => Field::None,
    }
}
