//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::UNREAD_GRAMMAR;
use crate::domain::syntax::service::{kind_bits, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::PatternNode;
use crate::domain::tokens::types::Token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(format!("{kind:?}").into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. The match has no wildcard arm, so a kind the
/// grammar gains is a compile error until it is shown.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "Every standard kind has one explicit witness or unread reason in this exhaustive table"
)]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(Node::Program | Node::Identifier) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/reference/input.ts"
        ))),
        Kind::Node(
            Node::AccessorProperty
            | Node::AssignmentExpression
            | Node::ClassExpression
            | Node::FunctionExpression
            | Node::MethodDefinition
            | Node::NewExpression
            | Node::PropertyDefinition
            | Node::TSEmptyBodyFunctionExpression
            | Node::TSEnumMember
            | Node::TSMethodSignature,
        ) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/syntax_tree/input.ts"
        ))),
        Kind::Node(
            Node::ArrayExpression
            | Node::ArrayPattern
            | Node::AssignmentPattern
            | Node::AwaitExpression
            | Node::BinaryExpression
            | Node::BreakStatement
            | Node::CatchClause
            | Node::ChainExpression
            | Node::ClassBody
            | Node::ConditionalExpression
            | Node::ContinueStatement
            | Node::DebuggerStatement
            | Node::DoWhileStatement
            | Node::EmptyStatement
            | Node::ExportAllDeclaration
            | Node::ExportDefaultDeclaration
            | Node::ExportNamedDeclaration
            | Node::ExportSpecifier
            | Node::ExpressionStatement
            | Node::ForInStatement
            | Node::ForOfStatement
            | Node::ForStatement
            | Node::IfStatement
            | Node::ImportAttribute
            | Node::ImportExpression
            | Node::JSXAttribute
            | Node::JSXClosingElement
            | Node::JSXClosingFragment
            | Node::JSXElement
            | Node::JSXEmptyExpression
            | Node::JSXExpressionContainer
            | Node::JSXFragment
            | Node::JSXIdentifier
            | Node::JSXMemberExpression
            | Node::JSXNamespacedName
            | Node::JSXOpeningElement
            | Node::JSXOpeningFragment
            | Node::JSXSpreadAttribute
            | Node::JSXSpreadChild
            | Node::JSXText
            | Node::LabeledStatement
            | Node::LogicalExpression
            | Node::MetaProperty
            | Node::ObjectExpression
            | Node::ObjectPattern
            | Node::PrivateIdentifier
            | Node::Property
            | Node::RestElement
            | Node::ReturnStatement
            | Node::SequenceExpression
            | Node::SpreadElement
            | Node::StaticBlock
            | Node::Super
            | Node::SwitchCase
            | Node::SwitchStatement
            | Node::TaggedTemplateExpression
            | Node::TemplateElement
            | Node::TemplateLiteral
            | Node::ThisExpression
            | Node::ThrowStatement
            | Node::TryStatement
            | Node::UnaryExpression
            | Node::UpdateExpression
            | Node::VariableDeclaration
            | Node::WhileStatement
            | Node::WithStatement
            | Node::YieldExpression
            | Node::TSAbstractAccessorProperty
            | Node::TSAbstractKeyword
            | Node::TSAbstractMethodDefinition
            | Node::TSAbstractPropertyDefinition
            | Node::TSAnyKeyword
            | Node::TSArrayType
            | Node::TSAsExpression
            | Node::TSAsyncKeyword
            | Node::TSBigIntKeyword
            | Node::TSBooleanKeyword
            | Node::TSCallSignatureDeclaration
            | Node::TSClassImplements
            | Node::TSConditionalType
            | Node::TSConstructorType
            | Node::TSConstructSignatureDeclaration
            | Node::TSDeclareKeyword
            | Node::TSEnumBody
            | Node::TSExportAssignment
            | Node::TSExportKeyword
            | Node::TSExternalModuleReference
            | Node::TSFunctionType
            | Node::TSImportEqualsDeclaration
            | Node::TSImportType
            | Node::TSIndexedAccessType
            | Node::TSIndexSignature
            | Node::TSInferType
            | Node::TSInstantiationExpression
            | Node::TSInterfaceBody
            | Node::TSInterfaceHeritage
            | Node::TSIntersectionType
            | Node::TSIntrinsicKeyword
            | Node::TSLiteralType
            | Node::TSMappedType
            | Node::TSNamedTupleMember
            | Node::TSNamespaceExportDeclaration
            | Node::TSNeverKeyword
            | Node::TSNonNullExpression
            | Node::TSNullKeyword
            | Node::TSNumberKeyword
            | Node::TSObjectKeyword
            | Node::TSOptionalType
            | Node::TSParameterProperty
            | Node::TSPrivateKeyword
            | Node::TSProtectedKeyword
            | Node::TSPublicKeyword
            | Node::TSReadonlyKeyword
            | Node::TSRestType
            | Node::TSSatisfiesExpression
            | Node::TSStaticKeyword
            | Node::TSStringKeyword
            | Node::TSSymbolKeyword
            | Node::TSTemplateLiteralType
            | Node::TSThisType
            | Node::TSTupleType
            | Node::TSTypeAssertion
            | Node::TSTypeLiteral
            | Node::TSTypeOperator
            | Node::TSTypeParameterDeclaration
            | Node::TSTypeParameterInstantiation
            | Node::TSTypePredicate
            | Node::TSTypeQuery
            | Node::TSTypeReference
            | Node::TSUndefinedKeyword
            | Node::TSUnionType
            | Node::TSUnknownKeyword
            | Node::TSVoidKeyword,
        )
        | Kind::Token(
            Token::Boolean
            | Token::IdentifierToken
            | Token::JSXIdentifierToken
            | Token::PrivateIdentifierToken
            | Token::JSXTextToken
            | Token::Keyword
            | Token::Null
            | Token::Numeric
            | Token::Punctuator
            | Token::RegularExpression
            | Token::String
            | Token::Template
            | Token::Block,
        )
        | Kind::Extra(Extra::Whitespace | Extra::Hashbang | Extra::Error) => {
            Sample::Unread(Why(UNREAD_GRAMMAR))
        }
        Kind::Pattern(
            PatternNode::Pattern
            | PatternNode::Disjunction
            | PatternNode::Alternative
            | PatternNode::BoundaryAssertion
            | PatternNode::LookAroundAssertion
            | PatternNode::Quantifier
            | PatternNode::Character
            | PatternNode::Dot
            | PatternNode::CharacterClassEscape
            | PatternNode::UnicodePropertyEscape
            | PatternNode::CharacterClass
            | PatternNode::CharacterClassRange
            | PatternNode::ClassStringDisjunction
            | PatternNode::ClassString
            | PatternNode::CapturingGroup
            | PatternNode::IgnoreGroup
            | PatternNode::IndexedReference
            | PatternNode::NamedReference,
        ) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/pattern_syntax/input.ts"
        ))),
        Kind::Node(Node::ArrowFunctionExpression) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSTypeParameter-arrow-const-modifier.ts"
        ))),
        Kind::Node(Node::BlockStatement | Node::FunctionDeclaration) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/scope/input.ts"),
        )),
        Kind::Node(Node::CallExpression | Node::MemberExpression) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/static_call/input.ts"),
        )),
        Kind::Node(Node::ClassDeclaration | Node::Decorator) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/decorates/input.ts"),
        )),
        Kind::Node(Node::ImportDeclaration | Node::ImportSpecifier) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/static_import/input.ts"),
        )),
        Kind::Node(Node::ImportDefaultSpecifier) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/ImportDeclaration-default-and-named-many.ts"
        ))),
        Kind::Node(Node::ImportNamespaceSpecifier) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/ImportDeclaration-assertion.ts"
        ))),
        Kind::Node(Node::Literal | Node::VariableDeclarator) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/defines/input.ts"),
        )),
        Kind::Node(Node::TSDeclareFunction) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSDeclareFunction-async-ambient.ts"
        ))),
        Kind::Node(Node::TSEnumDeclaration) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSEnumDeclaration-const.ts"
        ))),
        Kind::Node(Node::TSInterfaceDeclaration) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSInterfaceDeclaration-empty.ts"
        ))),
        Kind::Node(Node::TSModuleBlock | Node::TSModuleDeclaration) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/suite/valid/TSModuleDeclaration-global.ts"),
        )),
        Kind::Node(Node::TSPropertySignature | Node::TSQualifiedName) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/corpus/TSTupleType.ts"),
        )),
        Kind::Node(Node::TSTypeAliasDeclaration | Node::TSTypeParameter) => {
            Sample::Node(SampleText(include_str!(
                "../../tests/fixtures/suite/valid/TSTypeAliasDeclaration-type-param-many.ts"
            )))
        }
        Kind::Node(Node::TSTypeAnnotation) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/annotates/input.ts"
        ))),
        Kind::Token(Token::Line) => Sample::Token(SampleText(include_str!(
            "../../tests/fixtures/relations/comment/input.ts"
        ))),
    }
}
