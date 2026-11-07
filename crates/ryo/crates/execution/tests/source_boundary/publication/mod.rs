//! A consumer clones the leased publication, never its detached raw record.
use super::{
    derived::ROOT,
    fixtures::{Compilation, Expectation, Probe},
};
#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
pub(super) fn controls() -> Vec<Probe> {
    vec![
        Probe {
            compilation: Compilation::Library,
            owner: ROOT,
            source: fixtures::DETACHED_RECORD.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "record",
                second: "private",
            },
        },
        Probe {
            compilation: Compilation::Library,
            owner: ROOT,
            source: fixtures::BORROWED_RECORD.into(),
            expected: Expectation::Refuse {
                code: "E0515",
                first: "publication",
                second: "referencing",
            },
        },
        Probe {
            compilation: Compilation::Library,
            owner: ROOT,
            source: fixtures::LEASED_CLONE.into(),
            expected: Expectation::Accept,
        },
    ]
}
