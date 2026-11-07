//! Plain data of exact numeric constant keys.

/// An integer that is safe to compare exactly across supported numeric literal types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ExactInteger(pub i128);

/// The IEEE-754 payload of a float key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FloatPayload(pub u64);

/// A binary64 value while a key is composed or negated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Binary64(pub f64);

impl std::ops::Neg for Binary64 {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

/// A real or imaginary component after Python's bounded constant normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RealKey {
    /// An integer that is safe to compare exactly across supported numeric literal types.
    Integer(ExactInteger),
    /// The IEEE-754 payload, with both signed zeros canonicalized before storage.
    Float(FloatPayload),
}

/// The tagged real and imaginary components of a numeric constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NumberKey {
    /// The real component.
    pub real: RealKey,
    /// The imaginary component; ordinary reals use integer zero.
    pub imaginary: RealKey,
}

/// The numeric value available for a supported expression, or its exact structural fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumericState {
    /// A fully normalized numeric value.
    Number(NumberKey),
    /// A syntactically numeric value outside the numeric normalization range.
    StructuralInteger,
}
