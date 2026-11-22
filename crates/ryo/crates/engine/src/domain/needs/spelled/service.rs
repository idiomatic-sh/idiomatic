//! The services of the `spelled` part: a text asked numbered, and the file's tokens read once for the texts asked.

use super::types::{SpelledFile, SpelledTexts};
use ryo_language::domain::node_table::service::{
    element_count, embedded_segments, node_count, own_end, span_of, subtree_end_of,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::types::Position;

/// The number of a text asked, given the first time it is met.
pub(crate) fn text_number(texts: &mut SpelledTexts, text: &str) -> u32 {
    if let Some(number) = texts.numbers.get(text) {
        return *number;
    }
    let number = u32::try_from(texts.numbers.len()).unwrap_or(u32::MAX);
    let _was = texts.numbers.insert(text.into(), number);
    let (shortest, longest) = texts.lengths;
    texts.lengths = if number == 0 {
        (text.len(), text.len())
    } else {
        (shortest.min(text.len()), longest.max(text.len()))
    };
    number
}

/// Sets the bit of the text a token spells, when it is a text asked.
fn token_read(
    texts: &SpelledTexts,
    source: &str,
    nodes: &NodeTable,
    at: Position,
    bits: &mut [u64],
) -> bool {
    let span = span_of(nodes, at);
    let (start, end) = (
        usize::try_from(span.start.0).unwrap_or(usize::MAX),
        usize::try_from(span.end.0).unwrap_or(usize::MAX),
    );
    let Some(text) = source.get(start..end) else {
        return false;
    };
    if !text.is_ascii() {
        // A token past ASCII may be a name the language reads as another text (a normalised one): unless it holds what
        // no name holds (a quote or a space: a string's or a text's token), the file is not sure to spell what it reads.
        return !text
            .bytes()
            .any(|byte| byte == b'"' || byte == b'\'' || byte.is_ascii_whitespace());
    }
    let (shortest, longest) = texts.lengths;
    if text.len() < shortest || text.len() > longest {
        return false;
    }
    if let Some(number) = texts.numbers.get(text)
        && let Some(word) = bits.get_mut(usize::try_from(*number / 64).unwrap_or(usize::MAX))
    {
        *word |= 1_u64 << (number % 64);
    }
    false
}

/// Reads the file's tokens once for the texts asked: the own tree's (its elements past the nodes), then each embedded
/// segment's (past its own nodes). The text of a token is its exact source text, as the node table's `Text` column holds
/// it.
pub(crate) fn file_read(
    texts: &SpelledTexts,
    source: Option<&str>,
    nodes: &NodeTable,
    file: &mut SpelledFile,
) {
    let words = texts.numbers.len().div_ceil(64);
    file.own.clear();
    file.own.resize(words, 0);
    file.every.clear();
    file.every.resize(words, 0);
    file.known = texts.numbers.len();
    file.unsure = false;
    let Some(source) = source else {
        // A file with no source spells nothing that can be read: every text is taken to be there.
        file.unsure = true;
        return;
    };
    let own = own_end(nodes);
    let first = u32::try_from(node_count(nodes).0).unwrap_or(u32::MAX);
    for at in first..own.0 {
        file.unsure |= token_read(texts, source, nodes, Position(at), &mut file.own);
    }
    file.every.copy_from_slice(&file.own);
    let all = u32::try_from(element_count(nodes).0).unwrap_or(u32::MAX);
    for segment in embedded_segments(nodes) {
        let tokens = subtree_end_of(nodes, segment.root).0;
        for at in tokens..segment.end.0.min(all) {
            file.unsure |= token_read(texts, source, nodes, Position(at), &mut file.every);
        }
    }
}

/// Whether the file spells the text numbered `number`, in its own tree (`own`) or anywhere.
pub(crate) fn spelled(file: &SpelledFile, number: u32, own: bool) -> bool {
    if file.unsure {
        return true;
    }
    let bits = if own { &file.own } else { &file.every };
    bits.get(usize::try_from(number / 64).unwrap_or(usize::MAX))
        .is_some_and(|word| word & (1_u64 << (number % 64)) != 0)
}
