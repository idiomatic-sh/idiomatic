//! The `model` service's cases: the sections the headings of a file open, and what holds an element.

use super::service::{holding, owner, settle};
use super::types::Sections;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::parsed;
use ryo::languages;

/// A document.
type Input = &'static str;
/// The sections it opens, one for each heading, as `heading@parent..end`, and what holds its last paragraph.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_deeper_heading_is_held_by_the_section_above_it_and_ends_at_the_next_as_shallow",
        ),
        word: Word::Succeeds,
        input: || "# a\n\n## b\n\nx\n\n## c\n\n# d\n",
        expect: Expect::Value("@2<@0..20 @8<@2..14 @18<@2..20 @24<@0..24"),
    },
    Case {
        name: CaseName("a_document_with_no_heading_opens_no_section"),
        word: Word::Succeeds,
        input: || "x\n",
        expect: Expect::Value(""),
    },
];

fn subject(source: Input) -> Value {
    let mut parsed = parsed(source.as_bytes()).expect("the source is read");
    let tree = languages::Tree {
        elements: &parsed.elements,
        source: languages::ByteSlice(parsed.bytes),
        file: languages::FileId(0),
    };
    let mut sections = Sections::default();
    settle(&tree, &mut sections);
    let _ = (&mut parsed.symbols, holding, owner);
    let shown: Vec<String> = sections
        .0
        .iter()
        .map(|section| {
            format!(
                "@{}<@{}..{}",
                section.heading.0, section.parent.0, section.end.0
            )
        })
        .collect();
    Box::leak(shown.join(" ").into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
