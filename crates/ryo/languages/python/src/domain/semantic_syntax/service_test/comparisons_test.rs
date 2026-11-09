//! Visibility costs one bisection per occurrence and per embedded expression, never a scan.
//!
//! The replaced scan compared every embedded expression for every occurrence and every projected
//! expression: on the 1024 x 1024 case at least 1024 * 1024 / 2 comparisons, over its bound.
use super::super::service::{enclosing, project};
use super::super::types::{TypeContext, Visibility};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::{EmbeddedMapping, EmbeddedSource, Position};

/// Occurrences looked up, and synthetic embedded expressions projected.
#[derive(Clone, Copy, Debug)]
struct Input {
    /// N: occurrences spread evenly over native and embedded elements.
    rows: u32,
    /// M: embedded expressions, every odd one nested in its predecessor.
    spans: u32,
}
/// Whether the comparisons stayed within (N + M)(log2(M + 1) + 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cost {
    /// Within the bound.
    Logarithmic,
    /// Over the bound.
    Exceeded,
}
/// An occurrence answered differently from the expected projection.
type Failure = Position;

/// Native elements end at this position; embedded expression `i` spans `NATIVE + 4i ..+ 3`.
/// An even expression's host is native element `i / 2`; an odd one's lies in its predecessor.
const NATIVE: u32 = 1000;

const CASES: &[Case<Input, Cost, Failure>] = &[
    Case {
        name: CaseName("one occurrence over one embedded expression"),
        word: Word::Succeeds,
        input: || Input { rows: 1, spans: 1 },
        expect: Expect::Value(Cost::Logarithmic),
    },
    Case {
        name: CaseName(
            "occurrences and nested embedded expressions cost (N + M) log M comparisons, not N x M",
        ),
        word: Word::Succeeds,
        input: || Input {
            rows: 1024,
            spans: 1024,
        },
        expect: Expect::Value(Cost::Logarithmic),
    },
];

/// Project the synthetic expressions, then look up every occurrence, counting each interval read.
fn subject(Input { rows, spans }: Input) -> Result<Cost, Failure> {
    let sources = (0..spans).map(|index| {
        let root = NATIVE + 4 * index;
        EmbeddedSource {
            host: if index % 2 == 1 {
                Position(root - 3)
            } else {
                Position(index / 2)
            },
            root: Position(root),
            end: Position(root + 3),
            mapping: EmbeddedMapping::WholeHost,
        }
    });
    let mut count = 0_u64;
    let mut read = |source: &EmbeddedSource| {
        count += 1;
        source.root..source.end
    };
    let mut hidden = Vec::new();
    // A native host at 2 mod 4 is a value, so its nested successor is hidden through its holder.
    project(sources, &mut hidden, &mut read, |host| {
        if host.0 < NATIVE && host.0 % 4 == 2 {
            TypeContext::Value
        } else {
            TypeContext::Type
        }
    });
    let last = u64::from(NATIVE + 4 * spans);
    for row in 0..u64::from(rows) {
        let at = Position(u32::try_from(row * last / u64::from(rows)).unwrap_or(u32::MAX));
        let hidden_holder =
            at.0.checked_sub(NATIVE)
                .filter(|offset| offset % 4 < 3)
                .map(|offset| offset / 4)
                .map(|index| if index % 2 == 1 { index - 1 } else { index })
                .is_some_and(|holder| (holder / 2) % 4 == 2);
        let expected = if hidden_holder {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        let found = if enclosing(&hidden, at, &mut read).is_some() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if found != expected {
            return Err(at);
        }
    }
    let depth = u64::from((spans + 1).next_power_of_two().trailing_zeros());
    let bound = (u64::from(rows) + u64::from(spans)) * (depth + 2);
    Ok(if count <= bound {
        Cost::Logarithmic
    } else {
        Cost::Exceeded
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
