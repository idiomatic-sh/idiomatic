//! Plain application-owned format parser values.

/// The grammar used to parse a caller-selected format input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatDialect {
    /// Python printf-style string formatting with a `str` left operand.
    PercentString,
}

/// Why a format input could not be parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatErrorKind {
    /// A mapping key opened with `(` but did not close.
    UnmatchedKeyParentheses,
    /// A format ended before completing a conversion.
    IncompleteFormat,
    /// A conversion character is not supported by string percent formatting.
    UnsupportedFormatCharacter,
    /// A fixed width or precision exceeded the parser's accepted integer range.
    WidthOrPrecisionTooLarge,
}

/// The parser error and its position in Unicode scalar units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatParseError {
    /// The stable error category.
    pub kind: FormatErrorKind,
    /// The character the parser reported, if the category has one.
    pub(crate) character: Option<FormatCharacter>,
    /// The error position in Unicode scalar units.
    pub(crate) index: FormatScalarIndex,
}

/// Which argument a conversion consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatArgument {
    /// A positional argument.
    Positional,
    /// A mapping item named by its spelling within the parser input.
    Mapping(FormatMappingKey),
}

/// The mode for one width or precision modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatQuantityMode {
    /// No modifier.
    None,
    /// A fixed amount appears in the format text.
    Fixed,
    /// An amount is consumed from the positional value tuple.
    Star,
    /// A decimal point appears without a precision amount.
    Dot,
}

/// Standard percent-format flags, retained independently of conversion policy.
/// Repetition is idempotent; conflicting flags remain present without applying precedence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatFlags {
    /// Prefix nonnegative numbers with a blank.
    pub(crate) space_sign: FormatProperty,
    /// Alternate numeric conversion form.
    pub(crate) alternate_form: FormatProperty,
    /// Prefix nonnegative numbers with a plus sign.
    pub(crate) sign: FormatProperty,
    /// Left adjustment.
    pub(crate) left_adjust: FormatProperty,
    /// Zero padding.
    pub(crate) zero_pad: FormatProperty,
}

/// One parsed conversion field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormatFieldValue {
    /// Its zero-based order among conversions.
    pub ordinal: FormatFieldOrdinal,
    /// Its argument selection.
    pub(crate) argument: FormatArgument,
    /// Its conversion character.
    pub(crate) conversion: FormatCharacter,
    /// Width consumption.
    pub(crate) width: FormatQuantityMode,
    /// Precision consumption.
    pub(crate) precision: FormatQuantityMode,
    /// Parsed flags, without presentation-type precedence or lint policy.
    pub(crate) flags: FormatFlags,
}

/// Compact consumption facts used beside the individual format fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatSummary {
    /// Positional values consumed by conversions and dynamic modifiers.
    pub(crate) positional_consumption: PositionalConsumption,
    /// Whether at least one conversion selects a mapping key.
    pub(crate) mapping_present: FormatProperty,
    /// Whether at least one conversion consumes dynamic width.
    pub(crate) dynamic_width: FormatProperty,
    /// Whether at least one conversion consumes dynamic precision.
    pub(crate) dynamic_precision: FormatProperty,
}

/// A successfully parsed format input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedFormat {
    /// Conversion fields in their source order.
    pub fields: Vec<FormatFieldValue>,
    /// Aggregate argument-consumption properties.
    pub(crate) summary: FormatSummary,
}

/// The owned result of parsing one format input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatParseResult {
    /// Parsing completed successfully.
    Parsed(ParsedFormat),
    /// The parser recognized malformed format text.
    Malformed(FormatParseError),
    /// The selected dialect is not implemented by this parser.
    Unavailable,
}





/// The retained index of percent-format parses.


/// Presence of one parsed flag or consumption property.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FormatProperty {
    #[default]
    Absent,
    Present,
}
impl From<bool> for FormatProperty {
    fn from(value: bool) -> Self {
        if value { Self::Present } else { Self::Absent }
    }
}
impl From<FormatProperty> for bool {
    fn from(value: FormatProperty) -> Self {
        value == FormatProperty::Present
    }
}
impl From<FormatProperty> for usize {
    fn from(value: FormatProperty) -> Self {
        usize::from(bool::from(value))
    }
}
/// Recognition tells the grammar cursor whether to consume a flag character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlagRecognition {
    Consumed,
    Other,
}
/// One decoded Unicode scalar supplied to percent grammar operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatCharacter(pub char);
/// Canonical spelling of a parsed flag combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlagSpelling(pub &'static str);

/// Whether dynamic input was observed for this source occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicInput {
    NotObserved,
    Observed,
}
impl From<bool> for DynamicInput {
    fn from(value: bool) -> Self {
        if value {
            Self::Observed
        } else {
            Self::NotObserved
        }
    }
}
impl From<DynamicInput> for bool {
    fn from(value: DynamicInput) -> Self {
        value == DynamicInput::Observed
    }
}

/// Unicode scalar position in a decoded parser input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatScalarIndex(pub usize);
/// Source traversal order of a parsed conversion field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatFieldOrdinal(pub usize);
/// Number of positional operands consumed by one complete format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PositionalConsumption(pub usize);
impl std::ops::AddAssign<usize> for PositionalConsumption {
    fn add_assign(&mut self, value: usize) {
        self.0 += value;
    }
}
/// The exact spelling of a mapping key in the decoded grammar input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormatMappingKey(pub String);
/// The selected grammar's borrowed input; no decoding or copying is implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatInput<'a>(pub &'a str);
impl std::fmt::Display for FormatCharacter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}
