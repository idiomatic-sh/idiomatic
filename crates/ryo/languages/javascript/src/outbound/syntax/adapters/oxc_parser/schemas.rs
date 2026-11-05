//! Exhaustive typed Oxc AST mapping to `ESTree`. Parser wrappers are transparent; TypeScript kinds cannot occur in the selected JavaScript source modes; JSX kinds occur in `.js` and `.jsx`.
use crate::domain::nodes::types::Node;
use oxc_ast::{
    AstKind, AstType,
    ast::{ClassType, FunctionType},
};

/// The `ESTree` identity of a typed node, including declaration versus expression distinctions.
pub(super) fn node_kind(node: AstKind<'_>) -> Option<Node> {
    NodeSchema(node).into()
}
/// The parser's typed node at the conversion boundary.
pub(super) struct NodeSchema<'a>(pub AstKind<'a>);
impl From<NodeSchema<'_>> for Option<Node> {
    fn from(schema: NodeSchema<'_>) -> Self {
        let node = schema.0;
        if let AstKind::Function(function) = node {
            return match function.r#type {
                FunctionType::FunctionDeclaration => Some(Node::FunctionDeclaration),
                FunctionType::FunctionExpression => Some(Node::FunctionExpression),
                FunctionType::TSDeclareFunction | FunctionType::TSEmptyBodyFunctionExpression => {
                    None
                }
            };
        }
        if let AstKind::Class(class) = node {
            return Some(match class.r#type {
                ClassType::ClassDeclaration => Node::ClassDeclaration,
                ClassType::ClassExpression => Node::ClassExpression,
            });
        }
        if let AstKind::FormalParameter(parameter) = node
            && parameter.initializer.is_some()
        {
            return Some(Node::AssignmentPattern);
        }
        node_type(node.ty())
    }
}

/// Every upstream AST discriminant is explicitly mapped or identified as a transparent/non-JavaScript shape.
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive upstream vocabulary mapping keeps added variants a compile-time decision"
)]
pub(super) fn node_type(kind: AstType) -> Option<Node> {
    match kind {
        AstType::Program => Some(Node::Program),
        AstType::IdentifierName
        | AstType::IdentifierReference
        | AstType::BindingIdentifier
        | AstType::LabelIdentifier => Some(Node::Identifier),
        AstType::ThisExpression => Some(Node::ThisExpression),
        AstType::ArrayExpression => Some(Node::ArrayExpression),
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
        AstType::JSXElement => Some(Node::JSXElement),
        AstType::JSXOpeningElement => Some(Node::JSXOpeningElement),
        AstType::JSXClosingElement => Some(Node::JSXClosingElement),
        AstType::JSXFragment => Some(Node::JSXFragment),
        AstType::JSXOpeningFragment => Some(Node::JSXOpeningFragment),
        AstType::JSXClosingFragment => Some(Node::JSXClosingFragment),
        AstType::JSXNamespacedName => Some(Node::JSXNamespacedName),
        AstType::JSXMemberExpression => Some(Node::JSXMemberExpression),
        AstType::JSXExpressionContainer => Some(Node::JSXExpressionContainer),
        AstType::JSXEmptyExpression => Some(Node::JSXEmptyExpression),
        AstType::JSXAttribute => Some(Node::JSXAttribute),
        AstType::JSXSpreadAttribute => Some(Node::JSXSpreadAttribute),
        AstType::JSXIdentifier => Some(Node::JSXIdentifier),
        AstType::JSXSpreadChild => Some(Node::JSXSpreadChild),
        AstType::JSXText => Some(Node::JSXText),
        AstType::ObjectExpression => Some(Node::ObjectExpression),
        AstType::ObjectProperty
        | AstType::AssignmentTargetPropertyIdentifier
        | AstType::AssignmentTargetPropertyProperty
        | AstType::BindingProperty => Some(Node::Property),
        AstType::TemplateLiteral => Some(Node::TemplateLiteral),
        AstType::TaggedTemplateExpression => Some(Node::TaggedTemplateExpression),
        AstType::TemplateElement => Some(Node::TemplateElement),
        AstType::ComputedMemberExpression
        | AstType::StaticMemberExpression
        | AstType::PrivateFieldExpression => Some(Node::MemberExpression),
        AstType::CallExpression => Some(Node::CallExpression),
        AstType::NewExpression => Some(Node::NewExpression),
        AstType::ImportMeta | AstType::NewTarget => Some(Node::MetaProperty),
        AstType::SpreadElement => Some(Node::SpreadElement),
        AstType::UpdateExpression => Some(Node::UpdateExpression),
        AstType::UnaryExpression => Some(Node::UnaryExpression),
        AstType::BinaryExpression | AstType::PrivateInExpression => Some(Node::BinaryExpression),
        AstType::LogicalExpression => Some(Node::LogicalExpression),
        AstType::ConditionalExpression => Some(Node::ConditionalExpression),
        AstType::AssignmentExpression => Some(Node::AssignmentExpression),
        AstType::ArrayAssignmentTarget | AstType::ArrayPattern => Some(Node::ArrayPattern),
        AstType::ObjectAssignmentTarget | AstType::ObjectPattern => Some(Node::ObjectPattern),
        AstType::AssignmentTargetRest | AstType::BindingRestElement => Some(Node::RestElement),
        AstType::AssignmentTargetWithDefault | AstType::AssignmentPattern => {
            Some(Node::AssignmentPattern)
        }
        AstType::SequenceExpression => Some(Node::SequenceExpression),
        AstType::Super => Some(Node::Super),
        AstType::AwaitExpression => Some(Node::AwaitExpression),
        AstType::ChainExpression => Some(Node::ChainExpression),
        AstType::Directive | AstType::ExpressionStatement => Some(Node::ExpressionStatement),
        AstType::BlockStatement | AstType::FunctionBody => Some(Node::BlockStatement),
        AstType::VariableDeclaration => Some(Node::VariableDeclaration),
        AstType::VariableDeclarator => Some(Node::VariableDeclarator),
        AstType::EmptyStatement => Some(Node::EmptyStatement),
        AstType::IfStatement => Some(Node::IfStatement),
        AstType::DoWhileStatement => Some(Node::DoWhileStatement),
        AstType::WhileStatement => Some(Node::WhileStatement),
        AstType::ForStatement => Some(Node::ForStatement),
        AstType::ForInStatement => Some(Node::ForInStatement),
        AstType::ForOfStatement => Some(Node::ForOfStatement),
        AstType::ContinueStatement => Some(Node::ContinueStatement),
        AstType::BreakStatement => Some(Node::BreakStatement),
        AstType::ReturnStatement => Some(Node::ReturnStatement),
        AstType::WithStatement => Some(Node::WithStatement),
        AstType::SwitchStatement => Some(Node::SwitchStatement),
        AstType::SwitchCase => Some(Node::SwitchCase),
        AstType::LabeledStatement => Some(Node::LabeledStatement),
        AstType::ThrowStatement => Some(Node::ThrowStatement),
        AstType::TryStatement => Some(Node::TryStatement),
        AstType::CatchClause => Some(Node::CatchClause),
        AstType::DebuggerStatement => Some(Node::DebuggerStatement),
        AstType::ArrowFunctionExpression => Some(Node::ArrowFunctionExpression),
        AstType::YieldExpression => Some(Node::YieldExpression),
        AstType::ClassBody => Some(Node::ClassBody),
        AstType::MethodDefinition => Some(Node::MethodDefinition),
        AstType::PropertyDefinition => Some(Node::PropertyDefinition),
        AstType::PrivateIdentifier => Some(Node::PrivateIdentifier),
        AstType::StaticBlock => Some(Node::StaticBlock),
        AstType::ImportExpression => Some(Node::ImportExpression),
        AstType::ImportDeclaration => Some(Node::ImportDeclaration),
        AstType::ImportSpecifier => Some(Node::ImportSpecifier),
        AstType::ImportDefaultSpecifier => Some(Node::ImportDefaultSpecifier),
        AstType::ImportNamespaceSpecifier => Some(Node::ImportNamespaceSpecifier),
        AstType::ImportAttribute => Some(Node::ImportAttribute),
        AstType::ExportNamedDeclaration => Some(Node::ExportNamedDeclaration),
        AstType::ExportDefaultDeclaration => Some(Node::ExportDefaultDeclaration),
        AstType::ExportAllDeclaration => Some(Node::ExportAllDeclaration),
        AstType::ExportSpecifier => Some(Node::ExportSpecifier),
        AstType::BooleanLiteral
        | AstType::NullLiteral
        | AstType::NumericLiteral
        | AstType::StringLiteral
        | AstType::BigIntLiteral
        | AstType::RegExpLiteral => Some(Node::Literal),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
