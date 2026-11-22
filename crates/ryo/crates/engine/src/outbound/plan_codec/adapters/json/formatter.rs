//! How the plan's JSON is laid out: as Python's `json.dump` lays it out with its defaults, so the plan of record and this
//! adapter's plan are the same bytes.

use std::io;

/// `", "` between items, `": "` after a key, and every character past ASCII written as `\uXXXX` (two for a character
/// past the basic plane).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PythonFormatter;

impl serde_json::ser::Formatter for PythonFormatter {
    fn begin_array_value<W: ?Sized + io::Write>(&mut self, writer: &mut W, first: bool) -> io::Result<()> {
        if first { Ok(()) } else { writer.write_all(b", ") }
    }

    fn begin_object_key<W: ?Sized + io::Write>(&mut self, writer: &mut W, first: bool) -> io::Result<()> {
        if first { Ok(()) } else { writer.write_all(b", ") }
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }

    fn write_string_fragment<W: ?Sized + io::Write>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()> {
        for piece in fragment.split_inclusive(|character: char| !character.is_ascii()) {
            let (ascii, last) = match piece.char_indices().last() {
                Some((at, character)) if !character.is_ascii() => (piece.get(..at).unwrap_or_default(), Some(character)),
                Some(_) | None => (piece, None),
            };
            writer.write_all(ascii.as_bytes())?;
            if let Some(character) = last {
                let mut units = [0_u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    write!(writer, "\\u{unit:04x}")?;
                }
            }
        }
        Ok(())
    }
}
