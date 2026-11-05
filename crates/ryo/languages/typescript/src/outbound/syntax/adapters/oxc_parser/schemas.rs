//! Oxc's typed AST mapped to the pinned TypeScript-`ESTree` vocabulary.
//! Parser-only wrappers are transparent; their children retain their standard parent.

use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::tokens::types::Token;
use oxc_ast::{
    AstKind,
    ast::{ClassType, FunctionType},
};

/// The standard kind represented by an Oxc node; wrapper nodes have no `ESTree` identity.
/// The parser's typed node at the conversion boundary.
struct NodeSchema<'a>(AstKind<'a>);
impl From<NodeSchema<'_>> for Option<TypeScriptNode> {
    #[expect(
        clippy::too_many_lines,
        reason = "One exhaustive conversion table holds the pinned parser vocabulary and its transparent wrappers together"
    )]
    fn from(schema: NodeSchema<'_>) -> Self {
        match schema.0 {
            AstKind::Program(_) => Some(TypeScriptNode::Program),
            AstKind::IdentifierName(_)
            | AstKind::IdentifierReference(_)
            | AstKind::BindingIdentifier(_)
            | AstKind::LabelIdentifier(_)
            | AstKind::TSThisParameter(_)
            | AstKind::TSIndexSignatureName(_) => Some(TypeScriptNode::Identifier),
            AstKind::ThisExpression(_) => Some(TypeScriptNode::ThisExpression),
            AstKind::ArrayExpression(_) => Some(TypeScriptNode::ArrayExpression),
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
            AstKind::ObjectExpression(_) => Some(TypeScriptNode::ObjectExpression),
            AstKind::ObjectProperty(_)
            | AstKind::AssignmentTargetPropertyIdentifier(_)
            | AstKind::AssignmentTargetPropertyProperty(_)
            | AstKind::BindingProperty(_) => Some(TypeScriptNode::Property),
            AstKind::TemplateLiteral(_) => Some(TypeScriptNode::TemplateLiteral),
            AstKind::TaggedTemplateExpression(_) => Some(TypeScriptNode::TaggedTemplateExpression),
            AstKind::TemplateElement(_) => Some(TypeScriptNode::TemplateElement),
            AstKind::ComputedMemberExpression(_)
            | AstKind::StaticMemberExpression(_)
            | AstKind::PrivateFieldExpression(_) => Some(TypeScriptNode::MemberExpression),
            AstKind::CallExpression(_) => Some(TypeScriptNode::CallExpression),
            AstKind::NewExpression(_) => Some(TypeScriptNode::NewExpression),
            AstKind::ImportMeta(_) | AstKind::NewTarget(_) => Some(TypeScriptNode::MetaProperty),
            AstKind::SpreadElement(_) => Some(TypeScriptNode::SpreadElement),
            AstKind::UpdateExpression(_) => Some(TypeScriptNode::UpdateExpression),
            AstKind::UnaryExpression(_) => Some(TypeScriptNode::UnaryExpression),
            AstKind::BinaryExpression(_) | AstKind::PrivateInExpression(_) => {
                Some(TypeScriptNode::BinaryExpression)
            }
            AstKind::LogicalExpression(_) => Some(TypeScriptNode::LogicalExpression),
            AstKind::ConditionalExpression(_) => Some(TypeScriptNode::ConditionalExpression),
            AstKind::AssignmentExpression(_) => Some(TypeScriptNode::AssignmentExpression),
            AstKind::ArrayAssignmentTarget(_) | AstKind::ArrayPattern(_) => {
                Some(TypeScriptNode::ArrayPattern)
            }
            AstKind::ObjectAssignmentTarget(_) | AstKind::ObjectPattern(_) => {
                Some(TypeScriptNode::ObjectPattern)
            }
            AstKind::AssignmentTargetRest(_)
            | AstKind::BindingRestElement(_)
            | AstKind::FormalParameterRest(_) => Some(TypeScriptNode::RestElement),
            AstKind::AssignmentTargetWithDefault(_) | AstKind::AssignmentPattern(_) => {
                Some(TypeScriptNode::AssignmentPattern)
            }
            AstKind::SequenceExpression(_) => Some(TypeScriptNode::SequenceExpression),
            AstKind::Super(_) => Some(TypeScriptNode::Super),
            AstKind::AwaitExpression(_) => Some(TypeScriptNode::AwaitExpression),
            AstKind::ChainExpression(_) => Some(TypeScriptNode::ChainExpression),
            AstKind::Directive(_) | AstKind::ExpressionStatement(_) => {
                Some(TypeScriptNode::ExpressionStatement)
            }
            AstKind::BlockStatement(_) | AstKind::FunctionBody(_) => Some(TypeScriptNode::BlockStatement),
            AstKind::VariableDeclaration(_) => Some(TypeScriptNode::VariableDeclaration),
            AstKind::VariableDeclarator(_) => Some(TypeScriptNode::VariableDeclarator),
            AstKind::EmptyStatement(_) => Some(TypeScriptNode::EmptyStatement),
            AstKind::IfStatement(_) => Some(TypeScriptNode::IfStatement),
            AstKind::DoWhileStatement(_) => Some(TypeScriptNode::DoWhileStatement),
            AstKind::WhileStatement(_) => Some(TypeScriptNode::WhileStatement),
            AstKind::ForStatement(_) => Some(TypeScriptNode::ForStatement),
            AstKind::ForInStatement(_) => Some(TypeScriptNode::ForInStatement),
            AstKind::ForOfStatement(_) => Some(TypeScriptNode::ForOfStatement),
            AstKind::ContinueStatement(_) => Some(TypeScriptNode::ContinueStatement),
            AstKind::BreakStatement(_) => Some(TypeScriptNode::BreakStatement),
            AstKind::ReturnStatement(_) => Some(TypeScriptNode::ReturnStatement),
            AstKind::WithStatement(_) => Some(TypeScriptNode::WithStatement),
            AstKind::SwitchStatement(_) => Some(TypeScriptNode::SwitchStatement),
            AstKind::SwitchCase(_) => Some(TypeScriptNode::SwitchCase),
            AstKind::LabeledStatement(_) => Some(TypeScriptNode::LabeledStatement),
            AstKind::ThrowStatement(_) => Some(TypeScriptNode::ThrowStatement),
            AstKind::TryStatement(_) => Some(TypeScriptNode::TryStatement),
            AstKind::CatchClause(_) => Some(TypeScriptNode::CatchClause),
            AstKind::DebuggerStatement(_) => Some(TypeScriptNode::DebuggerStatement),
            AstKind::Function(function) => Some(match function.r#type {
                FunctionType::FunctionDeclaration => TypeScriptNode::FunctionDeclaration,
                FunctionType::FunctionExpression => TypeScriptNode::FunctionExpression,
                FunctionType::TSDeclareFunction => TypeScriptNode::TSDeclareFunction,
                FunctionType::TSEmptyBodyFunctionExpression => TypeScriptNode::TSEmptyBodyFunctionExpression,
            }),
            AstKind::ArrowFunctionExpression(_) => Some(TypeScriptNode::ArrowFunctionExpression),
            AstKind::YieldExpression(_) => Some(TypeScriptNode::YieldExpression),
            AstKind::Class(class) => Some(match class.r#type {
                ClassType::ClassDeclaration => TypeScriptNode::ClassDeclaration,
                ClassType::ClassExpression => TypeScriptNode::ClassExpression,
            }),
            AstKind::ClassBody(_) => Some(TypeScriptNode::ClassBody),
            AstKind::MethodDefinition(_) => Some(TypeScriptNode::MethodDefinition),
            AstKind::PropertyDefinition(_) => Some(TypeScriptNode::PropertyDefinition),
            AstKind::PrivateIdentifier(_) => Some(TypeScriptNode::PrivateIdentifier),
            AstKind::StaticBlock(_) => Some(TypeScriptNode::StaticBlock),
            AstKind::AccessorProperty(_) => Some(TypeScriptNode::AccessorProperty),
            AstKind::ImportExpression(_) => Some(TypeScriptNode::ImportExpression),
            AstKind::ImportDeclaration(_) => Some(TypeScriptNode::ImportDeclaration),
            AstKind::ImportSpecifier(_) => Some(TypeScriptNode::ImportSpecifier),
            AstKind::ImportDefaultSpecifier(_) => Some(TypeScriptNode::ImportDefaultSpecifier),
            AstKind::ImportNamespaceSpecifier(_) => Some(TypeScriptNode::ImportNamespaceSpecifier),
            AstKind::ImportAttribute(_) => Some(TypeScriptNode::ImportAttribute),
            AstKind::ExportNamedDeclaration(_) => Some(TypeScriptNode::ExportNamedDeclaration),
            AstKind::ExportDefaultDeclaration(_) => Some(TypeScriptNode::ExportDefaultDeclaration),
            AstKind::ExportAllDeclaration(_) => Some(TypeScriptNode::ExportAllDeclaration),
            AstKind::ExportSpecifier(_) => Some(TypeScriptNode::ExportSpecifier),
            AstKind::BooleanLiteral(_)
            | AstKind::NullLiteral(_)
            | AstKind::NumericLiteral(_)
            | AstKind::StringLiteral(_)
            | AstKind::BigIntLiteral(_)
            | AstKind::RegExpLiteral(_) => Some(TypeScriptNode::Literal),
            AstKind::JSXElement(_) => Some(TypeScriptNode::JSXElement),
            AstKind::JSXOpeningElement(_) => Some(TypeScriptNode::JSXOpeningElement),
            AstKind::JSXClosingElement(_) => Some(TypeScriptNode::JSXClosingElement),
            AstKind::JSXFragment(_) => Some(TypeScriptNode::JSXFragment),
            AstKind::JSXOpeningFragment(_) => Some(TypeScriptNode::JSXOpeningFragment),
            AstKind::JSXClosingFragment(_) => Some(TypeScriptNode::JSXClosingFragment),
            AstKind::JSXNamespacedName(_) => Some(TypeScriptNode::JSXNamespacedName),
            AstKind::JSXMemberExpression(_) => Some(TypeScriptNode::JSXMemberExpression),
            AstKind::JSXExpressionContainer(_) => Some(TypeScriptNode::JSXExpressionContainer),
            AstKind::JSXEmptyExpression(_) => Some(TypeScriptNode::JSXEmptyExpression),
            AstKind::JSXAttribute(_) => Some(TypeScriptNode::JSXAttribute),
            AstKind::JSXSpreadAttribute(_) => Some(TypeScriptNode::JSXSpreadAttribute),
            AstKind::JSXIdentifier(_) => Some(TypeScriptNode::JSXIdentifier),
            AstKind::JSXSpreadChild(_) => Some(TypeScriptNode::JSXSpreadChild),
            AstKind::JSXText(_) => Some(TypeScriptNode::JSXText),
            AstKind::TSEnumDeclaration(_) => Some(TypeScriptNode::TSEnumDeclaration),
            AstKind::TSEnumBody(_) => Some(TypeScriptNode::TSEnumBody),
            AstKind::TSEnumMember(_) => Some(TypeScriptNode::TSEnumMember),
            AstKind::TSTypeAnnotation(_) => Some(TypeScriptNode::TSTypeAnnotation),
            AstKind::TSLiteralType(_) => Some(TypeScriptNode::TSLiteralType),
            AstKind::TSConditionalType(_) => Some(TypeScriptNode::TSConditionalType),
            AstKind::TSUnionType(_) => Some(TypeScriptNode::TSUnionType),
            AstKind::TSIntersectionType(_) => Some(TypeScriptNode::TSIntersectionType),
            AstKind::TSTypeOperator(_) => Some(TypeScriptNode::TSTypeOperator),
            AstKind::TSArrayType(_) => Some(TypeScriptNode::TSArrayType),
            AstKind::TSIndexedAccessType(_) => Some(TypeScriptNode::TSIndexedAccessType),
            AstKind::TSTupleType(_) => Some(TypeScriptNode::TSTupleType),
            AstKind::TSNamedTupleMember(_) => Some(TypeScriptNode::TSNamedTupleMember),
            AstKind::TSOptionalType(_) => Some(TypeScriptNode::TSOptionalType),
            AstKind::TSRestType(_) => Some(TypeScriptNode::TSRestType),
            AstKind::TSAnyKeyword(_) => Some(TypeScriptNode::TSAnyKeyword),
            AstKind::TSStringKeyword(_) => Some(TypeScriptNode::TSStringKeyword),
            AstKind::TSBooleanKeyword(_) => Some(TypeScriptNode::TSBooleanKeyword),
            AstKind::TSNumberKeyword(_) => Some(TypeScriptNode::TSNumberKeyword),
            AstKind::TSNeverKeyword(_) => Some(TypeScriptNode::TSNeverKeyword),
            AstKind::TSIntrinsicKeyword(_) => Some(TypeScriptNode::TSIntrinsicKeyword),
            AstKind::TSUnknownKeyword(_) => Some(TypeScriptNode::TSUnknownKeyword),
            AstKind::TSNullKeyword(_) => Some(TypeScriptNode::TSNullKeyword),
            AstKind::TSUndefinedKeyword(_) => Some(TypeScriptNode::TSUndefinedKeyword),
            AstKind::TSVoidKeyword(_) => Some(TypeScriptNode::TSVoidKeyword),
            AstKind::TSSymbolKeyword(_) => Some(TypeScriptNode::TSSymbolKeyword),
            AstKind::TSThisType(_) => Some(TypeScriptNode::TSThisType),
            AstKind::TSObjectKeyword(_) => Some(TypeScriptNode::TSObjectKeyword),
            AstKind::TSBigIntKeyword(_) => Some(TypeScriptNode::TSBigIntKeyword),
            AstKind::TSTypeReference(_) => Some(TypeScriptNode::TSTypeReference),
            AstKind::TSQualifiedName(_) | AstKind::TSImportTypeQualifiedName(_) => {
                Some(TypeScriptNode::TSQualifiedName)
            }
            AstKind::TSTypeParameterInstantiation(_) => Some(TypeScriptNode::TSTypeParameterInstantiation),
            AstKind::TSTypeParameter(_) => Some(TypeScriptNode::TSTypeParameter),
            AstKind::TSTypeParameterDeclaration(_) => Some(TypeScriptNode::TSTypeParameterDeclaration),
            AstKind::TSTypeAliasDeclaration(_) => Some(TypeScriptNode::TSTypeAliasDeclaration),
            AstKind::TSClassImplements(_) => Some(TypeScriptNode::TSClassImplements),
            AstKind::TSInterfaceDeclaration(_) => Some(TypeScriptNode::TSInterfaceDeclaration),
            AstKind::TSInterfaceBody(_) => Some(TypeScriptNode::TSInterfaceBody),
            AstKind::TSPropertySignature(_) => Some(TypeScriptNode::TSPropertySignature),
            AstKind::TSIndexSignature(_) => Some(TypeScriptNode::TSIndexSignature),
            AstKind::TSCallSignatureDeclaration(_) => Some(TypeScriptNode::TSCallSignatureDeclaration),
            AstKind::TSMethodSignature(_) => Some(TypeScriptNode::TSMethodSignature),
            AstKind::TSConstructSignatureDeclaration(_) => {
                Some(TypeScriptNode::TSConstructSignatureDeclaration)
            }
            AstKind::TSInterfaceHeritage(_) => Some(TypeScriptNode::TSInterfaceHeritage),
            AstKind::TSTypePredicate(_) => Some(TypeScriptNode::TSTypePredicate),
            AstKind::TSModuleDeclaration(_) | AstKind::TSGlobalDeclaration(_) => {
                Some(TypeScriptNode::TSModuleDeclaration)
            }
            AstKind::TSModuleBlock(_) => Some(TypeScriptNode::TSModuleBlock),
            AstKind::TSTypeLiteral(_) => Some(TypeScriptNode::TSTypeLiteral),
            AstKind::TSInferType(_) => Some(TypeScriptNode::TSInferType),
            AstKind::TSTypeQuery(_) => Some(TypeScriptNode::TSTypeQuery),
            AstKind::TSImportType(_) => Some(TypeScriptNode::TSImportType),
            AstKind::TSFunctionType(_) => Some(TypeScriptNode::TSFunctionType),
            AstKind::TSConstructorType(_) => Some(TypeScriptNode::TSConstructorType),
            AstKind::TSMappedType(_) => Some(TypeScriptNode::TSMappedType),
            AstKind::TSTemplateLiteralType(_) => Some(TypeScriptNode::TSTemplateLiteralType),
            AstKind::TSAsExpression(_) => Some(TypeScriptNode::TSAsExpression),
            AstKind::TSSatisfiesExpression(_) => Some(TypeScriptNode::TSSatisfiesExpression),
            AstKind::TSTypeAssertion(_) => Some(TypeScriptNode::TSTypeAssertion),
            AstKind::TSImportEqualsDeclaration(_) => Some(TypeScriptNode::TSImportEqualsDeclaration),
            AstKind::TSExternalModuleReference(_) => Some(TypeScriptNode::TSExternalModuleReference),
            AstKind::TSNonNullExpression(_) => Some(TypeScriptNode::TSNonNullExpression),
            AstKind::Decorator(_) => Some(TypeScriptNode::Decorator),
            AstKind::TSExportAssignment(_) => Some(TypeScriptNode::TSExportAssignment),
            AstKind::TSNamespaceExportDeclaration(_) => Some(TypeScriptNode::TSNamespaceExportDeclaration),
            AstKind::TSInstantiationExpression(_) => Some(TypeScriptNode::TSInstantiationExpression),
        }
    }
}

/// The standard node for a parser node, or a transparent parser wrapper.
pub(super) fn node_kind(kind: AstKind<'_>) -> Option<TypeScriptNode> {
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
