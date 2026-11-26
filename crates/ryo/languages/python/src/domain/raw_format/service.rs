//! Parse preserved logical token spellings without decoding or concatenating bodies.
use super::types::{RawFormatBody, RawFormatToken};


/// Check the ordinary string-token envelope and borrow its raw body unchanged.
pub(crate) fn body(token: RawFormatToken<'_>) -> Option<RawFormatBody<'_>> {
    let token = token.0;
    let bytes = token.as_bytes();
    let prefix = usize::from(matches!(bytes.first(), Some(b'r' | b'R' | b'u' | b'U')));
    let quoted = token.get(prefix..)?;
    let quote = *quoted.as_bytes().first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let width = if quoted
        .as_bytes()
        .get(..3)
        .is_some_and(|first| first.iter().all(|byte| *byte == quote))
    {
        3
    } else {
        1
    };
    let end = quoted.len().checked_sub(width)?;
    if end < width
        || !quoted
            .as_bytes()
            .get(end..)?
            .iter()
            .all(|byte| *byte == quote)
    {
        return None;
    }
    quoted.get(width..end).map(RawFormatBody)
}
