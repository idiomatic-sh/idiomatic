//! The `links` extraction's cases, on source text read through the parse.

use std::sync::Arc;

use ryo::languages;

use super::service::note_of;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::service::facts_of;
use crate::domain::fact_notes::types::Naming;
use crate::domain::module_tree::types::{Body, Cleanliness, Facts};
use crate::domain::paths::types::Section;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::trees::tree_of;

/// A scenario: the source text read, and what is asked of its facts, shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

/// The facts of `source`, as the extraction notes them from the table the parse leaves.
fn facts_of_source(source: &str) -> Facts {
    let use_case = crate::inbound::contracts::parse::contract::ParseContract::new(
        Arc::new(SyntaxFake(
            tree_of(source).expect("a tree is written for the source"),
        )),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let mut table = languages::Elements::default();
    let mut symbols = languages::Symbols::default();
    languages::contract::build(
        &use_case,
        languages::ParseRequest {
            path: languages::SourcePath("src/lib.rs"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(source.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut table,
        &mut symbols,
    )
    .expect("the file parses");
    let tree = languages::Tree {
        elements: &table,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    };
    let note = note_of(&tree);
    facts_of(note, &|_| Naming::Known)
}

fn segments(facts: &Facts) -> String {
    facts
        .written
        .iter()
        .map(|each| {
            each.segments
                .iter()
                .map(|segment| &**segment)
                .collect::<Vec<_>>()
                .join("::")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn modules() -> String {
    let facts = facts_of_source("mod a;\nmod b { mod c; }\n#[path = \"x/y.rs\"] mod d;\n");
    let declared: Vec<String> = facts
        .declared
        .iter()
        .map(|each| {
            let path = match &each.path {
                Section::Present(path) => &**path,
                Section::Absent => "-",
            };
            format!(
                "{}:{}:{}:{path}",
                &*each.name,
                each.body == Body::Inline,
                each.around.len()
            )
        })
        .collect();
    format!(
        "{} {}",
        declared.join(" "),
        facts.clean == Cleanliness::Clean
    )
}

fn uses() -> String {
    segments(&facts_of_source(
        "use crate::a::{b, c as d, self};\nuse super::*;\nfn f() { crate::m::g(); Self::x(); single(); <T as Tr>::f(); }\n",
    ))
}

fn inline_modules() -> String {
    let facts = facts_of_source("mod outer { mod inner { fn f() { self::a::b(); } } }\n");
    let found = facts.written.first().expect("one path is written");
    format!(
        "{} {}",
        found
            .enclosing
            .iter()
            .map(|each| &**each)
            .collect::<Vec<_>>()
            .join(" "),
        found.start < found.end
    )
}

fn use_blocks() -> String {
    let facts = facts_of_source("fn f() { use a; { use b; } mod m { use c; } }");
    facts
        .written
        .iter()
        .map(|each| {
            let block = each.block.map_or_else(
                || "-".to_owned(),
                |span| format!("{}..{}", span.start.0, span.end.0),
            );
            format!("{}:{block}", each.text.0)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn raw_identifiers() -> String {
    let facts = facts_of_source("mod r#type;\nuse crate::r#type::r#enum;\n");
    let declared: Vec<&str> = facts.declared.iter().map(|each| &*each.name).collect();
    format!("{} {}", declared.join(" "), segments(&facts))
}

fn spellings() -> String {
    facts_of_source("use crate::a::{b, c};\nfn f() { crate :: m\n    ::g::<u8>(); }\n")
        .written
        .iter()
        .map(|each| &*each.text.0)
        .collect::<Vec<_>>()
        .join(" ")
}

fn syntax_error() -> String {
    (facts_of_source("fn f( {").clean == Cleanliness::Clean).to_string()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_module_is_declared_with_the_blocks_around_it_and_its_path_attribute"),
        word: Word::Succeeds,
        input: || modules,
        expect: Expect::Value("a:false:0:- b:true:0:- c:false:1:- d:false:0:x/y.rs true"),
    },
    Case {
        name: CaseName("a_use_is_written_leaf_by_leaf_and_a_path_of_two_segments_is_written_once"),
        word: Word::Succeeds,
        input: || uses,
        expect: Expect::Value("crate::a::b crate::a::c crate::a super::* crate::m::g Self::x"),
    },
    Case {
        name: CaseName("a_path_inside_an_inline_module_names_the_module_around_it"),
        word: Word::Succeeds,
        input: || inline_modules,
        expect: Expect::Value("outer inner true"),
    },
    Case {
        name: CaseName("a_use_in_a_block_binds_in_that_block_and_a_use_in_a_module_in_the_module"),
        word: Word::Succeeds,
        input: || use_blocks,
        expect: Expect::Value("a:7..45 b:16..26 c:-"),
    },
    Case {
        name: CaseName("a_raw_identifier_names_the_module_without_its_prefix"),
        word: Word::Succeeds,
        input: || raw_identifiers,
        expect: Expect::Value("type crate::type::enum"),
    },
    Case {
        name: CaseName(
            "a_path_is_spelled_as_the_file_writes_it_from_start_to_end_its_whitespace_left_out",
        ),
        word: Word::Succeeds,
        input: || spellings,
        expect: Expect::Value("b c crate::m::g::<u8>"),
    },
    Case {
        name: CaseName("a_file_with_a_syntax_error_is_not_clean"),
        word: Word::Succeeds,
        input: || syntax_error,
        expect: Expect::Value("false"),
    },
];

fn subject(scenario: Input) -> Value {
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(scenario().into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
