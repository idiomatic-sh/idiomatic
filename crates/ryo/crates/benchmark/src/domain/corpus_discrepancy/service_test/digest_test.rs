//! A corpus is identified by its file list and, when sealed, its context entries: links by their path and exact target
//! text; never its location.
use super::super::service::corpus_digest;
use super::{files, manifest, sealed};
use crate::cases::{Case, CaseName, Expect, Word, run};
use benchmark_contracts::domain::context::types::LinkTarget;
use benchmark_contracts::domain::requests::types::ContextSeal;

/// The corpus of the fixture file list: unsealed, sealed under a boundary, or sealed with these links (path, target).
enum Check {
    Unsealed,
    Sealed(ContextSeal),
    Linked(&'static [(&'static str, &'static str)]),
}
type Input = Check;
/// The corpus digest.
type Value = &'static str;
/// A digest has no refusal.
type Failure = ();

/// The digests of the fixture corpus as `json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False)` spells
/// its identity document. The linked ones come from an independent Python rendering that also reproduces SEALED and
/// UNSEALED: a `links` object of each link's path to its exact target text, present only when non-empty.
const SEALED: &str = "97cb4ed58227a9bbc5a452fef45492814061f5def35c80060e964fdc681060bb";
const UNSEALED: &str = "795d8c6afdcdeb9b0749f36eaa3a67d610244230fe1eb6d14f605e30ce764486";
const LINKED: &str = "f1b0622bc85b3149ebe480de52b13d5b5d94e9f283919ac4806b1fec9b474180";
const OTHER_PATH: &str = "5a9339c6204d25d7810578f224bfb17108c660ab29c826615fd14c951ca5d391";
const OTHER_TARGET: &str = "d3b272ad564ce65165ac661488feebd3c4d214d3ffd63fabbee93fecc0dff9b0";
const EQUIVALENT_SPELLING: &str =
    "452193059924a03220bd00d205370913bb07194e8528cb94fb2de8b615a0e11b";
const TWO_LINKS: &str = "8493d0b1d008cdef50b7d6483c575ad70d7dba348ba1a4069b4d74794e0a4ca5";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a sealed corpus digest is its files and context entries, not its location"),
        word: Word::Succeeds,
        input: || Check::Sealed(sealed("/elsewhere")),
        expect: Expect::Value(SEALED),
    },
    Case {
        name: CaseName("an unsealed corpus is its file list alone"),
        word: Word::Succeeds,
        input: || Check::Unsealed,
        expect: Expect::Value(UNSEALED),
    },
    Case {
        name: CaseName("a sealed link enters the corpus digest by its path and exact target text"),
        word: Word::Succeeds,
        input: || Check::Linked(&[("docs/café.png", "../setup.cfg")]),
        expect: Expect::Value(LINKED),
    },
    Case {
        name: CaseName("a link at another path is another corpus"),
        word: Word::Succeeds,
        input: || Check::Linked(&[("docs/other.png", "../setup.cfg")]),
        expect: Expect::Value(OTHER_PATH),
    },
    Case {
        name: CaseName("a link to another target is another corpus"),
        word: Word::Succeeds,
        input: || Check::Linked(&[("docs/café.png", "../café.py")]),
        expect: Expect::Value(OTHER_TARGET),
    },
    Case {
        name: CaseName("an equivalent spelling of the target is another corpus"),
        word: Word::Succeeds,
        input: || Check::Linked(&[("docs/café.png", "./../setup.cfg")]),
        expect: Expect::Value(EQUIVALENT_SPELLING),
    },
    Case {
        name: CaseName("an additional link is another corpus"),
        word: Word::Succeeds,
        input: || {
            Check::Linked(&[
                ("docs/café.png", "../setup.cfg"),
                ("docs/two.png", "../setup.cfg"),
            ])
        },
        expect: Expect::Value(TWO_LINKS),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject"
)]
fn subject(input: Input) -> Result<Value, Failure> {
    let context = match input {
        Check::Unsealed => ContextSeal::Unsealed,
        Check::Sealed(context) => context,
        Check::Linked(links) => {
            let mut manifest = manifest("/corpus");
            manifest.links = links
                .iter()
                .map(|(path, target)| ((*path).into(), LinkTarget((*target).into())))
                .collect();
            ContextSeal::Sealed(manifest)
        }
    };
    Ok(Box::leak(
        corpus_digest(&files(), &context).0.into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
