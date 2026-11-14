//! The `language` service's cases.

use super::service::{dialect, reading_of, reads, recognized};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::types::{Dialect, Scripting, Stretch, Written};
use ryo::languages;

/// What a row asks of a path.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Whether the language reads the file at the path.
    Reads(&'static str),
    /// The dialect the file at the path is read in.
    Dialect(&'static str),
    /// Whether the language reads the file at the path, of another format, carrying blocks written as these say, and the
    /// dialect it reads them in.
    Blocks(&'static str, &'static [Written]),
}

/// What the service answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// Whether the language reads the file.
    Reads(bool),
    /// The dialect the file is read in.
    Dialect(Dialect),
    /// Whether the language reads the file of blocks, and the dialect the script they make is read in.
    Blocks(bool, Dialect),
}

type Input = Scenario;
type Value = Answer;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_typescript_file_is_read"),
        word: Word::Succeeds,
        input: || Scenario::Reads("src/main.ts"),
        expect: Expect::Value(Answer::Reads(true)),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || Scenario::Reads("README.md"),
        expect: Expect::Value(Answer::Reads(false)),
    },
    Case {
        name: CaseName("a_ts_file_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_tsx_file_has_jsx"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/view.tsx"),
        expect: Expect::Value(Answer::Dialect(Dialect::Tsx)),
    },
    Case {
        name: CaseName("an_mts_file_is_a_module"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.mts"),
        expect: Expect::Value(Answer::Dialect(Dialect::Module)),
    },
    Case {
        name: CaseName("a_cts_file_is_commonjs"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.cts"),
        expect: Expect::Value(Answer::Dialect(Dialect::CommonJs)),
    },
    Case {
        name: CaseName("a_d_ts_file_is_declarations"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("types/index.d.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::Declarations)),
    },
    Case {
        name: CaseName("a_declaration_of_an_arbitrary_extension_is_declarations"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/styles.d.css.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::Declarations)),
    },
    Case {
        name: CaseName("a_d_mts_file_is_the_declarations_of_a_module"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("types/index.d.mts"),
        expect: Expect::Value(Answer::Dialect(Dialect::ModuleDeclarations)),
    },
    Case {
        name: CaseName("a_d_cts_file_is_the_declarations_of_commonjs"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("types/index.d.cts"),
        expect: Expect::Value(Answer::Dialect(Dialect::CommonJsDeclarations)),
    },
    Case {
        name: CaseName("a_file_named_d_ts_alone_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/d.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_js_file_is_read"),
        word: Word::Succeeds,
        input: || Scenario::Reads("src/main.js"),
        expect: Expect::Value(Answer::Reads(true)),
    },
    Case {
        name: CaseName("a_js_file_is_plain_javascript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.js"),
        expect: Expect::Value(Answer::Dialect(Dialect::JavaScript)),
    },
    Case {
        name: CaseName("a_jsx_file_is_javascript_with_jsx"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/view.jsx"),
        expect: Expect::Value(Answer::Dialect(Dialect::Jsx)),
    },
    Case {
        name: CaseName("an_mjs_file_is_a_javascript_module"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.mjs"),
        expect: Expect::Value(Answer::Dialect(Dialect::JavaScriptModule)),
    },
    Case {
        name: CaseName("a_cjs_file_is_javascript_commonjs"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.cjs"),
        expect: Expect::Value(Answer::Dialect(Dialect::JavaScriptCommonJs)),
    },
    Case {
        name: CaseName("a_ts_file_named_after_a_javascript_module_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/x.mjs.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_vue_file_of_one_plain_script_block_is_read_as_javascript"),
        word: Word::Succeeds,
        input: || Scenario::Blocks("x.vue", &[Written::Script]),
        expect: Expect::Value(Answer::Blocks(true, Dialect::JavaScript)),
    },
    Case {
        name: CaseName("an_html_file_of_two_plain_script_blocks_is_read_as_javascript"),
        word: Word::Succeeds,
        input: || Scenario::Blocks("x.html", &[Written::Script, Written::Script]),
        expect: Expect::Value(Answer::Blocks(true, Dialect::JavaScript)),
    },
    Case {
        name: CaseName("a_vue_file_of_one_typescript_block_is_read_as_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Blocks("x.vue", &[Written::TypeScript]),
        expect: Expect::Value(Answer::Blocks(true, Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_typed_block_reads_the_plain_blocks_beside_it_as_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Blocks("x.vue", &[Written::Script, Written::TypeScript]),
        expect: Expect::Value(Answer::Blocks(true, Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_vue_file_of_no_script_block_is_declined"),
        word: Word::Succeeds,
        input: || Scenario::Blocks("x.vue", &[]),
        expect: Expect::Value(Answer::Blocks(false, Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_file_of_another_extension_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("component.vue"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
];

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::Reads(path) => {
            Answer::Reads(reads(languages::SourcePath(path)) == languages::Recognition::Reads)
        }
        Scenario::Dialect(path) => Answer::Dialect(dialect(languages::SourcePath(path))),
        Scenario::Blocks(path, written) => {
            let bytes = [b' '; 64];
            let blocks: Vec<Stretch<'_>> = written
                .iter()
                .enumerate()
                .map(|(index, written)| Stretch {
                    start: languages::Count(index * 8),
                    text: languages::ByteSlice(b"x;"),
                    written: *written,
                })
                .collect();
            let reads = recognized(reads(languages::SourcePath(path)), Some(&blocks))
                == languages::Recognition::Reads;
            let read_as = match reading_of(
                languages::SourcePath(path),
                languages::ByteSlice(&bytes),
                Some(&blocks),
            ) {
                Scripting::Own(dialect) | Scripting::Script(_, dialect) => dialect,
            };
            Answer::Blocks(reads, read_as)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
