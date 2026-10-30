//! The types of the `expressions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a call is one the library writes as a conversion to a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Conversion {
    /// The library reads the callee as a type: `*(*T)(x)` is a conversion to the type `*(*T)`.
    Written,
    /// The library reads a call.
    Call,
}
