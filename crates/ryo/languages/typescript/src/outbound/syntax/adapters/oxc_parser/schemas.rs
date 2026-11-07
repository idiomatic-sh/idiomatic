//! Oxc's typed AST mapped to the pinned TypeScript-`ESTree` vocabulary.
//! Parser-only wrappers are transparent; their children retain their standard parent.

use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use oxc_ast::{
    AstKind,
    ast::{ClassType, FunctionType},
};

/// The standard kind represented by an Oxc node; wrapper nodes have no `ESTree` identity.
/// The parser's typed node at the conversion boundary.
struct NodeSchema<'a>(AstKind<'a>);
impl From<NodeSchema<'_>> for Option<Node> {
    #[expect(
        clippy::too_many_lines,
        reason = "One exhaustive conversion table holds the pinned parser vocabulary and its transparent wrappers together"
    )]
    fn from(schema: NodeSchema<'_>) -> Self {
        match schema.0 {
            AstKind::Program(_) => Some(Node::Program),
            AstKind::IdentifierName(_)
            | AstKind::IdentifierReference(_)
            | AstKind::BindingIdentifier(_)
            | AstKind::LabelIdentifier(_)
            | AstKind::TSThisParameter(_)
            | AstKind::TSIndexSignatureName(_) => Some(Node::Identifier),
            AstKind::ThisExpression(_) => Some(Node::ThisExpression),
            AstKind::ArrayExpression(_) => Some(Node::ArrayExpression),
            AstKind::Elision(_)
            | AstKind::ParenthesizedExpression(_)
            | AstKind::Hashbang(_)
            | AstKind::CatchParameter(_)
            | AstKind::FormalParameters(_)
            | AstKind::FormalParameter(_)
            | AstKind::WithClause(_)
            | AstKind::V8IntrinsicExpression(_)
            | AstKind::TSParenthesizedType(_)
            | AstKind::JSDocNullableType(_)
            | AstKind::JSDocNonNullableType(_)
            | AstKind::JSDocUnknownType(_) => None,
            AstKind::ObjectExpression(_) => Some(Node::ObjectExpression),
            AstKind::ObjectProperty(_)
            | AstKind::AssignmentTargetPropertyIdentifier(_)
            | AstKind::AssignmentTargetPropertyProperty(_)
            | AstKind::BindingProperty(_) => Some(Node::Property),
            AstKind::TemplateLiteral(_) => Some(Node::TemplateLiteral),
            AstKind::TaggedTemplateExpression(_) => Some(Node::TaggedTemplateExpression),
            AstKind::TemplateElement(_) => Some(Node::TemplateElement),
            AstKind::ComputedMemberExpression(_)
            | AstKind::StaticMemberExpression(_)
            | AstKind::PrivateFieldExpression(_) => Some(Node::MemberExpression),
            AstKind::CallExpression(_) => Some(Node::CallExpression),
            AstKind::NewExpression(_) => Some(Node::NewExpression),
            AstKind::ImportMeta(_) | AstKind::NewTarget(_) => Some(Node::MetaProperty),
            AstKind::SpreadElement(_) => Some(Node::SpreadElement),
            AstKind::UpdateExpression(_) => Some(Node::UpdateExpression),
            AstKind::UnaryExpression(_) => Some(Node::UnaryExpression),
            AstKind::BinaryExpression(_) | AstKind::PrivateInExpression(_) => {
                Some(Node::BinaryExpression)
            }
            AstKind::LogicalExpression(_) => Some(Node::LogicalExpression),
            AstKind::ConditionalExpression(_) => Some(Node::ConditionalExpression),
            AstKind::AssignmentExpression(_) => Some(Node::AssignmentExpression),
            AstKind::ArrayAssignmentTarget(_) | AstKind::ArrayPattern(_) => {
                Some(Node::ArrayPattern)
            }
            AstKind::ObjectAssignmentTarget(_) | AstKind::ObjectPattern(_) => {
                Some(Node::ObjectPattern)
            }
            AstKind::AssignmentTargetRest(_)
            | AstKind::BindingRestElement(_)
            | AstKind::FormalParameterRest(_) => Some(Node::RestElement),
            AstKind::AssignmentTargetWithDefault(_) | AstKind::AssignmentPattern(_) => {
                Some(Node::AssignmentPattern)
            }
            AstKind::SequenceExpression(_) => Some(Node::SequenceExpression),
            AstKind::Super(_) => Some(Node::Super),
            AstKind::AwaitExpression(_) => Some(Node::AwaitExpression),
            AstKind::ChainExpression(_) => Some(Node::ChainExpression),
            AstKind::Directive(_) | AstKind::ExpressionStatement(_) => {
                Some(Node::ExpressionStatement)
            }
            AstKind::BlockStatement(_) | AstKind::FunctionBody(_) => Some(Node::BlockStatement),
            AstKind::VariableDeclaration(_) => Some(Node::VariableDeclaration),
            AstKind::VariableDeclarator(_) => Some(Node::VariableDeclarator),
            AstKind::EmptyStatement(_) => Some(Node::EmptyStatement),
            AstKind::IfStatement(_) => Some(Node::IfStatement),
            AstKind::DoWhileStatement(_) => Some(Node::DoWhileStatement),
            AstKind::WhileStatement(_) => Some(Node::WhileStatement),
            AstKind::ForStatement(_) => Some(Node::ForStatement),
            AstKind::ForInStatement(_) => Some(Node::ForInStatement),
            AstKind::ForOfStatement(_) => Some(Node::ForOfStatement),
            AstKind::ContinueStatement(_) => Some(Node::ContinueStatement),
            AstKind::BreakStatement(_) => Some(Node::BreakStatement),
            AstKind::ReturnStatement(_) => Some(Node::ReturnStatement),
            AstKind::WithStatement(_) => Some(Node::WithStatement),
            AstKind::SwitchStatement(_) => Some(Node::SwitchStatement),
            AstKind::SwitchCase(_) => Some(Node::SwitchCase),
            AstKind::LabeledStatement(_) => Some(Node::LabeledStatement),
            AstKind::ThrowStatement(_) => Some(Node::ThrowStatement),
            AstKind::TryStatement(_) => Some(Node::TryStatement),
            AstKind::CatchClause(_) => Some(Node::CatchClause),
            AstKind::DebuggerStatement(_) => Some(Node::DebuggerStatement),
            AstKind::Function(function) => Some(match function.r#type {
                FunctionType::FunctionDeclaration => Node::FunctionDeclaration,
                FunctionType::FunctionExpression => Node::FunctionExpression,
                FunctionType::TSDeclareFunction => Node::TSDeclareFunction,
                FunctionType::TSEmptyBodyFunctionExpression => Node::TSEmptyBodyFunctionExpression,
            }),
            AstKind::ArrowFunctionExpression(_) => Some(Node::ArrowFunctionExpression),
            AstKind::YieldExpression(_) => Some(Node::YieldExpression),
            AstKind::Class(class) => Some(match class.r#type {
                ClassType::ClassDeclaration => Node::ClassDeclaration,
                ClassType::ClassExpression => Node::ClassExpression,
            }),
            AstKind::ClassBody(_) => Some(Node::ClassBody),
            AstKind::MethodDefinition(_) => Some(Node::MethodDefinition),
            AstKind::PropertyDefinition(_) => Some(Node::PropertyDefinition),
            AstKind::PrivateIdentifier(_) => Some(Node::PrivateIdentifier),
            AstKind::StaticBlock(_) => Some(Node::StaticBlock),
            AstKind::AccessorProperty(_) => Some(Node::AccessorProperty),
            AstKind::ImportExpression(_) => Some(Node::ImportExpression),
            AstKind::ImportDeclaration(_) => Some(Node::ImportDeclaration),
            AstKind::ImportSpecifier(_) => Some(Node::ImportSpecifier),
            AstKind::ImportDefaultSpecifier(_) => Some(Node::ImportDefaultSpecifier),
            AstKind::ImportNamespaceSpecifier(_) => Some(Node::ImportNamespaceSpecifier),
            AstKind::ImportAttribute(_) => Some(Node::ImportAttribute),
            AstKind::ExportNamedDeclaration(_) => Some(Node::ExportNamedDeclaration),
            AstKind::ExportDefaultDeclaration(_) => Some(Node::ExportDefaultDeclaration),
            AstKind::ExportAllDeclaration(_) => Some(Node::ExportAllDeclaration),
            AstKind::ExportSpecifier(_) => Some(Node::ExportSpecifier),
            AstKind::BooleanLiteral(_)
            | AstKind::NullLiteral(_)
            | AstKind::NumericLiteral(_)
            | AstKind::StringLiteral(_)
            | AstKind::BigIntLiteral(_)
            | AstKind::RegExpLiteral(_) => Some(Node::Literal),
            AstKind::JSXElement(_) => Some(Node::JSXElement),
            AstKind::JSXOpeningElement(_) => Some(Node::JSXOpeningElement),
            AstKind::JSXClosingElement(_) => Some(Node::JSXClosingElement),
            AstKind::JSXFragment(_) => Some(Node::JSXFragment),
            AstKind::JSXOpeningFragment(_) => Some(Node::JSXOpeningFragment),
            AstKind::JSXClosingFragment(_) => Some(Node::JSXClosingFragment),
            AstKind::JSXNamespacedName(_) => Some(Node::JSXNamespacedName),
            AstKind::JSXMemberExpression(_) => Some(Node::JSXMemberExpression),
            AstKind::JSXExpressionContainer(_) => Some(Node::JSXExpressionContainer),
            AstKind::JSXEmptyExpression(_) => Some(Node::JSXEmptyExpression),
            AstKind::JSXAttribute(_) => Some(Node::JSXAttribute),
            AstKind::JSXSpreadAttribute(_) => Some(Node::JSXSpreadAttribute),
            AstKind::JSXIdentifier(_) => Some(Node::JSXIdentifier),
            AstKind::JSXSpreadChild(_) => Some(Node::JSXSpreadChild),
            AstKind::JSXText(_) => Some(Node::JSXText),
            AstKind::TSEnumDeclaration(_) => Some(Node::TSEnumDeclaration),
            AstKind::TSEnumBody(_) => Some(Node::TSEnumBody),
            AstKind::TSEnumMember(_) => Some(Node::TSEnumMember),
            AstKind::TSTypeAnnotation(_) => Some(Node::TSTypeAnnotation),
            AstKind::TSLiteralType(_) => Some(Node::TSLiteralType),
            AstKind::TSConditionalType(_) => Some(Node::TSConditionalType),
            AstKind::TSUnionType(_) => Some(Node::TSUnionType),
            AstKind::TSIntersectionType(_) => Some(Node::TSIntersectionType),
            AstKind::TSTypeOperator(_) => Some(Node::TSTypeOperator),
            AstKind::TSArrayType(_) => Some(Node::TSArrayType),
            AstKind::TSIndexedAccessType(_) => Some(Node::TSIndexedAccessType),
            AstKind::TSTupleType(_) => Some(Node::TSTupleType),
            AstKind::TSNamedTupleMember(_) => Some(Node::TSNamedTupleMember),
            AstKind::TSOptionalType(_) => Some(Node::TSOptionalType),
            AstKind::TSRestType(_) => Some(Node::TSRestType),
            AstKind::TSAnyKeyword(_) => Some(Node::TSAnyKeyword),
            AstKind::TSStringKeyword(_) => Some(Node::TSStringKeyword),
            AstKind::TSBooleanKeyword(_) => Some(Node::TSBooleanKeyword),
            AstKind::TSNumberKeyword(_) => Some(Node::TSNumberKeyword),
            AstKind::TSNeverKeyword(_) => Some(Node::TSNeverKeyword),
            AstKind::TSIntrinsicKeyword(_) => Some(Node::TSIntrinsicKeyword),
            AstKind::TSUnknownKeyword(_) => Some(Node::TSUnknownKeyword),
            AstKind::TSNullKeyword(_) => Some(Node::TSNullKeyword),
            AstKind::TSUndefinedKeyword(_) => Some(Node::TSUndefinedKeyword),
            AstKind::TSVoidKeyword(_) => Some(Node::TSVoidKeyword),
            AstKind::TSSymbolKeyword(_) => Some(Node::TSSymbolKeyword),
            AstKind::TSThisType(_) => Some(Node::TSThisType),
            AstKind::TSObjectKeyword(_) => Some(Node::TSObjectKeyword),
            AstKind::TSBigIntKeyword(_) => Some(Node::TSBigIntKeyword),
            AstKind::TSTypeReference(_) => Some(Node::TSTypeReference),
            AstKind::TSQualifiedName(_) | AstKind::TSImportTypeQualifiedName(_) => {
                Some(Node::TSQualifiedName)
            }
            AstKind::TSTypeParameterInstantiation(_) => Some(Node::TSTypeParameterInstantiation),
            AstKind::TSTypeParameter(_) => Some(Node::TSTypeParameter),
            AstKind::TSTypeParameterDeclaration(_) => Some(Node::TSTypeParameterDeclaration),
            AstKind::TSTypeAliasDeclaration(_) => Some(Node::TSTypeAliasDeclaration),
            AstKind::TSClassImplements(_) => Some(Node::TSClassImplements),
            AstKind::TSInterfaceDeclaration(_) => Some(Node::TSInterfaceDeclaration),
            AstKind::TSInterfaceBody(_) => Some(Node::TSInterfaceBody),
            AstKind::TSPropertySignature(_) => Some(Node::TSPropertySignature),
            AstKind::TSIndexSignature(_) => Some(Node::TSIndexSignature),
            AstKind::TSCallSignatureDeclaration(_) => Some(Node::TSCallSignatureDeclaration),
            AstKind::TSMethodSignature(_) => Some(Node::TSMethodSignature),
            AstKind::TSConstructSignatureDeclaration(_) => {
                Some(Node::TSConstructSignatureDeclaration)
            }
            AstKind::TSInterfaceHeritage(_) => Some(Node::TSInterfaceHeritage),
            AstKind::TSTypePredicate(_) => Some(Node::TSTypePredicate),
            AstKind::TSModuleDeclaration(_) | AstKind::TSGlobalDeclaration(_) => {
                Some(Node::TSModuleDeclaration)
            }
            AstKind::TSModuleBlock(_) => Some(Node::TSModuleBlock),
            AstKind::TSTypeLiteral(_) => Some(Node::TSTypeLiteral),
            AstKind::TSInferType(_) => Some(Node::TSInferType),
            AstKind::TSTypeQuery(_) => Some(Node::TSTypeQuery),
            AstKind::TSImportType(_) => Some(Node::TSImportType),
            AstKind::TSFunctionType(_) => Some(Node::TSFunctionType),
            AstKind::TSConstructorType(_) => Some(Node::TSConstructorType),
            AstKind::TSMappedType(_) => Some(Node::TSMappedType),
            AstKind::TSTemplateLiteralType(_) => Some(Node::TSTemplateLiteralType),
            AstKind::TSAsExpression(_) => Some(Node::TSAsExpression),
            AstKind::TSSatisfiesExpression(_) => Some(Node::TSSatisfiesExpression),
            AstKind::TSTypeAssertion(_) => Some(Node::TSTypeAssertion),
            AstKind::TSImportEqualsDeclaration(_) => Some(Node::TSImportEqualsDeclaration),
            AstKind::TSExternalModuleReference(_) => Some(Node::TSExternalModuleReference),
            AstKind::TSNonNullExpression(_) => Some(Node::TSNonNullExpression),
            AstKind::Decorator(_) => Some(Node::Decorator),
            AstKind::TSExportAssignment(_) => Some(Node::TSExportAssignment),
            AstKind::TSNamespaceExportDeclaration(_) => Some(Node::TSNamespaceExportDeclaration),
            AstKind::TSInstantiationExpression(_) => Some(Node::TSInstantiationExpression),
        }
    }
}

/// The standard node for a parser node, or a transparent parser wrapper.
pub(super) fn node_kind(kind: AstKind<'_>) -> Option<Node> {
    NodeSchema(kind).into()
}

/// Classify a lexeme by the standard token categories, refusing unknown future parser tokens.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "Oxc marks its token enum non_exhaustive; known categories are mapped and the remaining token is returned as a refusal"
)]
pub(super) fn token_kind(kind: oxc_parser::Kind) -> Result<Token, oxc_parser::Kind> {
    use oxc_parser::Kind as Lexeme;
    match kind {
        Lexeme::True | Lexeme::False => Ok(Token::Boolean),
        Lexeme::Null => Ok(Token::Null),
        Lexeme::Let => Ok(Token::Keyword),
        Lexeme::Await => Ok(Token::IdentifierToken),
        Lexeme::Str => Ok(Token::String),
        Lexeme::RegExp => Ok(Token::RegularExpression),
        Lexeme::NoSubstitutionTemplate
        | Lexeme::TemplateHead
        | Lexeme::TemplateMiddle
        | Lexeme::TemplateTail => Ok(Token::Template),
        Lexeme::PrivateIdentifier => Ok(Token::PrivateIdentifierToken),
        Lexeme::JSXText => Ok(Token::JSXTextToken),
        other if other.is_number() => Ok(Token::Numeric),
        other if other.is_reserved_keyword() || other.is_future_reserved_keyword() => {
            Ok(Token::Keyword)
        }
        other if other.is_identifier_or_keyword() => Ok(Token::IdentifierToken),
        Lexeme::Amp
        | Lexeme::Amp2
        | Lexeme::Amp2Eq
        | Lexeme::AmpEq
        | Lexeme::Bang
        | Lexeme::Caret
        | Lexeme::CaretEq
        | Lexeme::Colon
        | Lexeme::Comma
        | Lexeme::Dot
        | Lexeme::Dot3
        | Lexeme::Eq
        | Lexeme::Eq2
        | Lexeme::Eq3
        | Lexeme::GtEq
        | Lexeme::LAngle
        | Lexeme::LBrack
        | Lexeme::LCurly
        | Lexeme::LParen
        | Lexeme::LtEq
        | Lexeme::Minus
        | Lexeme::Minus2
        | Lexeme::MinusEq
        | Lexeme::Neq
        | Lexeme::Neq2
        | Lexeme::Percent
        | Lexeme::PercentEq
        | Lexeme::Pipe
        | Lexeme::Pipe2
        | Lexeme::Pipe2Eq
        | Lexeme::PipeEq
        | Lexeme::Plus
        | Lexeme::Plus2
        | Lexeme::PlusEq
        | Lexeme::Question
        | Lexeme::Question2
        | Lexeme::Question2Eq
        | Lexeme::QuestionDot
        | Lexeme::RAngle
        | Lexeme::RBrack
        | Lexeme::RCurly
        | Lexeme::RParen
        | Lexeme::Semicolon
        | Lexeme::ShiftLeft
        | Lexeme::ShiftLeftEq
        | Lexeme::ShiftRight
        | Lexeme::ShiftRight3
        | Lexeme::ShiftRight3Eq
        | Lexeme::ShiftRightEq
        | Lexeme::Slash
        | Lexeme::SlashEq
        | Lexeme::Star
        | Lexeme::Star2
        | Lexeme::Star2Eq
        | Lexeme::StarEq
        | Lexeme::Tilde
        | Lexeme::Arrow
        | Lexeme::At => Ok(Token::Punctuator),
        unrecognized => Err(unrecognized),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
