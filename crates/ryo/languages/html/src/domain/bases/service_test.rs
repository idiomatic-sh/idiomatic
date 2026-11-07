//! The `bases` service's cases: a URL of a document is resolved against the base the document names, else against the file.

use ryo::languages::{SourcePath, WrittenTarget};

use super::service::{based, resolved};
use super::types::Whose;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The `href` of the document's `<base>`, if it has one; the URL written in the document, and whose it is.
type Input = (Option<&'static str>, &'static str, Whose);
/// What the URL names.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases, for a document in `docs/a.html`.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("with_no_base_a_url_is_resolved_against_the_file"),
        word: Word::Succeeds,
        input: || (None, "x.html", Whose::Other),
        expect: Expect::Value("docs/x.html"),
    },
    Case {
        name: CaseName("a_url_is_resolved_against_the_directory_a_base_names"),
        word: Word::Succeeds,
        input: || (Some("/assets/"), "x.html", Whose::Other),
        expect: Expect::Value("assets/x.html"),
    },
    Case {
        name: CaseName("a_base_that_is_a_file_is_the_directory_it_is_in"),
        word: Word::Succeeds,
        input: || (Some("/assets/index.html"), "x.html", Whose::Other),
        expect: Expect::Value("assets/x.html"),
    },
    Case {
        name: CaseName("a_base_with_a_query_is_its_directory"),
        word: Word::Succeeds,
        input: || (Some("/assets/?v=1"), "x.html", Whose::Other),
        expect: Expect::Value("assets/x.html"),
    },
    Case {
        name: CaseName("a_base_with_a_fragment_is_its_directory"),
        word: Word::Succeeds,
        input: || (Some("/assets/#top"), "x.html", Whose::Other),
        expect: Expect::Value("assets/x.html"),
    },
    Case {
        name: CaseName("a_base_file_with_a_query_is_the_directory_it_is_in"),
        word: Word::Succeeds,
        input: || (Some("/assets/index.html?v=1"), "x.html", Whose::Other),
        expect: Expect::Value("assets/x.html"),
    },
    Case {
        name: CaseName("an_empty_base_is_the_file_itself"),
        word: Word::Succeeds,
        input: || (Some(""), "x.html", Whose::Other),
        expect: Expect::Value("docs/x.html"),
    },
    Case {
        name: CaseName("a_base_of_the_root_is_the_root"),
        word: Word::Succeeds,
        input: || (Some("/"), "x.html", Whose::Other),
        expect: Expect::Value("x.html"),
    },
    Case {
        name: CaseName("a_base_beside_the_file_is_resolved_against_the_file_first"),
        word: Word::Succeeds,
        input: || (Some("sub/"), "x.html", Whose::Other),
        expect: Expect::Value("docs/sub/x.html"),
    },
    Case {
        name: CaseName("a_fragment_alone_is_the_file_whatever_the_base"),
        word: Word::Succeeds,
        input: || (Some("/assets/"), "#top", Whose::Other),
        expect: Expect::Value("docs/a.html#top"),
    },
    Case {
        name: CaseName("the_base_elements_own_href_is_resolved_against_the_file"),
        word: Word::Succeeds,
        input: || (Some("/assets/"), "/assets/", Whose::Base),
        expect: Expect::Value("assets"),
    },
    Case {
        name: CaseName("a_url_under_a_base_outside_the_run_leaves_the_run_as_written"),
        word: Word::Succeeds,
        input: || (Some("https://cdn.example/"), "x.html", Whose::Other),
        expect: Expect::Value("x.html"),
    },
];

fn subject((base, written, whose): Input) -> Value {
    let at = based(SourcePath("docs/a.html"), base.map(WrittenTarget));
    Box::leak(
        resolved(&at, WrittenTarget(written), whose)
            .0
            .into_boxed_str(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
