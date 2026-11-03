//! Pure standard percent-format flag operations; no presentation policy.
use super::types::FormatFlags;

/// Retain one recognized grammar flag; false leaves the cursor at the next grammar component.
pub(crate) fn insert_flag(flags: &mut FormatFlags, character: char) -> bool {
    match character {
        ' ' => flags.space_sign = true,
        '#' => flags.alternate_form = true,
        '+' => flags.sign = true,
        '-' => flags.left_adjust = true,
        '0' => flags.zero_pad = true,
        _ => return false,
    }
    true
}

/// Unique flag characters in ASCII order, with no allocation or semantic simplification.
pub(crate) fn flag_text(flags: FormatFlags) -> &'static str {
    const SPELLINGS: [&str; 32] = [
        "", " ", "#", " #", "+", " +", "#+", " #+", "-", " -", "#-", " #-", "+-", " +-", "#+-",
        " #+-", "0", " 0", "#0", " #0", "+0", " +0", "#+0", " #+0", "-0", " -0", "#-0", " #-0",
        "+-0", " +-0", "#+-0", " #+-0",
    ];
    let index = usize::from(flags.space_sign)
        | (usize::from(flags.alternate_form) << 1)
        | (usize::from(flags.sign) << 2)
        | (usize::from(flags.left_adjust) << 3)
        | (usize::from(flags.zero_pad) << 4);
    SPELLINGS[index]
}
