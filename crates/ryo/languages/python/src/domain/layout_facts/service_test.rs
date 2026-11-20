//! The `layout_facts` service's cases: the links between the tokens of a source, its indentation regions and its lines.

use super::service::{indent_regions, line_place, line_starts, token_links};
use super::types::FileLines;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::service::written;
use crate::domain::token_links::service::link_kind;
use crate::domain::token_links::types::Pairing;
use crate::fixtures::syntax_files::with_file;
use ryo::languages::{Offset, SyntaxCompleteness};

/// A source.
type Input = &'static str;
/// The links of the source, its regions and the place of its last byte, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_bracket_pair_matches_and_encloses_its_tokens"),
        word: Word::Succeeds,
        input: || "f(a)\n",
        expect: Expect::Value(
            "next-token \"f\">\"(\"|next-token \"(\">\"a\"|enclosing-delimiter \"a\">\"(\"|next-token \"a\">\")\"|matching-delimiter \"(\">\")\"|next-token \")\">\"\\n\"|region 0..5|ends at 2:0",
        ),
    },
    Case {
        name: CaseName("an_indented_block_is_a_region_of_the_module_s"),
        word: Word::Succeeds,
        input: || "if a:\n    b\n",
        expect: Expect::Value(
            "next-token \"if\">\"a\"|next-token \"a\">\":\"|next-token \":\">\"\\n\"|next-token \"\\n\">\"    \"|next-token \"    \">\"b\"|next-token \"b\">\"\\n\"|next-token \"\\n\">\"\"|region 0..12|region 6..12|ends at 3:0",
        ),
    },
    Case {
        name: CaseName("a_bracket_left_open_loses_the_pairing"),
        word: Word::Succeeds,
        input: || "f(a\n",
        expect: Expect::Value(
            "next-token \"f\">\"(\"|next-token \"(\">\"a\"|enclosing-delimiter \"a\">\"(\"|next-token \"a\">\"\\n\"|enclosing-delimiter \"\\n\">\"(\"|pairing lost|region 0..4|ends at 2:0",
        ),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(source, &mut |file| {
        let mut stack = Vec::new();
        let pairing = token_links(file, &mut stack, &mut |link| {
            let (from, to) = (written(file, link.source).0, written(file, link.target).0);
            shown.push(format!("{} {from:?}>{to:?}", link_kind(link.kind).0));
        });
        if pairing == Pairing::Lost {
            shown.push("pairing lost".to_owned());
        }
        let mut regions = Vec::new();
        if indent_regions(file, &mut regions) == SyntaxCompleteness::Complete {
            shown.extend(
                regions
                    .iter()
                    .map(|region| format!("region {}..{}", region.span.start.0, region.span.end.0)),
            );
        }
        let mut lines = FileLines::default();
        line_starts(file, &mut lines);
        let last = line_place(
            &lines,
            Offset(u32::try_from(source.len()).unwrap_or(u32::MAX)),
        );
        shown.push(format!("ends at {}:{}", last.line.0, last.column.0));
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
