//! The words of the binding relations, the declarations that bind names, and the names the language provides without a
//! declaration.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

/// The nodes that declare a name, each a `defines` row of every name it binds: the kinds the `defines` item reads.
pub(crate) const DECLARATIONS: &[Kind] = &[
    Kind::Node(Node::FunctionDeclaration),
    Kind::Node(Node::TSDeclareFunction),
    Kind::Node(Node::ClassDeclaration),
    Kind::Node(Node::TSInterfaceDeclaration),
    Kind::Node(Node::TSTypeAliasDeclaration),
    Kind::Node(Node::TSTypeParameter),
    Kind::Node(Node::TSEnumDeclaration),
    Kind::Node(Node::TSEnumMember),
    Kind::Node(Node::TSModuleDeclaration),
    Kind::Node(Node::VariableDeclarator),
    Kind::Node(Node::ImportSpecifier),
    Kind::Node(Node::ImportDefaultSpecifier),
    Kind::Node(Node::ImportNamespaceSpecifier),
    Kind::Node(Node::MethodDefinition),
    Kind::Node(Node::TSMethodSignature),
    Kind::Node(Node::PropertyDefinition),
    Kind::Node(Node::AccessorProperty),
    Kind::Node(Node::TSPropertySignature),
    Kind::Node(Node::LabeledStatement),
    Kind::Node(Node::FunctionExpression),
    Kind::Node(Node::ArrowFunctionExpression),
    Kind::Node(Node::TSEmptyBodyFunctionExpression),
    Kind::Node(Node::ClassExpression),
    Kind::Node(Node::CatchClause),
];

/// A reference the semantic pass resolves to one declaration.
pub(crate) const UNIQUE_DECLARATION: &str = "unique-declaration";
/// A reference the semantic pass resolves to a name declared more than once.
pub(crate) const ALTERNATIVE_DECLARATIONS: &str = "alternative-declarations";
/// An unresolved reference to a name the language provides.
pub(crate) const BUILTIN: &str = "builtin";
/// An unresolved reference to a name nothing provides.
pub(crate) const UNBOUND: &str = "unbound";

/// A `binding_use` row of the one declaration a reference names.
pub(crate) const UNIQUE: &str = "unique";
/// A `binding_use` row of one of the declarations of a name declared more than once.
pub(crate) const ALTERNATIVE: &str = "alternative";

/// The value properties of the ECMAScript global object, the names a script reads without declaring them: the `builtin` globals
/// oxlint 1.76.0 holds read-only.
pub(crate) const BUILTINS: &[&str] = &[
    "AggregateError",
    "Array",
    "ArrayBuffer",
    "Atomics",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Boolean",
    "DataView",
    "Date",
    "Error",
    "EvalError",
    "FinalizationRegistry",
    "Float16Array",
    "Float32Array",
    "Float64Array",
    "Function",
    "Infinity",
    "Int16Array",
    "Int32Array",
    "Int8Array",
    "Intl",
    "Iterator",
    "JSON",
    "Map",
    "Math",
    "NaN",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "Reflect",
    "RegExp",
    "Set",
    "SharedArrayBuffer",
    "String",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "URIError",
    "Uint16Array",
    "Uint32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "WeakMap",
    "WeakRef",
    "WeakSet",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "eval",
    "globalThis",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "undefined",
    "unescape",
];
