// the adapter spec allows one adapter_test.rs, which holds the port contract rows, the cases of the adapter's kind map and macro expansion, and the comparison of the whole tree with the pinned standard
//! The `ra_ap_syntax` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxRaApSyntaxAdapter;
use super::fixtures::comparison::{broken, difference, functions, read};
use super::fixtures::roles::roles;
use super::fixtures::unproduced::{kinds, trees_agree, unproduced_kinds_a_source_gives};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::EXPANDED_MACROS;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::contract::check;
use crate::outbound::syntax::types::Expanding;
use ryo::languages::{ByteSlice, Elements, Position, Symbols, kind_at, positions};

/// What a row is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// Source text whose functions, names and root are looked at.
    Functions(&'static str),
    /// Source text with a syntax error, whose functions are counted.
    Broken(&'static str),
    /// The map from the grammar's kinds to the language's.
    Kinds,
    /// The kinds the parser never gives to a tree built from a file.
    Unproduced,
    /// The trees written by hand for the domain's tests.
    Trees,
    /// The roles the table gives the children of constructs, held to the typed accessors of the library.
    Roles(&'static str),
    /// A source parsed when no query reads the roles: how many elements the table gives a role.
    Unasked(&'static str),
    /// Where the elements of a source come from: the ones a macro's arguments read again are the expansion's, under their call.
    Origins(&'static str),
    /// A file and the tree rust-analyzer's parser printed for it: what differs between our tree, printed the same way, and the
    /// pinned one.
    Standard(&'static str, &'static str),
    /// Every file whose tree is pinned in `standard/`: the ones whose tree differs from the pinned one.
    Pinned,
}

/// What a row of the cases is given.
type Input = Scenario;
/// What the adapter made of it, shown.
type Value = &'static str;
type Failure = ();

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_table_holds_each_function_as_an_fn_element_with_its_extent"),
        word: Word::Succeeds,
        input: || {
            Scenario::Functions("fn one() {}\nstruct S;\nimpl S {\n    fn two(&self) {}\n}\n")
        },
        expect: Expect::Value("2 fn one() {} true source-file"),
    },
    Case {
        name: CaseName("a_file_with_a_syntax_error_still_gives_what_could_be_read"),
        word: Word::Succeeds,
        input: || Scenario::Broken("fn good() {}\nfn broken("),
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("no_element_has_a_role_when_no_query_reads_the_roles"),
        word: Word::Succeeds,
        input: || Scenario::Unasked("pub fn f<T>(a: u8) -> u8 where T: A { a }"),
        expect: Expect::Value("0"),
    },
    Case {
        name: CaseName("a_function_has_its_parts_in_the_roles_its_accessors_name"),
        word: Word::Succeeds,
        input: || Scenario::Roles("pub fn f<T>(a: u8) -> u8 where T: A { a }"),
        expect: Expect::Value("agreed"),
    },
    Case {
        name: CaseName("operands_and_branches_are_in_the_roles_their_accessors_name"),
        word: Word::Succeeds,
        input: || {
            Scenario::Roles(
                "fn f() { let a = b + c; if a { d[e]; } else { g.h(); } for i in j { } while k { } match l { m if n => 1, _ => 2 } m..n; }",
            )
        },
        expect: Expect::Value("agreed"),
    },
    Case {
        name: CaseName("an_impl_has_its_trait_and_its_type_in_the_roles_its_accessors_name"),
        word: Word::Succeeds,
        input: || Scenario::Roles("impl A for B { } impl C { }"),
        expect: Expect::Value("agreed"),
    },
    Case {
        name: CaseName("every_kind_of_the_grammar_maps_to_the_kind_of_its_name"),
        word: Word::Succeeds,
        input: || Scenario::Kinds,
        expect: Expect::Value("mapped"),
    },
    Case {
        name: CaseName("a_kind_the_parser_never_produces_is_an_element_of_no_source"),
        word: Word::Succeeds,
        input: || Scenario::Unproduced,
        expect: Expect::Value("0"),
    },
    Case {
        name: CaseName("the_trees_written_by_hand_are_the_tables_the_adapter_makes"),
        word: Word::Succeeds,
        input: || Scenario::Trees,
        expect: Expect::Value("agreed"),
    },
    Case {
        name: CaseName("the_arguments_of_a_std_macro_are_the_expansion_of_its_call"),
        word: Word::Succeeds,
        input: || Scenario::Origins("fn f() { println!(\"{}\", a + 1); }"),
        expect: Expect::Value("expanded under the call"),
    },
    Case {
        name: CaseName("a_tree_that_is_not_rust_analyzers_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Scenario::Standard(
                "fn f() {}",
                "SOURCE_FILE\n  STRUCT\n    STRUCT_KW \"struct\"\n",
            )
        },
        expect: Expect::Value("line 2: we have `  FN`, the standard has `  STRUCT`"),
    },
    Case {
        name: CaseName("a_tree_printed_as_rust_analyzer_prints_it_is_no_difference"),
        word: Word::Succeeds,
        input: || Scenario::Standard("", "SOURCE_FILE\nerror 0: nothing\n"),
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("every_pinned_file_has_the_tree_rust_analyzer_prints"),
        word: Word::Succeeds,
        input: || Scenario::Pinned,
        expect: Expect::Value("none differ"),
    },
    Case {
        name: CaseName("a_file_without_a_macro_call_has_only_its_own_elements"),
        word: Word::Succeeds,
        input: || Scenario::Origins("fn f() { a + 1; }"),
        expect: Expect::Value("all the file's"),
    },
];

/// The files whose tree is pinned below `standard`, each with its dump and its input, both by path below `fixtures`, sorted: the corpus
/// and the suite at the same path, and each relation's input beside the page `standard/<relation>.rast`.
fn pinned(
    standard: &std::path::Path,
    fixtures: &std::path::Path,
) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut found = Vec::new();
    let mut folders = vec![standard.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        {
            let path = entry.path();
            let Ok(below) = path.strip_prefix(standard) else {
                continue;
            };
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            if path.extension().is_some_and(|each| each == "rast") {
                let input = if below.components().count() == 1 {
                    std::path::Path::new("relations")
                        .join(below.file_stem().unwrap_or_default())
                        .join("input.rs")
                } else {
                    below.with_extension("rs")
                };
                if fixtures.join(&input).exists() {
                    found.push((path.clone(), fixtures.join(input)));
                }
            }
        }
    }
    found.sort();
    found
}

/// The pinned files whose tree is not the one rust-analyzer printed, by path of the input; the answer says so when none differ.
fn pinned_differences() -> String {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let files = pinned(&fixtures.join("standard"), &fixtures);
    if files.is_empty() {
        return "nothing is pinned".to_owned();
    }
    let differing: Vec<String> = files
        .iter()
        .filter_map(|(dump, input)| {
            let (Ok(text), Ok(printed)) = (
                std::fs::read_to_string(input),
                std::fs::read_to_string(dump),
            ) else {
                return Some(format!("{} cannot be read", input.display()));
            };
            difference(&text, &printed).map(|each| format!("{}: {each}", input.display()))
        })
        .collect();
    if differing.is_empty() {
        "none differ".to_owned()
    } else {
        differing.join("; ")
    }
}

/// How many elements of `source` have a role, when the parse is not asked for the roles.
fn unasked(source: &'static str) -> String {
    use ryo::languages::{NO_FIELD, field_at};
    let language = crate::inbound::contracts::parse::contract::ParseContract::new(
        std::sync::Arc::new(SyntaxRaApSyntaxAdapter::new(Expanding(&EXPANDED_MACROS))),
        std::sync::Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    ryo::languages::contract::build(
        &language,
        ryo::languages::ParseRequest {
            path: ryo::languages::SourcePath("src/lib.rs"),
            file: ryo::languages::FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: ryo::languages::NoteRequest::Unwanted,
            fields: ryo::languages::FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
    )
    .expect("the file parses");
    positions(&elements)
        .filter(|at| field_at(&elements, *at) != NO_FIELD)
        .count()
        .to_string()
}

/// Where the elements of `source` come from, after the parse: whether any is an expansion's, and whether each is under a macro call.
fn origins(source: &'static str) -> String {
    use ryo::languages::{ElementOrigin, end_of, origin_at};
    let language = crate::inbound::contracts::parse::contract::ParseContract::new(
        std::sync::Arc::new(SyntaxRaApSyntaxAdapter::new(Expanding(&EXPANDED_MACROS))),
        std::sync::Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    ryo::languages::contract::build(
        &language,
        ryo::languages::ParseRequest {
            path: ryo::languages::SourcePath("src/lib.rs"),
            file: ryo::languages::FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: ryo::languages::NoteRequest::Unwanted,
            fields: ryo::languages::FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
    )
    .expect("the file parses");
    let mut expanded = 0_i32;
    for at in (0..end_of(&elements).0).map(Position) {
        if let ElementOrigin::Expansion { call_site, .. } = origin_at(&elements, at) {
            expanded += 1_i32;
            let under_a_call =
                crate::domain::syntax::service::kind_of(kind_at(&elements, call_site))
                    == Kind::Node(Node::MacroCall);
            if !under_a_call || call_site >= at {
                return "an expansion not under its call".to_owned();
            }
        }
    }
    if expanded == 0 {
        "all the file's".to_owned()
    } else {
        "expanded under the call".to_owned()
    }
}

fn subject(scenario: Input) -> Value {
    let shown = match scenario {
        Scenario::Functions(source) => functions(source),
        Scenario::Broken(source) => broken(source),
        Scenario::Kinds => kinds(),
        Scenario::Unproduced => unproduced_kinds_a_source_gives().to_string(),
        Scenario::Trees => trees_agree(),
        Scenario::Roles(source) => roles(source),
        Scenario::Unasked(source) => unasked(source),
        Scenario::Origins(source) => origins(source),
        Scenario::Standard(source, dump) => {
            difference(source, dump).unwrap_or_else(|| "none".to_owned())
        }
        Scenario::Pinned => pinned_differences(),
    };
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(shown.into_boxed_str())
}

#[test]
fn succeeds() {
    check(SyntaxRaApSyntaxAdapter::new);
    assert_eq!(
        kind_bits(Kind::Node(Node::SourceFile)),
        kind_at(&read("fn f() {}").0, Position(0))
    );
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
