//! The adapter's traversal and serialization of its external representation: a route record written as JSON.
use super::constants;
use crate::outbound::route_counts::types::{RouteDocument, RouteValue};

/// Writes `value` as JSON at the end of `out`.
pub(super) fn write(out: &mut RouteDocument, value: &RouteValue) {
    match value {
        RouteValue::Absent => out.0.push_str(constants::NULL),
        RouteValue::Flag(flag) => out.0.push_str(if flag.0 {
            constants::TRUE
        } else {
            constants::FALSE
        }),
        RouteValue::Count(count) => out.0.push_str(&count.0.to_string()),
        RouteValue::Tally(tally) => out.0.push_str(&tally.0.to_string()),
        RouteValue::Text(text) => quote(out, text.0.chars()),
        RouteValue::List(items) => {
            out.0.push(constants::OPEN_ARRAY);
            for (position, item) in items.iter().enumerate() {
                if position > 0 {
                    out.0.push(constants::SEPARATOR);
                }
                write(out, item);
            }
            out.0.push(constants::CLOSE_ARRAY);
        }
        RouteValue::Record(fields) => {
            out.0.push(constants::OPEN_OBJECT);
            for (position, field) in fields.iter().enumerate() {
                if position > 0 {
                    out.0.push(constants::SEPARATOR);
                }
                quote(out, field.key.0.chars());
                out.0.push(constants::ASSIGN);
                write(out, &field.value);
            }
            out.0.push(constants::CLOSE_OBJECT);
        }
    }
}

/// Writes `text` as a JSON string, escaping what JSON requires.
fn quote(out: &mut RouteDocument, text: std::str::Chars<'_>) {
    out.0.push(constants::QUOTE);
    for character in text {
        match character {
            constants::QUOTE => out.0.push_str(constants::ESCAPED_QUOTE),
            constants::BACKSLASH => out.0.push_str(constants::ESCAPED_BACKSLASH),
            constants::NEWLINE => out.0.push_str(constants::ESCAPED_NEWLINE),
            constants::RETURN => out.0.push_str(constants::ESCAPED_RETURN),
            constants::TAB => out.0.push_str(constants::ESCAPED_TAB),
            control if control.is_control() => {
                out.0.push_str(constants::ESCAPE_UNICODE);
                for nibble in (0..constants::HEX_DIGITS).rev() {
                    let digit = (u32::from(control) >> (nibble * constants::HEX_NIBBLE_BITS))
                        & constants::HEX_NIBBLE_MASK;
                    out.0.extend(char::from_digit(digit, constants::HEX_RADIX));
                }
            }
            other => out.0.push(other),
        }
    }
    out.0.push(constants::QUOTE);
}
