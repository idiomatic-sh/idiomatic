//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::constants::UNREAD_GRAMMAR;
use crate::domain::syntax::service::{kind_bits, kinds};
use crate::domain::syntax::types::Kind;
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
        Kind::Node(TypeScriptNode::Program | TypeScriptNode::Identifier) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/reference/input.ts"
        ))),
        Kind::Node(
            TypeScriptNode::AccessorProperty
            | TypeScriptNode::AssignmentExpression
            | TypeScriptNode::ClassExpression
            | TypeScriptNode::FunctionExpression
            | TypeScriptNode::MethodDefinition
            | TypeScriptNode::NewExpression
            | TypeScriptNode::PropertyDefinition
            | TypeScriptNode::TSEmptyBodyFunctionExpression
            | TypeScriptNode::TSEnumMember
            | TypeScriptNode::TSMethodSignature,
        ) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/syntax_tree/input.ts"
        ))),
        Kind::Node(
            TypeScriptNode::ArrayExpression
            | TypeScriptNode::ArrayPattern
            | TypeScriptNode::AssignmentPattern
            | TypeScriptNode::AwaitExpression
            | TypeScriptNode::BinaryExpression
            | TypeScriptNode::BreakStatement
            | TypeScriptNode::CatchClause
            | TypeScriptNode::ChainExpression
            | TypeScriptNode::ClassBody
            | TypeScriptNode::ConditionalExpression
            | TypeScriptNode::ContinueStatement
            | TypeScriptNode::DebuggerStatement
            | TypeScriptNode::DoWhileStatement
            | TypeScriptNode::EmptyStatement
            | TypeScriptNode::ExportAllDeclaration
            | TypeScriptNode::ExportDefaultDeclaration
            | TypeScriptNode::ExportNamedDeclaration
            | TypeScriptNode::ExportSpecifier
            | TypeScriptNode::ExpressionStatement
            | TypeScriptNode::ForInStatement
            | TypeScriptNode::ForOfStatement
            | TypeScriptNode::ForStatement
            | TypeScriptNode::IfStatement
            | TypeScriptNode::ImportAttribute
            | TypeScriptNode::ImportExpression
            | TypeScriptNode::JSXAttribute
            | TypeScriptNode::JSXClosingElement
            | TypeScriptNode::JSXClosingFragment
            | TypeScriptNode::JSXElement
            | TypeScriptNode::JSXEmptyExpression
            | TypeScriptNode::JSXExpressionContainer
            | TypeScriptNode::JSXFragment
            | TypeScriptNode::JSXIdentifier
            | TypeScriptNode::JSXMemberExpression
            | TypeScriptNode::JSXNamespacedName
            | TypeScriptNode::JSXOpeningElement
            | TypeScriptNode::JSXOpeningFragment
            | TypeScriptNode::JSXSpreadAttribute
            | TypeScriptNode::JSXSpreadChild
            | TypeScriptNode::JSXText
            | TypeScriptNode::LabeledStatement
            | TypeScriptNode::LogicalExpression
            | TypeScriptNode::MetaProperty
            | TypeScriptNode::ObjectExpression
            | TypeScriptNode::ObjectPattern
            | TypeScriptNode::PrivateIdentifier
            | TypeScriptNode::Property
            | TypeScriptNode::RestElement
            | TypeScriptNode::ReturnStatement
            | TypeScriptNode::SequenceExpression
            | TypeScriptNode::SpreadElement
            | TypeScriptNode::StaticBlock
            | TypeScriptNode::Super
            | TypeScriptNode::SwitchCase
            | TypeScriptNode::SwitchStatement
            | TypeScriptNode::TaggedTemplateExpression
            | TypeScriptNode::TemplateElement
            | TypeScriptNode::TemplateLiteral
            | TypeScriptNode::ThisExpression
            | TypeScriptNode::ThrowStatement
            | TypeScriptNode::TryStatement
            | TypeScriptNode::UnaryExpression
            | TypeScriptNode::UpdateExpression
            | TypeScriptNode::VariableDeclaration
            | TypeScriptNode::WhileStatement
            | TypeScriptNode::WithStatement
            | TypeScriptNode::YieldExpression
            | TypeScriptNode::TSAbstractAccessorProperty
            | TypeScriptNode::TSAbstractKeyword
            | TypeScriptNode::TSAbstractMethodDefinition
            | TypeScriptNode::TSAbstractPropertyDefinition
            | TypeScriptNode::TSAnyKeyword
            | TypeScriptNode::TSArrayType
            | TypeScriptNode::TSAsExpression
            | TypeScriptNode::TSAsyncKeyword
            | TypeScriptNode::TSBigIntKeyword
            | TypeScriptNode::TSBooleanKeyword
            | TypeScriptNode::TSCallSignatureDeclaration
            | TypeScriptNode::TSClassImplements
            | TypeScriptNode::TSConditionalType
            | TypeScriptNode::TSConstructorType
            | TypeScriptNode::TSConstructSignatureDeclaration
            | TypeScriptNode::TSDeclareKeyword
            | TypeScriptNode::TSEnumBody
            | TypeScriptNode::TSExportAssignment
            | TypeScriptNode::TSExportKeyword
            | TypeScriptNode::TSExternalModuleReference
            | TypeScriptNode::TSFunctionType
            | TypeScriptNode::TSImportEqualsDeclaration
            | TypeScriptNode::TSImportType
            | TypeScriptNode::TSIndexedAccessType
            | TypeScriptNode::TSIndexSignature
            | TypeScriptNode::TSInferType
            | TypeScriptNode::TSInstantiationExpression
            | TypeScriptNode::TSInterfaceBody
            | TypeScriptNode::TSInterfaceHeritage
            | TypeScriptNode::TSIntersectionType
            | TypeScriptNode::TSIntrinsicKeyword
            | TypeScriptNode::TSLiteralType
            | TypeScriptNode::TSMappedType
            | TypeScriptNode::TSNamedTupleMember
            | TypeScriptNode::TSNamespaceExportDeclaration
            | TypeScriptNode::TSNeverKeyword
            | TypeScriptNode::TSNonNullExpression
            | TypeScriptNode::TSNullKeyword
            | TypeScriptNode::TSNumberKeyword
            | TypeScriptNode::TSObjectKeyword
            | TypeScriptNode::TSOptionalType
            | TypeScriptNode::TSParameterProperty
            | TypeScriptNode::TSPrivateKeyword
            | TypeScriptNode::TSProtectedKeyword
            | TypeScriptNode::TSPublicKeyword
            | TypeScriptNode::TSReadonlyKeyword
            | TypeScriptNode::TSRestType
            | TypeScriptNode::TSSatisfiesExpression
            | TypeScriptNode::TSStaticKeyword
            | TypeScriptNode::TSStringKeyword
            | TypeScriptNode::TSSymbolKeyword
            | TypeScriptNode::TSTemplateLiteralType
            | TypeScriptNode::TSThisType
            | TypeScriptNode::TSTupleType
            | TypeScriptNode::TSTypeAssertion
            | TypeScriptNode::TSTypeLiteral
            | TypeScriptNode::TSTypeOperator
            | TypeScriptNode::TSTypeParameterDeclaration
            | TypeScriptNode::TSTypeParameterInstantiation
            | TypeScriptNode::TSTypePredicate
            | TypeScriptNode::TSTypeQuery
            | TypeScriptNode::TSTypeReference
            | TypeScriptNode::TSUndefinedKeyword
            | TypeScriptNode::TSUnionType
            | TypeScriptNode::TSUnknownKeyword
            | TypeScriptNode::TSVoidKeyword,
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
        Kind::Node(TypeScriptNode::ArrowFunctionExpression) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSTypeParameter-arrow-const-modifier.ts"
        ))),
        Kind::Node(TypeScriptNode::BlockStatement | TypeScriptNode::FunctionDeclaration) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/scope/input.ts"),
        )),
        Kind::Node(TypeScriptNode::CallExpression | TypeScriptNode::MemberExpression) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/static_call/input.ts"),
        )),
        Kind::Node(TypeScriptNode::ClassDeclaration | TypeScriptNode::Decorator) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/decorates/input.ts"),
        )),
        Kind::Node(TypeScriptNode::ImportDeclaration | TypeScriptNode::ImportSpecifier) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/static_import/input.ts"),
        )),
        Kind::Node(TypeScriptNode::ImportDefaultSpecifier) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/ImportDeclaration-default-and-named-many.ts"
        ))),
        Kind::Node(TypeScriptNode::ImportNamespaceSpecifier) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/ImportDeclaration-assertion.ts"
        ))),
        Kind::Node(TypeScriptNode::Literal | TypeScriptNode::VariableDeclarator) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/relations/defines/input.ts"),
        )),
        Kind::Node(TypeScriptNode::TSDeclareFunction) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSDeclareFunction-async-ambient.ts"
        ))),
        Kind::Node(TypeScriptNode::TSEnumDeclaration) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSEnumDeclaration-const.ts"
        ))),
        Kind::Node(TypeScriptNode::TSInterfaceDeclaration) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/suite/valid/TSInterfaceDeclaration-empty.ts"
        ))),
        Kind::Node(TypeScriptNode::TSModuleBlock | TypeScriptNode::TSModuleDeclaration) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/suite/valid/TSModuleDeclaration-global.ts"),
        )),
        Kind::Node(TypeScriptNode::TSPropertySignature | TypeScriptNode::TSQualifiedName) => Sample::Node(SampleText(
            include_str!("../../tests/fixtures/corpus/TSTupleType.ts"),
        )),
        Kind::Node(TypeScriptNode::TSTypeAliasDeclaration | TypeScriptNode::TSTypeParameter) => {
            Sample::Node(SampleText(include_str!(
                "../../tests/fixtures/suite/valid/TSTypeAliasDeclaration-type-param-many.ts"
            )))
        }
        Kind::Node(TypeScriptNode::TSTypeAnnotation) => Sample::Node(SampleText(include_str!(
            "../../tests/fixtures/relations/annotates/input.ts"
        ))),
        Kind::Token(Token::Line) => Sample::Token(SampleText(include_str!(
            "../../tests/fixtures/relations/comment/input.ts"
        ))),
    }
}
