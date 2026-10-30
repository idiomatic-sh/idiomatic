//! The pinned typescript-estree tree, independent of the runtime parser and its projection.

#![expect(
    clippy::expect_used,
    reason = "Malformed or missing pinned reference evidence is a test failure, never a parser refusal"
)]

use ryo::languages::Offset;
use ryo::languages::contract::{Leafness, Walked};

use crate::domain::syntax::service::{kind_bits, kind_name, kinds};

/// The reference parser's walk of every pinned input, one line for each element, which the adapter's tree is compared with.
const OUTPUT: &str = include_str!("../../../../../../tests/fixtures/standard/output.txt");

/// The reference parser's disposition of a pinned input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Acceptance {
    /// The reference produced an AST.
    Accepted,
    /// The reference threw a syntax error and returned no AST.
    Refused,
    /// The pinned inputs do not contain these bytes.
    Unknown,
}

/// The pinned lines for this exact byte sequence, including its file header.
pub(crate) fn block(text: &str) -> Vec<&'static str> {
    let fingerprint = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |state, byte| {
        (state ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    let wanted = format!("file {fingerprint:x} {}", text.len());
    let rejected = format!("{wanted} refused");
    let mut records = OUTPUT
        .lines()
        .skip_while(|line| *line != wanted && *line != rejected);
    let Some(header) = records.next() else {
        return Vec::new();
    };
    std::iter::once(header)
        .chain(records.take_while(|line| !line.starts_with("file ")))
        .collect()
}

/// Whether this exact source was accepted by the independently executed reference.
#[must_use]
pub(crate) fn acceptance(text: &str) -> Acceptance {
    match block(text).first() {
        Some(header) if header.ends_with(" refused") => Acceptance::Refused,
        Some(_) => Acceptance::Accepted,
        None => Acceptance::Unknown,
    }
}

/// The complete standard tree for a pinned input.
///
/// # Panics
///
/// Missing inputs, malformed records and unmapped standard kinds fail the conformance evidence.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    assert_eq!(
        acceptance(text),
        Acceptance::Accepted,
        "reference has no AST for this input; regenerate standard/output.txt or test a refused input as a refusal"
    );
    parsed(&block(text).join("\n"))
}

/// Decode a reference tree while checking its depths, vocabulary and byte ranges.
///
/// # Panics
///
/// An invalid reference dump is a fixture failure.
#[must_use]
pub(crate) fn parsed(records: &str) -> Walked {
    let mut stack: Vec<Walked> = Vec::new();
    for line in records
        .lines()
        .filter_map(|line| line.strip_prefix("tree "))
    {
        let [depth, leafness, named, start, end]: [&str; 5] = line
            .split_whitespace()
            .collect::<Vec<_>>()
            .try_into()
            .expect("malformed reference tree record");
        let depth: usize = depth.parse().expect("reference depth is an integer");
        while stack.len() > depth {
            let completed = stack.pop().expect("an open node exists");
            stack
                .last_mut()
                .expect("a tree has exactly one root")
                .children
                .push(completed);
        }
        assert_eq!(stack.len(), depth, "reference depth skips a parent");
        let mapped = kinds()
            .find(|kind| kind_name(*kind).0 == named)
            .expect("every standard kind has a vocabulary entry");
        let leaf = match leafness {
            "N" => Some(Leafness::Node),
            "T" => Some(Leafness::Token),
            _ => None,
        }
        .expect("reference element is neither a node nor a token");
        let start = Offset(start.parse().expect("reference start is a byte offset"));
        let end = Offset(end.parse().expect("reference end is a byte offset"));
        assert!(start.0 <= end.0, "reference ranges are ordered");
        stack.push(Walked {
            kind: kind_bits(mapped),
            leaf,
            start,
            end,
            children: Vec::new(),
        });
    }
    while stack.len() > 1 {
        let completed = stack.pop().expect("an open node exists");
        stack
            .last_mut()
            .expect("the root remains open")
            .children
            .push(completed);
    }
    stack.pop().expect("the reference has a root")
}

/// A reference rejection is explicit; missing evidence still fails instead of becoming a rejection.
///
/// # Panics
///
/// The source has no pinned evidence or its reference tree is malformed.
pub(crate) fn reference(text: &str) -> Result<Walked, ryo::languages::contract::ReferenceRefusal> {
    match acceptance(text) {
        Acceptance::Refused => Err(ryo::languages::contract::ReferenceRefusal),
        Acceptance::Accepted | Acceptance::Unknown => Ok(walked(text)),
    }
}
