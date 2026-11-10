//! Plain application-owned format parser values.

/// The grammar used to parse a caller-selected format input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatDialect {
    /// Python printf-style string formatting with a `str` left operand.
    PercentString,
}

/// Whether a candidate format occurrence belongs to the caller's semantic projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatVisibility {
    /// The source occurrence participates in format relations.
    Visible,
    /// The source occurrence is outside the selected semantic syntax.
    Hidden,
}

impl From<bool> for FormatVisibility {
    fn from(visible: bool) -> Self {
        if visible { Self::Visible } else { Self::Hidden }
    }
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
    pub character: Option<FormatCharacter>,
    /// The error position in Unicode scalar units.
    pub index: FormatScalarIndex,
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
    pub space_sign: FormatProperty,
    /// Alternate numeric conversion form.
    pub alternate_form: FormatProperty,
    /// Prefix nonnegative numbers with a plus sign.
    pub sign: FormatProperty,
    /// Left adjustment.
    pub left_adjust: FormatProperty,
    /// Zero padding.
    pub zero_pad: FormatProperty,
}

/// One parsed conversion field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormatFieldValue {
    /// Its zero-based order among conversions.
    pub ordinal: FormatFieldOrdinal,
    /// Its argument selection.
    pub argument: FormatArgument,
    /// Its conversion character.
    pub conversion: FormatCharacter,
    /// Width consumption.
    pub width: FormatQuantityMode,
    /// Precision consumption.
    pub precision: FormatQuantityMode,
    /// Parsed flags, without presentation-type precedence or lint policy.
    pub flags: FormatFlags,
}

/// Compact consumption facts used beside the individual format fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatSummary {
    /// Positional values consumed by conversions and dynamic modifiers.
    pub positional_consumption: PositionalConsumption,
    /// Whether at least one conversion selects a mapping key.
    pub mapping_present: FormatProperty,
    /// Whether at least one conversion consumes dynamic width.
    pub dynamic_width: FormatProperty,
    /// Whether at least one conversion consumes dynamic precision.
    pub dynamic_precision: FormatProperty,
}

/// A successfully parsed format input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedFormat {
    /// Conversion fields in their source order.
    pub fields: Vec<FormatFieldValue>,
    /// Aggregate argument-consumption properties.
    pub summary: FormatSummary,
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

/// A result retained once for one distinct decoded input value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CachedFormat {
    /// The parser's result for this value.
    pub result: FormatParseResult,
}

/// One original string literal and the cached parser value, when it has static decoded text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatOccurrence {
    /// The source-tree node for this literal.
    pub node: ryo::languages::Position,
    /// The result in `FormatParseIndex::values`; absent means unknown or unavailable input.
    pub cached: Option<FormatResultIndex>,
    /// Whether the input was dynamic rather than unavailable parser data.
    pub unknown_input: DynamicInput,
}

/// Current source occurrences and a separate byte-budgeted content memo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetainedFormatIndex<Output, Value, State> {
    /// Budgeted previous-source grammar results; no source-node identities.
    pub memo: crate::domain::grammar_cache::types::GrammarMemo<Output>,
    /// Input text to one cached result, avoiding duplicate grammar work.
    pub by_input: indexmap::IndexMap<FormatCacheInput, FormatResultIndex>,
    /// Parsed or malformed results, one per distinct decoded input.
    pub values: Vec<Value>,
    /// Source occurrences and their cached values.
    pub occurrences: Vec<FormatOccurrence>,
    /// Cache validity for the current source.
    pub state: State,
}

impl<Output, Value, State: Default> Default for RetainedFormatIndex<Output, Value, State> {
    fn default() -> Self {
        Self {
            memo: crate::domain::grammar_cache::types::GrammarMemo::default(),
            by_input: indexmap::IndexMap::new(),
            values: Vec::new(),
            occurrences: Vec::new(),
            state: State::default(),
        }
    }
}

/// The retained index of percent-format parses.

/// Whether the source-local format index is fresh, complete or partial.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FormatParseState {
    /// No format reader has built the index since the last parse.
    #[default]
    Fresh,
    /// Every visible string occurrence and parser result was represented.
    Complete,
    /// A parser input or source occurrence could not be represented.
    Partial,
}

/// Exact parser/dialect identity, supplied by the selected grammar adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GrammarIdentity(pub &'static str);

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

/// One result position in an owning format index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FormatResultIndex(pub usize);
/// Exact grammar input used as a content lookup key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FormatCacheInput(pub String);
impl From<String> for FormatCacheInput {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl std::borrow::Borrow<str> for FormatCacheInput {
    fn borrow(&self) -> &str {
        &self.0
    }
}
impl std::borrow::Borrow<String> for FormatCacheInput {
    fn borrow(&self) -> &String {
        &self.0
    }
}
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

/// The caller-owned assembly buffer for decoded pieces.
pub(crate) struct FormatScratch<'a>(pub &'a mut String);
/// Static decoded input is distinct from a dynamic interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecodedInput {
    Static,
    Dynamic,
}
/// Whether a source node owns an original string occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StringOccurrence {
    String,
    Other,
}
