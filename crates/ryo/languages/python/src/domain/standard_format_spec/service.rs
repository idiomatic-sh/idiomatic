//! Finite recognition of static standard format specs for Python language facts.
use super::types::StandardSpecOutcome;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

fn align(character: char) -> bool {
    matches!(character, '<' | '>' | '=' | '^')
}

fn digit(character: &char) -> bool {
    character.general_category() == GeneralCategory::DecimalNumber
}

/// Observe the standard mini-language only after the replacement-field grammar established
/// whether nested fields are present. A custom value type may accept any text independently.
#[must_use]
pub(crate) fn recognize(
    raw: &str,
    has_nested_fields: bool,
    target: (i64, i64),
) -> StandardSpecOutcome {
    if has_nested_fields {
        return StandardSpecOutcome::Dynamic;
    }
    let mut chars = raw.chars().peekable();
    let mut lookahead = chars.clone();
    let first = lookahead.next();
    let second = lookahead.next();
    if second.is_some_and(align) {
        let _fill = chars.next();
        let _alignment = chars.next();
    } else if first.is_some_and(align) {
        let _alignment = chars.next();
    }
    if chars
        .peek()
        .is_some_and(|character| matches!(*character, '+' | '-' | ' '))
    {
        let _sign = chars.next();
    }
    // Python 3.11 introduced sign-aware zero coercion as the `z` option.
    if target >= (3, 11) && chars.peek() == Some(&'z') {
        let _zero_coercion = chars.next();
    }
    if chars.peek() == Some(&'#') {
        let _alternate = chars.next();
    }
    if chars.peek() == Some(&'0') {
        let _zero = chars.next();
    }
    while chars.peek().is_some_and(digit) {
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
        if !chars.peek().is_some_and(digit) {
            return StandardSpecOutcome::OtherError;
        }
        while chars.peek().is_some_and(digit) {
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
        return StandardSpecOutcome::InvalidType(presentation);
    }
    if chars.next().is_some() {
        StandardSpecOutcome::OtherError
    } else {
        StandardSpecOutcome::Recognized
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
