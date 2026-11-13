//! Pure standard percent-format flag operations; no presentation policy.
use super::types::{FlagRecognition, FlagSpelling, FormatCharacter, FormatFlags, FormatProperty};

/// Retain one recognized grammar flag; false leaves the cursor at the next grammar component.
pub(crate) fn insert_flag(flags: &mut FormatFlags, character: FormatCharacter) -> FlagRecognition {
    match character.0 {
        ' ' => flags.space_sign = FormatProperty::Present,
        '#' => flags.alternate_form = FormatProperty::Present,
        '+' => flags.sign = FormatProperty::Present,
        '-' => flags.left_adjust = FormatProperty::Present,
        '0' => flags.zero_pad = FormatProperty::Present,
        _ => return FlagRecognition::Other,
    }
    FlagRecognition::Consumed
}

/// Unique flag characters in ASCII order, with no allocation or semantic simplification.
pub(crate) fn flag_text(flags: FormatFlags) -> FlagSpelling {
    let index = usize::from(flags.space_sign)
        | (usize::from(flags.alternate_form) << 1)
        | (usize::from(flags.sign) << 2)
        | (usize::from(flags.left_adjust) << 3)
        | (usize::from(flags.zero_pad) << 4);
    FlagSpelling(super::constants::FLAG_SPELLINGS[index])
}
