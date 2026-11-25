//! The holding of the numbered writer: the role of a child from its parent's kind and its siblings, as `holding` reads it from a finished tree.

use super::super::constants::{
    ARROW, AS, COLON, DOT, EQUALS, EXTENDS, FROM, IN, OPEN_PARENTHESIS, SEMICOLON,
};
use super::placing::{by_kind, by_marker, count_before, kinded, marked, nth, place, placed_nth, sharing};
use super::super::types::{Field, Lastness, Sharing};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use super::Family;

/// The role of a child of a property: the default of a shorthand pattern (`{ a = 1 }`) is its value and the name beside it its key,
/// though the wider default comes first; any other child is the key, then the value.
pub(super) fn property(fam: &Family<'_>, at: usize, child: Kind) -> Field {
    let defaulted = fam.kinds.contains(&Kind::Node(Node::AssignmentPattern));
    if child == Kind::Node(Node::AssignmentPattern) {
        Field::Value
    } else if defaulted {
        Field::Key
    } else {
        placed_nth(fam, at, &[Field::Key, Field::Value])
    }
}

/// The role of a child of an export specifier: `export { a }` holds one name in both properties, the two nodes of one range, which
/// TypeScript-ESTree orders `exported` first; `export { a as b }` writes the local name first.
pub(super) fn export_specifier(fam: &Family<'_>, at: usize) -> Field {
    match sharing(fam, at) {
        Sharing::Shared => placed_nth(fam, at, &[Field::Exported, Field::Local]),
        Sharing::Apart => placed_nth(fam, at, &[Field::Local, Field::Exported]),
    }
}

/// The role of a node under a node of `holder` kind that `aside` does not settle.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind of TypeScript-ESTree node: the table of the properties is the proof, and splitting it would split it"
)]
pub(super) fn held(
    fam: &Family<'_>,
    at: usize,
    holder: Node,
    child: Kind,
) -> Field {
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
        Node::SwitchCase => by_marker(
            fam,
            at,
            &[marked(COLON, Field::Consequent)],
            Field::Test,
        ),
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
            by_marker(
                fam,
                at,
                &[marked(FROM, Field::Source)],
                Field::Declaration,
            ),
        ),
        Node::ExportAllDeclaration => by_kind(
            child,
            &[kinded(Node::ImportAttribute, Field::Attributes)],
            by_marker(
                fam,
                at,
                &[marked(FROM, Field::Source)],
                Field::Exported,
            ),
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
        | Node::TSQualifiedName => placed_nth(fam, at, &[Field::Left, Field::Right]),
        Node::ConditionalExpression | Node::IfStatement => placed_nth(
            fam,
            at,
            &[Field::Test, Field::Consequent, Field::Alternate],
        ),
        Node::MemberExpression | Node::JSXMemberExpression => {
            placed_nth(fam, at, &[Field::Object, Field::Property])
        }
        Node::WhileStatement => placed_nth(fam, at, &[Field::Test, Field::Body]),
        Node::DoWhileStatement => placed_nth(fam, at, &[Field::Body, Field::Test]),
        Node::WithStatement => placed_nth(fam, at, &[Field::Object, Field::Body]),
        Node::LabeledStatement => placed_nth(fam, at, &[Field::Label, Field::Body]),
        Node::ForInStatement | Node::ForOfStatement => {
            placed_nth(fam, at, &[Field::Left, Field::Right, Field::Body])
        }
        Node::ImportSpecifier => placed_nth(fam, at, &[Field::Imported, Field::Local]),
        Node::ExportSpecifier => export_specifier(fam, at),
        Node::Property => property(fam, at, child),
        Node::ImportAttribute
        | Node::PropertyDefinition
        | Node::AccessorProperty
        | Node::TSAbstractPropertyDefinition
        | Node::TSAbstractAccessorProperty => placed_nth(fam, at, &[Field::Key, Field::Value]),
        Node::JSXAttribute => placed_nth(fam, at, &[Field::Name, Field::Value]),
        Node::JSXNamespacedName => placed_nth(fam, at, &[Field::Namespace, Field::Name]),
        Node::ImportExpression => placed_nth(fam, at, &[Field::Source, Field::Options]),
        Node::VariableDeclarator => placed_nth(fam, at, &[Field::Id, Field::Init]),
        Node::TSEnumMember => placed_nth(fam, at, &[Field::Id, Field::Initializer]),
        Node::TSImportEqualsDeclaration => {
            placed_nth(fam, at, &[Field::Id, Field::ModuleReference])
        }
        Node::TSIndexedAccessType => placed_nth(fam, at, &[Field::ObjectType, Field::IndexType]),
        Node::TSNamedTupleMember => placed_nth(fam, at, &[Field::Label, Field::ElementType]),
        Node::TSConditionalType => placed_nth(
            fam,
            at,
            &[
                Field::CheckType,
                Field::ExtendsType,
                Field::TrueType,
                Field::FalseType,
            ],
        ),
        Node::TSAsExpression | Node::TSSatisfiesExpression => {
            placed_nth(fam, at, &[Field::Expression, Field::TypeAnnotation])
        }
        Node::TSTypeAssertion => placed_nth(fam, at, &[Field::TypeAnnotation, Field::Expression]),
        Node::TSTypeAliasDeclaration => placed_nth(fam, at, &[Field::Id, Field::TypeAnnotation]),
        Node::CallExpression | Node::NewExpression => {
            placed_nth(fam, at, &[Field::Callee, Field::Arguments])
        }
        Node::TaggedTemplateExpression => placed_nth(fam, at, &[Field::Tag, Field::Quasi]),
        // A method's function is its last node: what comes before is its key.
        Node::MethodDefinition | Node::TSAbstractMethodDefinition => match place(fam, at).lastness
        {
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
            placed_nth(fam, at, &[Field::Block, Field::Finalizer]),
        ),
        // A `for` writes each of its three clauses only when it has one: the semicolons before a clause say which it is; its body is
        // its last node.
        Node::ForStatement => match place(fam, at).lastness {
            Lastness::Last => Field::Body,
            Lastness::NotLast => nth(
                &[Field::Init, Field::Test, Field::Update],
                count_before(fam, at, SEMICOLON),
            ),
        },
        Node::ArrowFunctionExpression => by_marker(
            fam,
            at,
            &[marked(ARROW, Field::Body)],
            Field::Params,
        ),
        Node::FunctionDeclaration
        | Node::FunctionExpression
        | Node::TSDeclareFunction
        | Node::TSEmptyBodyFunctionExpression => by_kind(
            child,
            &[kinded(Node::BlockStatement, Field::Body)],
            by_marker(
                fam,
                at,
                &[marked(OPEN_PARENTHESIS, Field::Params)],
                Field::Id,
            ),
        ),
        Node::TSMethodSignature => by_marker(
            fam,
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
            by_marker(
                fam,
                at,
                &[marked(EXTENDS, Field::SuperClass)],
                Field::Id,
            ),
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
            by_marker(
                fam,
                at,
                &[marked(DOT, Field::Qualifier)],
                Field::Source,
            ),
        ),
        Node::TSMappedType => by_marker(
            fam,
            at,
            &[
                marked(COLON, Field::TypeAnnotation),
                marked(AS, Field::NameType),
                marked(IN, Field::Constraint),
            ],
            Field::Key,
        ),
        Node::TSTypeParameter => by_marker(
            fam,
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
        Node::JSXOpeningElement => placed_nth(fam, at, &[Field::Name, Field::Attributes]),
        Node::MetaProperty => placed_nth(fam, at, &[Field::Meta, Field::Property]),
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
