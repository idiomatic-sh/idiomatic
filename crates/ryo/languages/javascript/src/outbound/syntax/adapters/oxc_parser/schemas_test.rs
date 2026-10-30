//! The complete upstream kind census and explicit representative `ESTree` conversions.
use super::node_type;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use oxc_ast::AstType;
#[derive(Clone, Copy, Debug)]
enum Given {
    One(AstType),
    Every,
}
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Kind(Option<Node>),
    Accounted(Vec<AstType>),
}
type Input = Given;
type Value = Seen;
type Failure = ();
const LIBRARY_KINDS: &[AstType] = &[
    AstType::Program,
    AstType::IdentifierName,
    AstType::IdentifierReference,
    AstType::BindingIdentifier,
    AstType::LabelIdentifier,
    AstType::ThisExpression,
    AstType::ArrayExpression,
    AstType::Elision,
    AstType::ObjectExpression,
    AstType::ObjectProperty,
    AstType::TemplateLiteral,
    AstType::TaggedTemplateExpression,
    AstType::TemplateElement,
    AstType::ComputedMemberExpression,
    AstType::StaticMemberExpression,
    AstType::PrivateFieldExpression,
    AstType::CallExpression,
    AstType::NewExpression,
    AstType::ImportMeta,
    AstType::NewTarget,
    AstType::SpreadElement,
    AstType::UpdateExpression,
    AstType::UnaryExpression,
    AstType::BinaryExpression,
    AstType::PrivateInExpression,
    AstType::LogicalExpression,
    AstType::ConditionalExpression,
    AstType::AssignmentExpression,
    AstType::ArrayAssignmentTarget,
    AstType::ObjectAssignmentTarget,
    AstType::AssignmentTargetRest,
    AstType::AssignmentTargetWithDefault,
    AstType::AssignmentTargetPropertyIdentifier,
    AstType::AssignmentTargetPropertyProperty,
    AstType::SequenceExpression,
    AstType::Super,
    AstType::AwaitExpression,
    AstType::ChainExpression,
    AstType::ParenthesizedExpression,
    AstType::Directive,
    AstType::Hashbang,
    AstType::BlockStatement,
    AstType::VariableDeclaration,
    AstType::VariableDeclarator,
    AstType::EmptyStatement,
    AstType::ExpressionStatement,
    AstType::IfStatement,
    AstType::DoWhileStatement,
    AstType::WhileStatement,
    AstType::ForStatement,
    AstType::ForInStatement,
    AstType::ForOfStatement,
    AstType::ContinueStatement,
    AstType::BreakStatement,
    AstType::ReturnStatement,
    AstType::WithStatement,
    AstType::SwitchStatement,
    AstType::SwitchCase,
    AstType::LabeledStatement,
    AstType::ThrowStatement,
    AstType::TryStatement,
    AstType::CatchClause,
    AstType::CatchParameter,
    AstType::DebuggerStatement,
    AstType::AssignmentPattern,
    AstType::ObjectPattern,
    AstType::BindingProperty,
    AstType::ArrayPattern,
    AstType::BindingRestElement,
    AstType::Function,
    AstType::FormalParameters,
    AstType::FormalParameter,
    AstType::FormalParameterRest,
    AstType::FunctionBody,
    AstType::ArrowFunctionExpression,
    AstType::YieldExpression,
    AstType::Class,
    AstType::ClassBody,
    AstType::MethodDefinition,
    AstType::PropertyDefinition,
    AstType::PrivateIdentifier,
    AstType::StaticBlock,
    AstType::AccessorProperty,
    AstType::ImportExpression,
    AstType::ImportDeclaration,
    AstType::ImportSpecifier,
    AstType::ImportDefaultSpecifier,
    AstType::ImportNamespaceSpecifier,
    AstType::WithClause,
    AstType::ImportAttribute,
    AstType::ExportNamedDeclaration,
    AstType::ExportDefaultDeclaration,
    AstType::ExportAllDeclaration,
    AstType::ExportSpecifier,
    AstType::V8IntrinsicExpression,
    AstType::BooleanLiteral,
    AstType::NullLiteral,
    AstType::NumericLiteral,
    AstType::StringLiteral,
    AstType::BigIntLiteral,
    AstType::RegExpLiteral,
    AstType::JSXElement,
    AstType::JSXOpeningElement,
    AstType::JSXClosingElement,
    AstType::JSXFragment,
    AstType::JSXOpeningFragment,
    AstType::JSXClosingFragment,
    AstType::JSXNamespacedName,
    AstType::JSXMemberExpression,
    AstType::JSXExpressionContainer,
    AstType::JSXEmptyExpression,
    AstType::JSXAttribute,
    AstType::JSXSpreadAttribute,
    AstType::JSXIdentifier,
    AstType::JSXSpreadChild,
    AstType::JSXText,
    AstType::TSThisParameter,
    AstType::TSEnumDeclaration,
    AstType::TSEnumBody,
    AstType::TSEnumMember,
    AstType::TSTypeAnnotation,
    AstType::TSLiteralType,
    AstType::TSConditionalType,
    AstType::TSUnionType,
    AstType::TSIntersectionType,
    AstType::TSParenthesizedType,
    AstType::TSTypeOperator,
    AstType::TSArrayType,
    AstType::TSIndexedAccessType,
    AstType::TSTupleType,
    AstType::TSNamedTupleMember,
    AstType::TSOptionalType,
    AstType::TSRestType,
    AstType::TSAnyKeyword,
    AstType::TSStringKeyword,
    AstType::TSBooleanKeyword,
    AstType::TSNumberKeyword,
    AstType::TSNeverKeyword,
    AstType::TSIntrinsicKeyword,
    AstType::TSUnknownKeyword,
    AstType::TSNullKeyword,
    AstType::TSUndefinedKeyword,
    AstType::TSVoidKeyword,
    AstType::TSSymbolKeyword,
    AstType::TSThisType,
    AstType::TSObjectKeyword,
    AstType::TSBigIntKeyword,
    AstType::TSTypeReference,
    AstType::TSQualifiedName,
    AstType::TSTypeParameterInstantiation,
    AstType::TSTypeParameter,
    AstType::TSTypeParameterDeclaration,
    AstType::TSTypeAliasDeclaration,
    AstType::TSClassImplements,
    AstType::TSInterfaceDeclaration,
    AstType::TSInterfaceBody,
    AstType::TSPropertySignature,
    AstType::TSIndexSignature,
    AstType::TSCallSignatureDeclaration,
    AstType::TSMethodSignature,
    AstType::TSConstructSignatureDeclaration,
    AstType::TSIndexSignatureName,
    AstType::TSInterfaceHeritage,
    AstType::TSTypePredicate,
    AstType::TSModuleDeclaration,
    AstType::TSGlobalDeclaration,
    AstType::TSModuleBlock,
    AstType::TSTypeLiteral,
    AstType::TSInferType,
    AstType::TSTypeQuery,
    AstType::TSImportType,
    AstType::TSImportTypeQualifiedName,
    AstType::TSFunctionType,
    AstType::TSConstructorType,
    AstType::TSMappedType,
    AstType::TSTemplateLiteralType,
    AstType::TSAsExpression,
    AstType::TSSatisfiesExpression,
    AstType::TSTypeAssertion,
    AstType::TSImportEqualsDeclaration,
    AstType::TSExternalModuleReference,
    AstType::TSNonNullExpression,
    AstType::Decorator,
    AstType::TSExportAssignment,
    AstType::TSNamespaceExportDeclaration,
    AstType::TSInstantiationExpression,
    AstType::JSDocNullableType,
    AstType::JSDocNonNullableType,
    AstType::JSDocUnknownType,
];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("program_has_standard_identity"),
        word: Word::Succeeds,
        input: || Given::One(AstType::Program),
        expect: Expect::Value(Seen::Kind(Some(Node::Program))),
    },
    Case {
        name: CaseName("binding_identifier_has_standard_identity"),
        word: Word::Succeeds,
        input: || Given::One(AstType::BindingIdentifier),
        expect: Expect::Value(Seen::Kind(Some(Node::Identifier))),
    },
    Case {
        name: CaseName("number_has_standard_literal_identity"),
        word: Word::Succeeds,
        input: || Given::One(AstType::NumericLiteral),
        expect: Expect::Value(Seen::Kind(Some(Node::Literal))),
    },
    Case {
        name: CaseName("jsx_element_has_standard_identity"),
        word: Word::Succeeds,
        input: || Given::One(AstType::JSXElement),
        expect: Expect::Value(Seen::Kind(Some(Node::JSXElement))),
    },
    Case {
        name: CaseName("typed_syntax_is_outside_javascript"),
        word: Word::Succeeds,
        input: || Given::One(AstType::TSAnyKeyword),
        expect: Expect::Value(Seen::Kind(None)),
    },
    Case {
        name: CaseName("every_upstream_kind_is_mapped_or_explicitly_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(Vec::new())),
    },
];
fn library_kinds() -> Vec<AstType> {
    let source = include_str!("../../../../../tests/fixtures/standard/ast_kind.rs");
    let declaration = source
        .split("pub enum AstType {")
        .nth(1)
        .expect("upstream enum exists")
        .split('}')
        .next()
        .expect("enum closes");
    let names: Vec<_> = declaration
        .lines()
        .filter_map(|line| line.trim().split_once(" = ").map(|(name, _)| name))
        .collect();
    let ours: Vec<_> = LIBRARY_KINDS
        .iter()
        .map(|kind| format!("{kind:?}"))
        .collect();
    assert_eq!(
        names, ours,
        "census is the pinned upstream enumeration in declaration order"
    );
    assert_eq!(
        LIBRARY_KINDS.len(),
        usize::from(oxc_ast::ast_kind::AST_TYPE_MAX) + 1,
        "installed parser matches pinned census"
    );
    LIBRARY_KINDS.to_vec()
}
fn dropped(kind: AstType) -> Option<&'static str> {
    let name = format!("{kind:?}");
    if name.starts_with("TS")
        || name.starts_with("JSDoc")
        || kind == AstType::AccessorProperty
        || kind == AstType::Decorator
        || kind == AstType::V8IntrinsicExpression
    {
        return Some("TypeScript and decorator extensions are outside ECMAScript 2025.");
    }
    if matches!(
        kind,
        AstType::Elision
            | AstType::ParenthesizedExpression
            | AstType::CatchParameter
            | AstType::FormalParameters
            | AstType::FormalParameterRest
            | AstType::WithClause
            | AstType::Hashbang
    ) {
        return Some(
            "Parser bookkeeping is transparent; its children or source lexemes remain in ESTree.",
        );
    }
    if matches!(
        kind,
        AstType::Function | AstType::Class | AstType::FormalParameter
    ) {
        return Some(
            "The runtime typed schema resolves declaration/expression or default-parameter identity from fields.",
        );
    }
    None
}
fn accounted(kind: AstType) -> bool {
    if let Some(reason) = dropped(kind) {
        return !reason.is_empty();
    }
    node_type(kind).is_some()
}
fn subject(given: Input) -> Value {
    match given {
        Given::One(kind) => Seen::Kind(node_type(kind)),
        Given::Every => Seen::Accounted(
            library_kinds()
                .into_iter()
                .filter(|kind| !accounted(*kind))
                .collect(),
        ),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
