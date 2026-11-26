//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// A string occurring within an explicit annotation field.
pub(crate) const ANNOTATION_SEED: &str = "annotation";

/// An incomplete frontend cannot establish the complete identifier set.
pub(crate) const IDENTIFIERS_INCOMPLETE: &str = "identifiers require a complete syntax tree";

/// Literal category preserved from the parser's typed scalar payload.
pub(crate) const SCALAR_KIND: ryo::languages::PropertyId = ryo::languages::PropertyId(2);
/// Parser-owned scalar payload used by structural equality.
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

/// When the module's annotations are evaluated: never where they stand under the future import or in a stub (a stub is never
/// run), else where they stand.
pub(crate) const MODULE_ANNOTATION_TIMING: ryo::languages::PropertyId =
    ryo::languages::PropertyId(9);
/// The module's annotations are postponed.
pub(crate) const MODULE_ANNOTATIONS_DEFERRED: &str = "deferred";
/// The module's annotations are evaluated where they stand.
pub(crate) const MODULE_ANNOTATIONS_IMMEDIATE: &str = "immediate";

/// What a declaration `x: T` without a value does to `x` in the module: binds it in a stub (the typing specification reads it as
/// the declaration of `x`), gives it no value in a module that runs.
pub(crate) const MODULE_ANNOTATED_DECLARATIONS: ryo::languages::PropertyId =
    ryo::languages::PropertyId(10);
/// A declaration without a value binds the name it declares.
pub(crate) const MODULE_DECLARATIONS_DECLARED: &str = "declared";
/// A declaration without a value gives the name no value.
pub(crate) const MODULE_DECLARATIONS_UNBOUND: &str = "unbound";

/// The provenance and label of a name the language itself provides.
pub(crate) const BUILTIN: &str = "builtin";

/// The label of a name or member that is not there.
pub(crate) const ABSENT: &str = "absent";

/// The separator of a dotted module path.
pub(crate) const DOT: &str = ".";
