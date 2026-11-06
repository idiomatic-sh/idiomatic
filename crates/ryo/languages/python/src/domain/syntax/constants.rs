//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// Syntactic context for a candidate quoted expression, before lexical typing classification.
pub(crate) const ANNOTATION_CONTEXT: ryo::languages::PropertyId = ryo::languages::PropertyId(7);
/// A string occurring within an explicit annotation field.
pub(crate) const ANNOTATION_SEED: &str = "annotation";
/// A call argument or annotated value requiring semantic classification.
pub(crate) const CANDIDATE_SEED: &str = "candidate";

/// An expression no relation reads by itself.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const EXPRESSION: &str = "an expression no relation reads by itself: the relations read the names, calls and strings inside it";

/// A node the standard lacks, which the relations read through the construct it is in.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const EXTRA: &str =
    "a node the standard lacks, which the relations read through the construct it is in";

/// The text of a name, a keyword, a number or a string.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const LITERAL_TOKEN: &str = "the text of a name, a keyword, a number or a string: the relations read the node it is in, and `string_literal` its decoded text";

/// Never given.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NEVER_GIVEN: &str =
    "never given: the lexer gives each operator and delimiter its exact type";

/// Never given for a file.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NOT_A_FILE: &str = "never given for a file: the standard's form for an expression, an interactive line or a function type parsed alone";

/// Never in a module file.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NOT_IN_A_MODULE: &str =
    "never in a module file: the library gives it for an IPython notebook";

/// Never given.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NOT_SPLIT: &str = "never given: the language does not split what a keyword token says; the unsplit kind with its `async` or `except*` token holds it";

/// Never given.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NO_TYPE_COMMENTS: &str = "never given: the library reads no type comments";

/// Never a node of the tree.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const OPERATOR: &str = "never a node of the tree: the operator's token holds it, and a context is where the expression stands";

/// A part the relations read through the construct it is in.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PART: &str = "a part the relations read through the construct it is in";

/// A pattern that binds no name of its own.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PATTERN: &str =
    "a pattern that binds no name of its own: `defines` reads the captures inside it";

/// A piece of punctuation or an operator.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PUNCTUATION: &str = "a piece of punctuation or an operator: syntax between the things the relations read, which read the construct it belongs to";

/// A statement no relation reads by itself.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STATEMENT: &str = "a statement no relation reads by itself: the relations read the definitions, names, calls and strings inside it";

/// White space, a line break or an indentation.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TRIVIA: &str =
    "white space, a line break or an indentation: `syntax_tree` holds it as it stands";

/// Text the lexer could not read as any token.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "text the lexer could not read as any token: `syntax_error` reads what the library said of it";

/// Canonical full identifier spelling when normalization changes its source text.
pub(crate) const NORMALIZED_IDENTIFIER: ryo::languages::PropertyId = ryo::languages::PropertyId(1);

/// An incomplete frontend cannot establish the complete identifier set.
pub(crate) const IDENTIFIERS_INCOMPLETE: &str = "identifiers require a complete syntax tree";

/// Literal category preserved from the parser's typed scalar payload.
pub(crate) const SCALAR_KIND: ryo::languages::PropertyId = ryo::languages::PropertyId(2);
/// Canonical scalar payload, independent of its source spelling.
pub(crate) const SCALAR_VALUE: ryo::languages::PropertyId = ryo::languages::PropertyId(3);
/// The imaginary component of a complex scalar, as IEEE bits.
pub(crate) const SCALAR_IMAGINARY: ryo::languages::PropertyId = ryo::languages::PropertyId(4);
/// Meaningful debug text retained by a formatted/template interpolation.
pub(crate) const INTERPOLATION_DEBUG: ryo::languages::PropertyId = ryo::languages::PropertyId(5);
/// Explicit interpolation conversion, distinct from no conversion.
pub(crate) const INTERPOLATION_CONVERSION: ryo::languages::PropertyId =
    ryo::languages::PropertyId(6);
/// Typed numeric payload categories; these are AST identity, not runtime equality.
/// Parser-derived integer payload category.
pub(crate) const INTEGER_SCALAR: &str = "integer";
/// Parser-derived float payload category.
pub(crate) const FLOAT_SCALAR: &str = "float";
/// Parser-derived complex payload category.
pub(crate) const COMPLEX_SCALAR: &str = "complex";
/// Parser-derived bytes payload category.
pub(crate) const BYTES_SCALAR: &str = "bytes";

/// A parsed boolean is distinct from an integer of equal runtime value.
pub(crate) const BOOLEAN_SCALAR: &str = "boolean";
/// The unique None literal category.
pub(crate) const NONE_SCALAR: &str = "none";
/// The unique ellipsis literal category.
pub(crate) const ELLIPSIS_SCALAR: &str = "ellipsis";

/// Compiler future-annotations activation from a validated module prefix.
pub(crate) const MODULE_FUTURE_ANNOTATIONS: ryo::languages::PropertyId =
    ryo::languages::PropertyId(8);

/// A valid module-prefix future directive enabled postponed annotations.
pub(crate) const MODULE_FUTURE_ANNOTATIONS_ENABLED: &str = "enabled";
/// No valid module compiler directive enables postponed annotations.
pub(crate) const MODULE_FUTURE_ANNOTATIONS_DISABLED: &str = "disabled";

/// The keyword of a coroutine definition or comprehension, as its `NAME` token writes it.
pub(crate) const ASYNC: &str = "async";
