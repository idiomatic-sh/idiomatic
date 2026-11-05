//! Exhaustive typed Oxc AST mapping to `ESTree`. Parser wrappers are transparent; TypeScript kinds cannot occur in the selected JavaScript source modes; JSX kinds occur in `.js` and `.jsx`.
use crate::domain::nodes::types::JavaScriptNode;
use oxc_ast::{
    AstKind, AstType,
    ast::{ClassType, FunctionType},
};

/// The `ESTree` identity of a typed node, including declaration versus expression distinctions.
pub(super) fn node_kind(node: AstKind<'_>) -> Option<JavaScriptNode> {
    NodeSchema(node).into()
}
/// The parser's typed node at the conversion boundary.
pub(super) struct NodeSchema<'a>(pub AstKind<'a>);
impl From<NodeSchema<'_>> for Option<JavaScriptNode> {
    fn from(schema: NodeSchema<'_>) -> Self {
        let node = schema.0;
        if let AstKind::Function(function) = node {
            return match function.r#type {
                FunctionType::FunctionDeclaration => Some(JavaScriptNode::FunctionDeclaration),
                FunctionType::FunctionExpression => Some(JavaScriptNode::FunctionExpression),
                FunctionType::TSDeclareFunction | FunctionType::TSEmptyBodyFunctionExpression => {
                    None
                }
            };
        }
        if let AstKind::Class(class) = node {
            return Some(match class.r#type {
                ClassType::ClassDeclaration => JavaScriptNode::ClassDeclaration,
                ClassType::ClassExpression => JavaScriptNode::ClassExpression,
            });
        }
        if let AstKind::FormalParameter(parameter) = node
            && parameter.initializer.is_some()
        {
            return Some(JavaScriptNode::AssignmentPattern);
        }
        node_type(node.ty())
    }
}

/// Every upstream AST discriminant is explicitly mapped or identified as a transparent/non-JavaScript shape.
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive upstream vocabulary mapping keeps added variants a compile-time decision"
)]
pub(super) fn node_type(kind: AstType) -> Option<JavaScriptNode> {
    match kind {
        AstType::Program => Some(JavaScriptNode::Program),
        AstType::IdentifierName
        | AstType::IdentifierReference
        | AstType::BindingIdentifier
        | AstType::LabelIdentifier => Some(JavaScriptNode::Identifier),
        AstType::ThisExpression => Some(JavaScriptNode::ThisExpression),
        AstType::ArrayExpression => Some(JavaScriptNode::ArrayExpression),
        AstType::Elision
        | AstType::ParenthesizedExpression
        | AstType::Hashbang
        | AstType::CatchParameter
        | AstType::Function
        | AstType::FormalParameters
        | AstType::FormalParameter
        | AstType::FormalParameterRest
        | AstType::Class
        | AstType::AccessorProperty
        | AstType::WithClause
        | AstType::V8IntrinsicExpression
        | AstType::TSThisParameter
        | AstType::TSEnumDeclaration
        | AstType::TSEnumBody
        | AstType::TSEnumMember
        | AstType::TSTypeAnnotation
        | AstType::TSLiteralType
        | AstType::TSConditionalType
        | AstType::TSUnionType
        | AstType::TSIntersectionType
        | AstType::TSParenthesizedType
        | AstType::TSTypeOperator
        | AstType::TSArrayType
        | AstType::TSIndexedAccessType
        | AstType::TSTupleType
        | AstType::TSNamedTupleMember
        | AstType::TSOptionalType
        | AstType::TSRestType
        | AstType::TSAnyKeyword
        | AstType::TSStringKeyword
        | AstType::TSBooleanKeyword
        | AstType::TSNumberKeyword
        | AstType::TSNeverKeyword
        | AstType::TSIntrinsicKeyword
        | AstType::TSUnknownKeyword
        | AstType::TSNullKeyword
        | AstType::TSUndefinedKeyword
        | AstType::TSVoidKeyword
        | AstType::TSSymbolKeyword
        | AstType::TSThisType
        | AstType::TSObjectKeyword
        | AstType::TSBigIntKeyword
        | AstType::TSTypeReference
        | AstType::TSQualifiedName
        | AstType::TSTypeParameterInstantiation
        | AstType::TSTypeParameter
        | AstType::TSTypeParameterDeclaration
        | AstType::TSTypeAliasDeclaration
        | AstType::TSClassImplements
        | AstType::TSInterfaceDeclaration
        | AstType::TSInterfaceBody
        | AstType::TSPropertySignature
        | AstType::TSIndexSignature
        | AstType::TSCallSignatureDeclaration
        | AstType::TSMethodSignature
        | AstType::TSConstructSignatureDeclaration
        | AstType::TSIndexSignatureName
        | AstType::TSInterfaceHeritage
        | AstType::TSTypePredicate
        | AstType::TSModuleDeclaration
        | AstType::TSGlobalDeclaration
        | AstType::TSModuleBlock
        | AstType::TSTypeLiteral
        | AstType::TSInferType
        | AstType::TSTypeQuery
        | AstType::TSImportType
        | AstType::TSImportTypeQualifiedName
        | AstType::TSFunctionType
        | AstType::TSConstructorType
        | AstType::TSMappedType
        | AstType::TSTemplateLiteralType
        | AstType::TSAsExpression
        | AstType::TSSatisfiesExpression
        | AstType::TSTypeAssertion
        | AstType::TSImportEqualsDeclaration
        | AstType::TSExternalModuleReference
        | AstType::TSNonNullExpression
        | AstType::Decorator
        | AstType::TSExportAssignment
        | AstType::TSNamespaceExportDeclaration
        | AstType::TSInstantiationExpression
        | AstType::JSDocNullableType
        | AstType::JSDocNonNullableType
        | AstType::JSDocUnknownType => None,
        AstType::JSXElement => Some(JavaScriptNode::JSXElement),
        AstType::JSXOpeningElement => Some(JavaScriptNode::JSXOpeningElement),
        AstType::JSXClosingElement => Some(JavaScriptNode::JSXClosingElement),
        AstType::JSXFragment => Some(JavaScriptNode::JSXFragment),
        AstType::JSXOpeningFragment => Some(JavaScriptNode::JSXOpeningFragment),
        AstType::JSXClosingFragment => Some(JavaScriptNode::JSXClosingFragment),
        AstType::JSXNamespacedName => Some(JavaScriptNode::JSXNamespacedName),
        AstType::JSXMemberExpression => Some(JavaScriptNode::JSXMemberExpression),
        AstType::JSXExpressionContainer => Some(JavaScriptNode::JSXExpressionContainer),
        AstType::JSXEmptyExpression => Some(JavaScriptNode::JSXEmptyExpression),
        AstType::JSXAttribute => Some(JavaScriptNode::JSXAttribute),
        AstType::JSXSpreadAttribute => Some(JavaScriptNode::JSXSpreadAttribute),
        AstType::JSXIdentifier => Some(JavaScriptNode::JSXIdentifier),
        AstType::JSXSpreadChild => Some(JavaScriptNode::JSXSpreadChild),
        AstType::JSXText => Some(JavaScriptNode::JSXText),
        AstType::ObjectExpression => Some(JavaScriptNode::ObjectExpression),
        AstType::ObjectProperty
        | AstType::AssignmentTargetPropertyIdentifier
        | AstType::AssignmentTargetPropertyProperty
        | AstType::BindingProperty => Some(JavaScriptNode::Property),
        AstType::TemplateLiteral => Some(JavaScriptNode::TemplateLiteral),
        AstType::TaggedTemplateExpression => Some(JavaScriptNode::TaggedTemplateExpression),
        AstType::TemplateElement => Some(JavaScriptNode::TemplateElement),
        AstType::ComputedMemberExpression
        | AstType::StaticMemberExpression
        | AstType::PrivateFieldExpression => Some(JavaScriptNode::MemberExpression),
        AstType::CallExpression => Some(JavaScriptNode::CallExpression),
        AstType::NewExpression => Some(JavaScriptNode::NewExpression),
        AstType::ImportMeta | AstType::NewTarget => Some(JavaScriptNode::MetaProperty),
        AstType::SpreadElement => Some(JavaScriptNode::SpreadElement),
        AstType::UpdateExpression => Some(JavaScriptNode::UpdateExpression),
        AstType::UnaryExpression => Some(JavaScriptNode::UnaryExpression),
        AstType::BinaryExpression | AstType::PrivateInExpression => Some(JavaScriptNode::BinaryExpression),
        AstType::LogicalExpression => Some(JavaScriptNode::LogicalExpression),
        AstType::ConditionalExpression => Some(JavaScriptNode::ConditionalExpression),
        AstType::AssignmentExpression => Some(JavaScriptNode::AssignmentExpression),
        AstType::ArrayAssignmentTarget | AstType::ArrayPattern => Some(JavaScriptNode::ArrayPattern),
        AstType::ObjectAssignmentTarget | AstType::ObjectPattern => Some(JavaScriptNode::ObjectPattern),
        AstType::AssignmentTargetRest | AstType::BindingRestElement => Some(JavaScriptNode::RestElement),
        AstType::AssignmentTargetWithDefault | AstType::AssignmentPattern => {
            Some(JavaScriptNode::AssignmentPattern)
        }
        AstType::SequenceExpression => Some(JavaScriptNode::SequenceExpression),
        AstType::Super => Some(JavaScriptNode::Super),
        AstType::AwaitExpression => Some(JavaScriptNode::AwaitExpression),
        AstType::ChainExpression => Some(JavaScriptNode::ChainExpression),
        AstType::Directive | AstType::ExpressionStatement => Some(JavaScriptNode::ExpressionStatement),
        AstType::BlockStatement | AstType::FunctionBody => Some(JavaScriptNode::BlockStatement),
        AstType::VariableDeclaration => Some(JavaScriptNode::VariableDeclaration),
        AstType::VariableDeclarator => Some(JavaScriptNode::VariableDeclarator),
        AstType::EmptyStatement => Some(JavaScriptNode::EmptyStatement),
        AstType::IfStatement => Some(JavaScriptNode::IfStatement),
        AstType::DoWhileStatement => Some(JavaScriptNode::DoWhileStatement),
        AstType::WhileStatement => Some(JavaScriptNode::WhileStatement),
        AstType::ForStatement => Some(JavaScriptNode::ForStatement),
        AstType::ForInStatement => Some(JavaScriptNode::ForInStatement),
        AstType::ForOfStatement => Some(JavaScriptNode::ForOfStatement),
        AstType::ContinueStatement => Some(JavaScriptNode::ContinueStatement),
        AstType::BreakStatement => Some(JavaScriptNode::BreakStatement),
        AstType::ReturnStatement => Some(JavaScriptNode::ReturnStatement),
        AstType::WithStatement => Some(JavaScriptNode::WithStatement),
        AstType::SwitchStatement => Some(JavaScriptNode::SwitchStatement),
        AstType::SwitchCase => Some(JavaScriptNode::SwitchCase),
        AstType::LabeledStatement => Some(JavaScriptNode::LabeledStatement),
        AstType::ThrowStatement => Some(JavaScriptNode::ThrowStatement),
        AstType::TryStatement => Some(JavaScriptNode::TryStatement),
        AstType::CatchClause => Some(JavaScriptNode::CatchClause),
        AstType::DebuggerStatement => Some(JavaScriptNode::DebuggerStatement),
        AstType::ArrowFunctionExpression => Some(JavaScriptNode::ArrowFunctionExpression),
        AstType::YieldExpression => Some(JavaScriptNode::YieldExpression),
        AstType::ClassBody => Some(JavaScriptNode::ClassBody),
        AstType::MethodDefinition => Some(JavaScriptNode::MethodDefinition),
        AstType::PropertyDefinition => Some(JavaScriptNode::PropertyDefinition),
        AstType::PrivateIdentifier => Some(JavaScriptNode::PrivateIdentifier),
        AstType::StaticBlock => Some(JavaScriptNode::StaticBlock),
        AstType::ImportExpression => Some(JavaScriptNode::ImportExpression),
        AstType::ImportDeclaration => Some(JavaScriptNode::ImportDeclaration),
        AstType::ImportSpecifier => Some(JavaScriptNode::ImportSpecifier),
        AstType::ImportDefaultSpecifier => Some(JavaScriptNode::ImportDefaultSpecifier),
        AstType::ImportNamespaceSpecifier => Some(JavaScriptNode::ImportNamespaceSpecifier),
        AstType::ImportAttribute => Some(JavaScriptNode::ImportAttribute),
        AstType::ExportNamedDeclaration => Some(JavaScriptNode::ExportNamedDeclaration),
        AstType::ExportDefaultDeclaration => Some(JavaScriptNode::ExportDefaultDeclaration),
        AstType::ExportAllDeclaration => Some(JavaScriptNode::ExportAllDeclaration),
        AstType::ExportSpecifier => Some(JavaScriptNode::ExportSpecifier),
        AstType::BooleanLiteral
        | AstType::NullLiteral
        | AstType::NumericLiteral
        | AstType::StringLiteral
        | AstType::BigIntLiteral
        | AstType::RegExpLiteral => Some(JavaScriptNode::Literal),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
