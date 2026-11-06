//! Render a canonical document and take its digest, with the sorted-object encoding of the reference harness.
use super::constants;
use super::types::{Document, Fragment, Label, Quantity};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use sha2::Digest as _;

fn quote(label: &Label) -> Document {
    let mut out = String::from(constants::QUOTE);
    for ch in label.0.chars() {
        match ch {
            '"' => out.push_str(constants::ESCAPED_QUOTE),
            '\\' => out.push_str(constants::ESCAPED_BACKSLASH),
            '\n' => out.push_str(constants::ESCAPED_NEWLINE),
            '\r' => out.push_str(constants::ESCAPED_RETURN),
            '\t' => out.push_str(constants::ESCAPED_TAB),
            '\u{8}' => out.push_str(constants::ESCAPED_BACKSPACE),
            '\u{c}' => out.push_str(constants::ESCAPED_FORM_FEED),
            control if control < '\u{20}' => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push_str(constants::QUOTE);
    Document(out)
}

/// The compact text of a document: sorted keys, no padding, characters beyond ASCII unescaped.
pub(crate) fn render(fragment: &Fragment) -> Document {
    Document(match fragment {
        Fragment::Null => constants::NULL.to_owned(),
        Fragment::Number(Quantity(number)) => number.to_string(),
        Fragment::Text(label) => quote(label).0,
        Fragment::List(items) => format!(
            "{}{}{}",
            constants::OPEN_LIST,
            items
                .iter()
                .map(|item| render(item).0)
                .collect::<Vec<_>>()
                .join(constants::COMMA),
            constants::CLOSE_LIST
        ),
        Fragment::Object(entries) => format!(
            "{}{}{}",
            constants::OPEN_OBJECT,
            entries
                .iter()
                .map(|(name, value)| format!(
                    "{}{}{}",
                    quote(name).0,
                    constants::COLON,
                    render(value).0
                ))
                .collect::<Vec<_>>()
                .join(constants::COMMA),
            constants::CLOSE_OBJECT
        ),
    })
}

/// The unframed SHA256 of exact text.
pub(crate) fn text_digest(text: &Document) -> ArtifactDigest {
    ArtifactDigest(format!("{:x}", sha2::Sha256::digest(text.0.as_bytes())))
}

/// The digest of a document's compact text.
pub(crate) fn digest(fragment: &Fragment) -> ArtifactDigest {
    text_digest(&render(fragment))
}
