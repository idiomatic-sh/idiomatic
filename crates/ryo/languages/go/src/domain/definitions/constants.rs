//! The words of the `definitions` concept: what a declaration is in Go's own words.

use crate::domain::signatures::types::Refinement;

/// A function declared outside a type.
pub(crate) const FUNCTION: Refinement = Refinement("function");
/// A function declared with a receiver.
pub(crate) const METHOD: Refinement = Refinement("method");
/// A type declared as a struct.
pub(crate) const STRUCT: Refinement = Refinement("struct");
/// A type declared as an interface.
pub(crate) const INTERFACE: Refinement = Refinement("interface");
/// A name for a type that already has one.
pub(crate) const ALIAS: Refinement = Refinement("alias");
/// A type declared as any other type.
pub(crate) const DEFINED_TYPE: Refinement = Refinement("type");
/// A name `const` declares.
pub(crate) const CONSTANT: Refinement = Refinement("const");
/// A name `var` declares.
pub(crate) const VARIABLE: Refinement = Refinement("var");
/// A name `:=` declares.
pub(crate) const SHORT_VARIABLE: Refinement = Refinement("short-var");
/// A name a `range` clause declares.
pub(crate) const RANGE_VARIABLE: Refinement = Refinement("range-var");
/// A name a type switch declares.
pub(crate) const TYPE_SWITCH_VARIABLE: Refinement = Refinement("type-switch-var");
/// A name an import brings in.
pub(crate) const IMPORT: Refinement = Refinement("import");
/// A field of a struct.
pub(crate) const FIELD: Refinement = Refinement("field");
/// A type embedded in a struct, which declares a field named by it.
pub(crate) const EMBEDDED_FIELD: Refinement = Refinement("embedded-field");
/// A method of an interface.
pub(crate) const INTERFACE_METHOD: Refinement = Refinement("interface-method");
/// A type parameter of a function or a type.
pub(crate) const TYPE_PARAMETER: Refinement = Refinement("type-parameter");
/// A label a labeled statement declares.
pub(crate) const LABEL: Refinement = Refinement("label");
