//! Ruff JSON decoding and Unicode scalar coordinates to UTF-8 bytes.
use ryo_benchmark::domain::diagnostics::errors::Error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Finding {
    pub code: String,
    pub filename: String,
    pub location: Location,
    pub end_location: Location,
    pub cell: Option<usize>,
}
#[derive(Debug, Deserialize)]
pub(crate) struct Location {
    pub row: usize,
    pub column: usize,
}

pub(crate) fn offset(source: &str, location: &Location) -> Result<usize, Error> {
    if location.row == 0 || location.column == 0 {
        return Err(Error::Invalid("Ruff location is zero-based".into()));
    }
    let bytes = source.as_bytes();
    let mut start = 0;
    for _ in 1..location.row {
        let newline = bytes[start..]
            .iter()
            .position(|byte| matches!(byte, b'\r' | b'\n'))
            .ok_or_else(|| Error::Invalid("Ruff row outside source".into()))?
            + start;
        start = newline + 1;
        if bytes[newline] == b'\r' && bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
    }
    if location.row == 1 && source.starts_with('\u{feff}') {
        start += '\u{feff}'.len_utf8();
    }
    let tail = &source[start..];
    let line = tail.split(['\r', '\n']).next().unwrap_or("");
    let column = location.column - 1;
    let relative = if column == line.chars().count() {
        line.len()
    } else {
        line.char_indices()
            .nth(column)
            .map(|(byte, _)| byte)
            .ok_or_else(|| Error::Invalid("Ruff column outside source".into()))?
    };
    Ok(start + relative)
}
