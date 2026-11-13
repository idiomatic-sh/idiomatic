//! The words of the `definitions` concept: what a declaration is in Go's own words.

use crate::domain::signatures::types::SignatureRefinement;

/// A function declared outside a type.
pub(crate) const FUNCTION: SignatureRefinement = SignatureRefinement("function");
/// A function declared with a receiver.
pub(crate) const METHOD: SignatureRefinement = SignatureRefinement("method");
/// A type declared as a struct.
pub(crate) const STRUCT: SignatureRefinement = SignatureRefinement("struct");
/// A type declared as an interface.
pub(crate) const INTERFACE: SignatureRefinement = SignatureRefinement("interface");
/// A name for a type that already has one.
pub(crate) const ALIAS: SignatureRefinement = SignatureRefinement("alias");
/// A type declared as any other type.
pub(crate) const DEFINED_TYPE: SignatureRefinement = SignatureRefinement("type");
/// A name `const` declares.
pub(crate) const CONSTANT: SignatureRefinement = SignatureRefinement("const");
/// A name `var` declares.
pub(crate) const VARIABLE: SignatureRefinement = SignatureRefinement("var");
/// A name `:=` declares.
pub(crate) const SHORT_VARIABLE: SignatureRefinement = SignatureRefinement("short-var");
/// A name a `range` clause declares.
pub(crate) const RANGE_VARIABLE: SignatureRefinement = SignatureRefinement("range-var");
/// A name a type switch declares.
pub(crate) const TYPE_SWITCH_VARIABLE: SignatureRefinement = SignatureRefinement("type-switch-var");
/// A name an import brings in.
pub(crate) const IMPORT: SignatureRefinement = SignatureRefinement("import");
/// A field of a struct.
pub(crate) const FIELD: SignatureRefinement = SignatureRefinement("field");
/// A type embedded in a struct, which declares a field named by it.
pub(crate) const EMBEDDED_FIELD: SignatureRefinement = SignatureRefinement("embedded-field");
/// A method of an interface.
pub(crate) const INTERFACE_METHOD: SignatureRefinement = SignatureRefinement("interface-method");
/// A type parameter of a function or a type.
pub(crate) const TYPE_PARAMETER: SignatureRefinement = SignatureRefinement("type-parameter");
/// A label a labeled statement declares.
pub(crate) const LABEL: SignatureRefinement = SignatureRefinement("label");
