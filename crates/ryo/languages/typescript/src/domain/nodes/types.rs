//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is named
//! as the standard names it, in kebab case.

/// What a node of a typescript syntax tree is. One closed list, each name once.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Node {
    /// The standard’s `AccessorProperty` kind.
    AccessorProperty,
    /// The standard’s `ArrayExpression` kind.
    ArrayExpression,
    /// The standard’s `ArrayPattern` kind.
    ArrayPattern,
    /// The standard’s `ArrowFunctionExpression` kind.
    ArrowFunctionExpression,
    /// The standard’s `AssignmentExpression` kind.
    AssignmentExpression,
    /// The standard’s `AssignmentPattern` kind.
    AssignmentPattern,
    /// The standard’s `AwaitExpression` kind.
    AwaitExpression,
    /// The standard’s `BinaryExpression` kind.
    BinaryExpression,
    /// The standard’s `BlockStatement` kind.
    BlockStatement,
    /// The standard’s `BreakStatement` kind.
    BreakStatement,
    /// The standard’s `CallExpression` kind.
    CallExpression,
    /// The standard’s `CatchClause` kind.
    CatchClause,
    /// The standard’s `ChainExpression` kind.
    ChainExpression,
    /// The standard’s `ClassBody` kind.
    ClassBody,
    /// The standard’s `ClassDeclaration` kind.
    ClassDeclaration,
    /// The standard’s `ClassExpression` kind.
    ClassExpression,
    /// The standard’s `ConditionalExpression` kind.
    ConditionalExpression,
    /// The standard’s `ContinueStatement` kind.
    ContinueStatement,
    /// The standard’s `DebuggerStatement` kind.
    DebuggerStatement,
    /// The standard’s `Decorator` kind.
    Decorator,
    /// The standard’s `DoWhileStatement` kind.
    DoWhileStatement,
    /// The standard’s `EmptyStatement` kind.
    EmptyStatement,
    /// The standard’s `ExportAllDeclaration` kind.
    ExportAllDeclaration,
    /// The standard’s `ExportDefaultDeclaration` kind.
    ExportDefaultDeclaration,
    /// The standard’s `ExportNamedDeclaration` kind.
    ExportNamedDeclaration,
    /// The standard’s `ExportSpecifier` kind.
    ExportSpecifier,
    /// The standard’s `ExpressionStatement` kind.
    ExpressionStatement,
    /// The standard’s `ForInStatement` kind.
    ForInStatement,
    /// The standard’s `ForOfStatement` kind.
    ForOfStatement,
    /// The standard’s `ForStatement` kind.
    ForStatement,
    /// The standard’s `FunctionDeclaration` kind.
    FunctionDeclaration,
    /// The standard’s `FunctionExpression` kind.
    FunctionExpression,
    /// The standard’s `Identifier` kind.
    Identifier,
    /// The standard’s `IfStatement` kind.
    IfStatement,
    /// The standard’s `ImportAttribute` kind.
    ImportAttribute,
    /// The standard’s `ImportDeclaration` kind.
    ImportDeclaration,
    /// The standard’s `ImportDefaultSpecifier` kind.
    ImportDefaultSpecifier,
    /// The standard’s `ImportExpression` kind.
    ImportExpression,
    /// The standard’s `ImportNamespaceSpecifier` kind.
    ImportNamespaceSpecifier,
    /// The standard’s `ImportSpecifier` kind.
    ImportSpecifier,
    /// The standard’s `JSXAttribute` kind.
    JSXAttribute,
    /// The standard’s `JSXClosingElement` kind.
    JSXClosingElement,
    /// The standard’s `JSXClosingFragment` kind.
    JSXClosingFragment,
    /// The standard’s `JSXElement` kind.
    JSXElement,
    /// The standard’s `JSXEmptyExpression` kind.
    JSXEmptyExpression,
    /// The standard’s `JSXExpressionContainer` kind.
    JSXExpressionContainer,
    /// The standard’s `JSXFragment` kind.
    JSXFragment,
    /// The standard’s `JSXIdentifier` kind.
    JSXIdentifier,
    /// The standard’s `JSXMemberExpression` kind.
    JSXMemberExpression,
    /// The standard’s `JSXNamespacedName` kind.
    JSXNamespacedName,
    /// The standard’s `JSXOpeningElement` kind.
    JSXOpeningElement,
    /// The standard’s `JSXOpeningFragment` kind.
    JSXOpeningFragment,
    /// The standard’s `JSXSpreadAttribute` kind.
    JSXSpreadAttribute,
    /// The standard’s `JSXSpreadChild` kind.
    JSXSpreadChild,
    /// The standard’s `JSXText` kind.
    JSXText,
    /// The standard’s `LabeledStatement` kind.
    LabeledStatement,
    /// The standard’s `Literal` kind.
    Literal,
    /// The standard’s `LogicalExpression` kind.
    LogicalExpression,
    /// The standard’s `MemberExpression` kind.
    MemberExpression,
    /// The standard’s `MetaProperty` kind.
    MetaProperty,
    /// The standard’s `MethodDefinition` kind.
    MethodDefinition,
    /// The standard’s `NewExpression` kind.
    NewExpression,
    /// The standard’s `ObjectExpression` kind.
    ObjectExpression,
    /// The standard’s `ObjectPattern` kind.
    ObjectPattern,
    /// The standard’s `PrivateIdentifier` kind.
    PrivateIdentifier,
    /// The standard’s `Program` kind.
    Program,
    /// The standard’s `Property` kind.
    Property,
    /// The standard’s `PropertyDefinition` kind.
    PropertyDefinition,
    /// The standard’s `RestElement` kind.
    RestElement,
    /// The standard’s `ReturnStatement` kind.
    ReturnStatement,
    /// The standard’s `SequenceExpression` kind.
    SequenceExpression,
    /// The standard’s `SpreadElement` kind.
    SpreadElement,
    /// The standard’s `StaticBlock` kind.
    StaticBlock,
    /// The standard’s `Super` kind.
    Super,
    /// The standard’s `SwitchCase` kind.
    SwitchCase,
    /// The standard’s `SwitchStatement` kind.
    SwitchStatement,
    /// The standard’s `TaggedTemplateExpression` kind.
    TaggedTemplateExpression,
    /// The standard’s `TemplateElement` kind.
    TemplateElement,
    /// The standard’s `TemplateLiteral` kind.
    TemplateLiteral,
    /// The standard’s `ThisExpression` kind.
    ThisExpression,
    /// The standard’s `ThrowStatement` kind.
    ThrowStatement,
    /// The standard’s `TryStatement` kind.
    TryStatement,
    /// The standard’s `UnaryExpression` kind.
    UnaryExpression,
    /// The standard’s `UpdateExpression` kind.
    UpdateExpression,
    /// The standard’s `VariableDeclaration` kind.
    VariableDeclaration,
    /// The standard’s `VariableDeclarator` kind.
    VariableDeclarator,
    /// The standard’s `WhileStatement` kind.
    WhileStatement,
    /// The standard’s `WithStatement` kind.
    WithStatement,
    /// The standard’s `YieldExpression` kind.
    YieldExpression,
    /// The standard’s `TSAbstractAccessorProperty` kind.
    TSAbstractAccessorProperty,
    /// The standard’s `TSAbstractKeyword` kind.
    TSAbstractKeyword,
    /// The standard’s `TSAbstractMethodDefinition` kind.
    TSAbstractMethodDefinition,
    /// The standard’s `TSAbstractPropertyDefinition` kind.
    TSAbstractPropertyDefinition,
    /// The standard’s `TSAnyKeyword` kind.
    TSAnyKeyword,
    /// The standard’s `TSArrayType` kind.
    TSArrayType,
    /// The standard’s `TSAsExpression` kind.
    TSAsExpression,
    /// The standard’s `TSAsyncKeyword` kind.
    TSAsyncKeyword,
    /// The standard’s `TSBigIntKeyword` kind.
    TSBigIntKeyword,
    /// The standard’s `TSBooleanKeyword` kind.
    TSBooleanKeyword,
    /// The standard’s `TSCallSignatureDeclaration` kind.
    TSCallSignatureDeclaration,
    /// The standard’s `TSClassImplements` kind.
    TSClassImplements,
    /// The standard’s `TSConditionalType` kind.
    TSConditionalType,
    /// The standard’s `TSConstructorType` kind.
    TSConstructorType,
    /// The standard’s `TSConstructSignatureDeclaration` kind.
    TSConstructSignatureDeclaration,
    /// The standard’s `TSDeclareFunction` kind.
    TSDeclareFunction,
    /// The standard’s `TSDeclareKeyword` kind.
    TSDeclareKeyword,
    /// The standard’s `TSEmptyBodyFunctionExpression` kind.
    TSEmptyBodyFunctionExpression,
    /// The standard’s `TSEnumBody` kind.
    TSEnumBody,
    /// The standard’s `TSEnumDeclaration` kind.
    TSEnumDeclaration,
    /// The standard’s `TSEnumMember` kind.
    TSEnumMember,
    /// The standard’s `TSExportAssignment` kind.
    TSExportAssignment,
    /// The standard’s `TSExportKeyword` kind.
    TSExportKeyword,
    /// The standard’s `TSExternalModuleReference` kind.
    TSExternalModuleReference,
    /// The standard’s `TSFunctionType` kind.
    TSFunctionType,
    /// The standard’s `TSImportEqualsDeclaration` kind.
    TSImportEqualsDeclaration,
    /// The standard’s `TSImportType` kind.
    TSImportType,
    /// The standard’s `TSIndexedAccessType` kind.
    TSIndexedAccessType,
    /// The standard’s `TSIndexSignature` kind.
    TSIndexSignature,
    /// The standard’s `TSInferType` kind.
    TSInferType,
    /// The standard’s `TSInstantiationExpression` kind.
    TSInstantiationExpression,
    /// The standard’s `TSInterfaceBody` kind.
    TSInterfaceBody,
    /// The standard’s `TSInterfaceDeclaration` kind.
    TSInterfaceDeclaration,
    /// The standard’s `TSInterfaceHeritage` kind.
    TSInterfaceHeritage,
    /// The standard’s `TSIntersectionType` kind.
    TSIntersectionType,
    /// The standard’s `TSIntrinsicKeyword` kind.
    TSIntrinsicKeyword,
    /// The standard’s `TSLiteralType` kind.
    TSLiteralType,
    /// The standard’s `TSMappedType` kind.
    TSMappedType,
    /// The standard’s `TSMethodSignature` kind.
    TSMethodSignature,
    /// The standard’s `TSModuleBlock` kind.
    TSModuleBlock,
    /// The standard’s `TSModuleDeclaration` kind.
    TSModuleDeclaration,
    /// The standard’s `TSNamedTupleMember` kind.
    TSNamedTupleMember,
    /// The standard’s `TSNamespaceExportDeclaration` kind.
    TSNamespaceExportDeclaration,
    /// The standard’s `TSNeverKeyword` kind.
    TSNeverKeyword,
    /// The standard’s `TSNonNullExpression` kind.
    TSNonNullExpression,
    /// The standard’s `TSNullKeyword` kind.
    TSNullKeyword,
    /// The standard’s `TSNumberKeyword` kind.
    TSNumberKeyword,
    /// The standard’s `TSObjectKeyword` kind.
    TSObjectKeyword,
    /// The standard’s `TSOptionalType` kind.
    TSOptionalType,
    /// The standard’s `TSParameterProperty` kind.
    TSParameterProperty,
    /// The standard’s `TSPrivateKeyword` kind.
    TSPrivateKeyword,
    /// The standard’s `TSPropertySignature` kind.
    TSPropertySignature,
    /// The standard’s `TSProtectedKeyword` kind.
    TSProtectedKeyword,
    /// The standard’s `TSPublicKeyword` kind.
    TSPublicKeyword,
    /// The standard’s `TSQualifiedName` kind.
    TSQualifiedName,
    /// The standard’s `TSReadonlyKeyword` kind.
    TSReadonlyKeyword,
    /// The standard’s `TSRestType` kind.
    TSRestType,
    /// The standard’s `TSSatisfiesExpression` kind.
    TSSatisfiesExpression,
    /// The standard’s `TSStaticKeyword` kind.
    TSStaticKeyword,
    /// The standard’s `TSStringKeyword` kind.
    TSStringKeyword,
    /// The standard’s `TSSymbolKeyword` kind.
    TSSymbolKeyword,
    /// The standard’s `TSTemplateLiteralType` kind.
    TSTemplateLiteralType,
    /// The standard’s `TSThisType` kind.
    TSThisType,
    /// The standard’s `TSTupleType` kind.
    TSTupleType,
    /// The standard’s `TSTypeAliasDeclaration` kind.
    TSTypeAliasDeclaration,
    /// The standard’s `TSTypeAnnotation` kind.
    TSTypeAnnotation,
    /// The standard’s `TSTypeAssertion` kind.
    TSTypeAssertion,
    /// The standard’s `TSTypeLiteral` kind.
    TSTypeLiteral,
    /// The standard’s `TSTypeOperator` kind.
    TSTypeOperator,
    /// The standard’s `TSTypeParameter` kind.
    TSTypeParameter,
    /// The standard’s `TSTypeParameterDeclaration` kind.
    TSTypeParameterDeclaration,
    /// The standard’s `TSTypeParameterInstantiation` kind.
    TSTypeParameterInstantiation,
    /// The standard’s `TSTypePredicate` kind.
    TSTypePredicate,
    /// The standard’s `TSTypeQuery` kind.
    TSTypeQuery,
    /// The standard’s `TSTypeReference` kind.
    TSTypeReference,
    /// The standard’s `TSUndefinedKeyword` kind.
    TSUndefinedKeyword,
    /// The standard’s `TSUnionType` kind.
    TSUnionType,
    /// The standard’s `TSUnknownKeyword` kind.
    TSUnknownKeyword,
    /// The standard’s `TSVoidKeyword` kind.
    TSVoidKeyword,
}
