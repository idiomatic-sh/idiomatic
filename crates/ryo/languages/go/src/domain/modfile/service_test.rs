//! The `modfile` service's cases: the tree it writes for a `go.mod` holds its directives as lines and blocks, and its tokens spell the
//! file.

use super::service::read;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_name, kind_of};
use ryo::languages::{self, ByteSlice, Elements, Symbols};
use std::fmt::Write as _;

/// What a row asks of the service, of the `go.mod` text it reads.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The nodes of the tree, one line each.
    Outline(&'static str),
    /// The tokens of the table, joined.
    Spelling(&'static str),
}

type Input = Question;
/// The elements of the tree, one line each: how deep, its kind and the text it spans; or, for a spelling row, the tokens joined.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_directive_is_a_line_of_its_words_and_a_block_holds_its_lines"),
        word: Word::Succeeds,
        input: || Question::Outline("module m\nrequire (\n\ta v1\n)\n"),
        expect: Expect::Value(
            "0 mod-file \"module m\\nrequire (\\n\\ta v1\\n)\\n\"\n\
             1 mod-line \"module m\"\n\
             1 mod-line-block \"require (\\n\\ta v1\\n)\"\n\
             2 mod-line \"a v1\"\n",
        ),
    },
    Case {
        name: CaseName("a_comment_and_a_quoted_word_are_tokens_of_their_own"),
        word: Word::Succeeds,
        input: || Question::Outline("require \"x/y\" v1 // indirect\n"),
        expect: Expect::Value(
            "0 mod-file \"require \\\"x/y\\\" v1 // indirect\\n\"\n\
             1 mod-line \"require \\\"x/y\\\" v1\"\n",
        ),
    },
    Case {
        name: CaseName("a_block_left_open_ends_with_the_file"),
        word: Word::Succeeds,
        input: || Question::Outline("require (\n\ta v1\n"),
        expect: Expect::Value(
            "0 mod-file \"require (\\n\\ta v1\\n\"\n\
             1 mod-line-block \"require (\\n\\ta v1\\n\"\n\
             2 mod-line \"a v1\"\n",
        ),
    },
    Case {
        name: CaseName("the_tokens_spell_the_file"),
        word: Word::Succeeds,
        input: || {
            Question::Spelling("module m // c\n\nrequire (\n\t\"a\" v1\n\tb v2 // indirect\n)\n")
        },
        expect: Expect::Value("module m // c\n\nrequire (\n\t\"a\" v1\n\tb v2 // indirect\n)\n"),
    },
];

/// The nodes of the tree the adapter writes for `text`, one line each, with how deep each is.
fn outline(text: &'static str) -> Value {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    read(ByteSlice(text.as_bytes()), &mut builder);
    let mut shown = String::new();
    for at in languages::positions(&elements) {
        if languages::flags_at(&elements, at).0 & languages::TOKEN.0 != 0 {
            continue;
        }
        let mut depth = 0_usize;
        let mut up = at;
        while languages::parent_at(&elements, up) != up {
            up = languages::parent_at(&elements, up);
            depth += 1_usize;
        }
        let (start, end) = (
            usize::try_from(languages::start_at(&elements, at).0).unwrap_or(0),
            usize::try_from(languages::end_at(&elements, at).0).unwrap_or(0),
        );
        let spanned = text.get(start..end).unwrap_or_default();
        writeln!(
            shown,
            "{depth} {} \"{}\"",
            kind_name(kind_of(languages::kind_at(&elements, at))).0,
            spanned.escape_debug()
        )
        .unwrap_or_default();
    }
    Box::leak(shown.into_boxed_str())
}

/// The tokens of the table for `text`, joined: the file again, byte for byte.
fn spelled(text: &'static str) -> Value {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder =
        languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    read(ByteSlice(text.as_bytes()), &mut builder);
    let joined: String = languages::positions(&elements)
        .filter(|at| languages::flags_at(&elements, *at).0 & languages::TOKEN.0 != 0)
        .map(|at| {
            let (start, end) = (
                usize::try_from(languages::start_at(&elements, at).0).unwrap_or(0),
                usize::try_from(languages::end_at(&elements, at).0).unwrap_or(0),
            );
            text.get(start..end).unwrap_or_default()
        })
        .collect();
    Box::leak(joined.into_boxed_str())
}

fn subject(question: Question) -> Value {
    match question {
        Question::Outline(text) => outline(text),
        Question::Spelling(text) => spelled(text),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |question| Ok(subject(question)));
}
