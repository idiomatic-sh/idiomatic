//! Finite recognition of static standard format specs for Python language facts.
use super::types::{
    Alignment, DecimalDigit, FormatSpecText, NestedFields, PresentationCharacter,
    StandardFormatTarget, StandardSpecOutcome,
};
/// Observe the standard mini-language only after the replacement-field grammar established
/// whether nested fields are present. A custom value type may accept any text independently.
#[must_use]
pub(crate) fn recognize(
    raw: FormatSpecText<'_>,
    nested: NestedFields,
    target: StandardFormatTarget,
) -> StandardSpecOutcome {
    if nested == NestedFields::Present {
        return StandardSpecOutcome::Dynamic;
    }
    let mut chars = raw.0.chars().peekable();
    let mut lookahead = chars.clone();
    let first = lookahead.next();
    let second = lookahead.next();
    if second.is_some_and(|character| Alignment::from(character) != Alignment::Other) {
        let _fill = chars.next();
        let _alignment = chars.next();
    } else if first.is_some_and(|character| Alignment::from(character) != Alignment::Other) {
        let _alignment = chars.next();
    }
    if chars
        .peek()
        .is_some_and(|character| matches!(*character, '+' | '-' | ' '))
    {
        let _sign = chars.next();
    }
    // Python 3.11 introduced sign-aware zero coercion as the `z` option.
    if target >= super::constants::ZERO_COERCION_TARGET && chars.peek() == Some(&'z') {
        let _zero_coercion = chars.next();
    }
    if chars.peek() == Some(&'#') {
        let _alternate = chars.next();
    }
    if chars.peek() == Some(&'0') {
        let _zero = chars.next();
    }
    while chars
        .peek()
        .is_some_and(|character| DecimalDigit::from(*character) == DecimalDigit::Decimal)
    {
        let _width = chars.next();
    }
    if chars
        .peek()
        .is_some_and(|character| matches!(*character, ',' | '_'))
    {
        let _grouping = chars.next();
    }
    if chars.peek() == Some(&'.') {
        let _dot = chars.next();
        if !chars
            .peek()
            .is_some_and(|character| DecimalDigit::from(*character) == DecimalDigit::Decimal)
        {
            return StandardSpecOutcome::OtherError;
        }
        while chars
            .peek()
            .is_some_and(|character| DecimalDigit::from(*character) == DecimalDigit::Decimal)
        {
            let _precision = chars.next();
        }
    }
    let Some(presentation) = chars.next() else {
        return StandardSpecOutcome::Recognized;
    };
    if !matches!(
        presentation,
        's' | 'b' | 'c' | 'd' | 'o' | 'x' | 'X' | 'n' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%'
    ) {
        return StandardSpecOutcome::InvalidType(PresentationCharacter(presentation));
    }
    if chars.next().is_some() {
        StandardSpecOutcome::OtherError
    } else {
        StandardSpecOutcome::Recognized
    }
}
