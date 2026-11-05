//! A standard mini-language observation; custom `__format__` may interpret the same text differently.

/// Recognition of one complete decoded format specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardSpecOutcome {
    /// Every static clause belongs to the target version's standard mini-language.
    Recognized,
    /// Nested replacement fields defer final interpretation until their values are substituted.
    Dynamic,
    /// The first candidate presentation type is absent from the target's standard grammar.
    InvalidType(PresentationCharacter),
    /// Another structural error prevents an invalid-type conclusion.
    OtherError,
}

/// The decoded specification being recognized, without changing its spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatSpecText<'a>(pub &'a str);
/// One observed Unicode presentation character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PresentationCharacter(pub char);
/// Whether nested fields require runtime substitution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NestedFields {
    Absent,
    Present,
}
impl From<bool> for NestedFields {
    fn from(value: bool) -> Self {
        if value { Self::Present } else { Self::Absent }
    }
}
/// One component of the configured language target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TargetVersionPart(pub i64);
/// The complete configured target; tuple conversion keeps the caller's exact version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StandardFormatTarget {
    pub major: TargetVersionPart,
    pub minor: TargetVersionPart,
}
impl From<(i64, i64)> for StandardFormatTarget {
    fn from((major, minor): (i64, i64)) -> Self {
        Self {
            major: TargetVersionPart(major),
            minor: TargetVersionPart(minor),
        }
    }
}
/// The alignment grammar classification, before any formatting policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Alignment {
    Left,
    Right,
    Equal,
    Center,
    Other,
}
impl From<char> for Alignment {
    fn from(value: char) -> Self {
        match value {
            '<' => Self::Left,
            '>' => Self::Right,
            '=' => Self::Equal,
            '^' => Self::Center,
            _ => Self::Other,
        }
    }
}
/// Unicode decimal digits are distinct from other numeric categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecimalDigit {
    Decimal,
    Other,
}
impl From<char> for DecimalDigit {
    fn from(value: char) -> Self {
        use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
        if value.general_category() == GeneralCategory::DecimalNumber {
            Self::Decimal
        } else {
            Self::Other
        }
    }
}
impl std::fmt::Display for PresentationCharacter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}
