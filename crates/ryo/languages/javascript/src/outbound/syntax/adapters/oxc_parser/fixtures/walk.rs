//! Complete `ESTree` trees pinned from Acorn, independently of Oxc and the adapter.
#![expect(
    clippy::expect_used,
    reason = "malformed or missing independent reference evidence must fail the fixture rather than silently passing"
)]
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use ryo::languages::{
    Offset,
    contract::{Leafness, ReferenceRefusal, Walked},
};

/// The reference parser's walk of every pinned input, one line for each element, which the adapter's tree is compared with.
const OUTPUT: &str = include_str!("../../../../../../tests/fixtures/standard/output.txt");

/// Find the reference records for these exact source bytes.
pub(crate) fn records(source: &str) -> Vec<&'static str> {
    let hash = source
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |value, byte| {
            (value ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    let header = format!("file {hash:x} {}", source.len());
    let refused = format!("{header} refused");
    let mut lines = OUTPUT
        .lines()
        .skip_while(|line| *line != header && *line != refused);
    let first = lines
        .next()
        .expect("every tested source must have independently pinned reference evidence");
    std::iter::once(first)
        .chain(lines.take_while(|line| !line.starts_with("file ")))
        .collect()
}

/// Read the independently pinned tree, distinguishing standard refusal from absent evidence.
///
/// # Panics
/// Missing evidence or malformed reference records fail the test.
pub(crate) fn reference(source: &str) -> Result<Walked, ReferenceRefusal> {
    let lines = records(source);
    if lines.first().is_some_and(|line| line.ends_with(" refused")) {
        return Err(ReferenceRefusal);
    }
    let mut stack: Vec<Walked> = Vec::new();
    for record in lines.iter().filter_map(|line| line.strip_prefix("tree ")) {
        let [depth, leafness, named, start, end]: [&str; 5] = record
            .split_whitespace()
            .collect::<Vec<_>>()
            .try_into()
            .expect("reference record has five fields");
        let depth: usize = depth.parse().expect("reference depth is numeric");
        while stack.len() > depth {
            let child = stack.pop().expect("open child");
            stack
                .last_mut()
                .expect("exactly one reference root")
                .children
                .push(child);
        }
        assert_eq!(stack.len(), depth, "reference depth cannot skip a parent");
        let kind = kinds()
            .find(|kind| kind_name(*kind).0 == named)
            .expect("every reference kind is mapped");
        stack.push(Walked {
            kind: kind_bits(kind),
            leaf: match leafness {
                "N" => Some(Leafness::Node),
                "T" => Some(Leafness::Token),
                _ => None,
            }
            .expect("unknown reference leafness"),
            start: Offset(start.parse().expect("byte offset")),
            end: Offset(end.parse().expect("byte offset")),
            children: Vec::new(),
        });
    }
    while stack.len() > 1 {
        let child = stack.pop().expect("open child");
        stack
            .last_mut()
            .expect("root remains open")
            .children
            .push(child);
    }
    Ok(stack.pop().expect("accepted source has a reference root"))
}
/// The complete accepted standard tree.
#[must_use]
pub(crate) fn walked(source: &str) -> Walked {
    reference(source).expect("reference refused this input")
}
