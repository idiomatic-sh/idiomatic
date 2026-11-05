//! The pinned standard's closed node vocabulary.
/// The kinds of node a JavaScript tree has, by the standard's names.
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
pub(crate) enum JavaScriptNode {
    /// `Program` in the standard.
    Program,
    /// `Identifier` in the standard.
    Identifier,
    /// `PrivateIdentifier` in the standard.
    PrivateIdentifier,
    /// `Literal` in the standard.
    Literal,
    /// `ExpressionStatement` in the standard.
    ExpressionStatement,
    /// `BlockStatement` in the standard.
    BlockStatement,
    /// `EmptyStatement` in the standard.
    EmptyStatement,
    /// `DebuggerStatement` in the standard.
    DebuggerStatement,
    /// `WithStatement` in the standard.
    WithStatement,
    /// `ReturnStatement` in the standard.
    ReturnStatement,
    /// `LabeledStatement` in the standard.
    LabeledStatement,
    /// `BreakStatement` in the standard.
    BreakStatement,
    /// `ContinueStatement` in the standard.
    ContinueStatement,
    /// `IfStatement` in the standard.
    IfStatement,
    /// `SwitchStatement` in the standard.
    SwitchStatement,
    /// `SwitchCase` in the standard.
    SwitchCase,
    /// `ThrowStatement` in the standard.
    ThrowStatement,
    /// `TryStatement` in the standard.
    TryStatement,
    /// `CatchClause` in the standard.
    CatchClause,
    /// `WhileStatement` in the standard.
    WhileStatement,
    /// `DoWhileStatement` in the standard.
    DoWhileStatement,
    /// `ForStatement` in the standard.
    ForStatement,
    /// `ForInStatement` in the standard.
    ForInStatement,
    /// `ForOfStatement` in the standard.
    ForOfStatement,
    /// `FunctionDeclaration` in the standard.
    FunctionDeclaration,
    /// `FunctionExpression` in the standard.
    FunctionExpression,
    /// `ArrowFunctionExpression` in the standard.
    ArrowFunctionExpression,
    /// `VariableDeclaration` in the standard.
    VariableDeclaration,
    /// `VariableDeclarator` in the standard.
    VariableDeclarator,
    /// `ThisExpression` in the standard.
    ThisExpression,
    /// `Super` in the standard.
    Super,
    /// `ArrayExpression` in the standard.
    ArrayExpression,
    /// `ObjectExpression` in the standard.
    ObjectExpression,
    /// `Property` in the standard.
    Property,
    /// `SequenceExpression` in the standard.
    SequenceExpression,
    /// `UnaryExpression` in the standard.
    UnaryExpression,
    /// `BinaryExpression` in the standard.
    BinaryExpression,
    /// `AssignmentExpression` in the standard.
    AssignmentExpression,
    /// `UpdateExpression` in the standard.
    UpdateExpression,
    /// `LogicalExpression` in the standard.
    LogicalExpression,
    /// `ConditionalExpression` in the standard.
    ConditionalExpression,
    /// `NewExpression` in the standard.
    NewExpression,
    /// `CallExpression` in the standard.
    CallExpression,
    /// `MemberExpression` in the standard.
    MemberExpression,
    /// `YieldExpression` in the standard.
    YieldExpression,
    /// `AwaitExpression` in the standard.
    AwaitExpression,
    /// `TemplateLiteral` in the standard.
    TemplateLiteral,
    /// `TaggedTemplateExpression` in the standard.
    TaggedTemplateExpression,
    /// `TemplateElement` in the standard.
    TemplateElement,
    /// `SpreadElement` in the standard.
    SpreadElement,
    /// `RestElement` in the standard.
    RestElement,
    /// `AssignmentPattern` in the standard.
    AssignmentPattern,
    /// `ArrayPattern` in the standard.
    ArrayPattern,
    /// `ObjectPattern` in the standard.
    ObjectPattern,
    /// `ClassDeclaration` in the standard.
    ClassDeclaration,
    /// `ClassExpression` in the standard.
    ClassExpression,
    /// `ClassBody` in the standard.
    ClassBody,
    /// `MethodDefinition` in the standard.
    MethodDefinition,
    /// `PropertyDefinition` in the standard.
    PropertyDefinition,
    /// `StaticBlock` in the standard.
    StaticBlock,
    /// `MetaProperty` in the standard.
    MetaProperty,
    /// `ImportDeclaration` in the standard.
    ImportDeclaration,
    /// `ImportSpecifier` in the standard.
    ImportSpecifier,
    /// `ImportDefaultSpecifier` in the standard.
    ImportDefaultSpecifier,
    /// `ImportNamespaceSpecifier` in the standard.
    ImportNamespaceSpecifier,
    /// `ImportExpression` in the standard.
    ImportExpression,
    /// `ImportAttribute` in the standard.
    ImportAttribute,
    /// `ExportNamedDeclaration` in the standard.
    ExportNamedDeclaration,
    /// `ExportDefaultDeclaration` in the standard.
    ExportDefaultDeclaration,
    /// `ExportAllDeclaration` in the standard.
    ExportAllDeclaration,
    /// `ExportSpecifier` in the standard.
    ExportSpecifier,
    /// `ChainExpression` in the standard.
    ChainExpression,
    /// `JSXElement` in the JSX extension of the standard.
    JSXElement,
    /// `JSXOpeningElement` in the JSX extension of the standard.
    JSXOpeningElement,
    /// `JSXClosingElement` in the JSX extension of the standard.
    JSXClosingElement,
    /// `JSXFragment` in the JSX extension of the standard.
    JSXFragment,
    /// `JSXOpeningFragment` in the JSX extension of the standard.
    JSXOpeningFragment,
    /// `JSXClosingFragment` in the JSX extension of the standard.
    JSXClosingFragment,
    /// `JSXNamespacedName` in the JSX extension of the standard.
    JSXNamespacedName,
    /// `JSXMemberExpression` in the JSX extension of the standard.
    JSXMemberExpression,
    /// `JSXExpressionContainer` in the JSX extension of the standard.
    JSXExpressionContainer,
    /// `JSXEmptyExpression` in the JSX extension of the standard.
    JSXEmptyExpression,
    /// `JSXAttribute` in the JSX extension of the standard.
    JSXAttribute,
    /// `JSXSpreadAttribute` in the JSX extension of the standard.
    JSXSpreadAttribute,
    /// `JSXIdentifier` in the JSX extension of the standard.
    JSXIdentifier,
    /// `JSXSpreadChild` in the JSX extension of the standard.
    JSXSpreadChild,
    /// `JSXText` in the JSX extension of the standard.
    JSXText,
}
