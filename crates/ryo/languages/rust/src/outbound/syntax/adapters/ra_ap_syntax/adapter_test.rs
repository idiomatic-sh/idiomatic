// the adapter spec allows one adapter_test.rs, which holds the port contract rows, the cases of the adapter's kind map and macro expansion, and the comparison of the whole tree with the pinned standard
//! The `ra_ap_syntax` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::constants::EXPANDED_MACROS;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::types::{Expanding, SourceText};
use ra_ap_syntax::SyntaxKind;
use ryo::languages::{
    ByteSlice, Elements, Position, Symbol, Symbols, TOKEN, end_at, kind_at, positions, start_at,
    table_builder, text_of,
};

use super::adapter::SyntaxRaApSyntaxAdapter;
use super::schemas::kind_of;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_bits, kind_name};
use crate::domain::syntax::types::Kind;
use crate::fixtures::kinds::sample;
use crate::outbound::syntax::fixtures::contract::{Told, check, expanding, made_by};
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::trees::{SOURCES, tree_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo::languages::Sample;

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

fn read(source: &str) -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxRaApSyntaxAdapter::new(Expanding(&EXPANDED_MACROS)).read(
        SourceText(source.as_bytes()),
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes())),
    );
    (elements, symbols)
}

fn functions(source: &'static str) -> String {
    let (elements, symbols) = read(source);
    let functions: Vec<Position> = positions(&elements)
        .filter(|position| kind_of_bits(&elements, *position) == Kind::Node(RustNode::Fn))
        .collect();
    let first = functions[0];
    let (start, end) = (start_at(&elements, first), end_at(&elements, first));
    let named: Vec<&str> = (0..ryo::languages::contract::name_count(&symbols).0)
        .filter_map(|place| {
            text_of(
                &symbols,
                ByteSlice(source.as_bytes()),
                Symbol(u32::try_from(place).unwrap_or(0)),
            )
            .map(|text| text.0)
        })
        .collect();
    format!(
        "{} {} {} {}",
        functions.len(),
        source
            .get(usize::try_from(start.0).unwrap_or(0)..usize::try_from(end.0).unwrap_or(0))
            .unwrap_or_default(),
        named.contains(&"one") && named.contains(&"two"),
        kind_name(kind_of_bits(&elements, Position(0))).0
    )
}

/// `source` read into a table, with no macro read again: the file's own tree.
fn own_tree(source: &str) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxRaApSyntaxAdapter::new(Expanding(&[])).read(
        SourceText(source.as_bytes()),
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes())),
    );
    elements
}

/// Our tree of `source` printed as the tests of `ra_ap_parser` print theirs: a line for each node, `KIND`, and each token, `KIND "text"`,
/// indented two spaces for each level.
fn printed(source: &str) -> String {
    let elements = own_tree(source);
    let mut levels: Vec<usize> = Vec::new();
    let mut out = String::new();
    for at in positions(&elements) {
        let parent = ryo::languages::parent_at(&elements, at);
        let level = if parent == at {
            0
        } else {
            usize::try_from(parent.0)
                .ok()
                .and_then(|place| levels.get(place))
                .map_or(0, |above| above + 1)
        };
        levels.push(level);
        let kind = kind_of_bits(&elements, at);
        let name = kind_name(kind).0.to_uppercase().replace('-', "_");
        out.push_str(&"  ".repeat(level));
        out.push_str(&name);
        // The parser's `ERROR` is a kind the language lists among its tokens, and a node of the tree: it is a leaf when the table says so.
        if ryo::languages::flags_at(&elements, at).0 & TOKEN.0 != 0 {
            let (from, to) = (
                usize::try_from(start_at(&elements, at).0).unwrap_or(0),
                usize::try_from(end_at(&elements, at).0).unwrap_or(0),
            );
            let text = format!(" {:?}", source.get(from..to).unwrap_or_default());
            out.push_str(&text);
        }
        out.push('\n');
    }
    out
}

/// The first line where our tree of `source` is not the pinned `dump`'s tree, whose `error` lines (the parser's errors, which are not
/// the tree) are left out; nothing when they are the same.
fn difference(source: &str, dump: &str) -> Option<String> {
    let ours = printed(source);
    let theirs: Vec<&str> = dump
        .lines()
        .take_while(|line| !line.starts_with("error "))
        .collect();
    let mine: Vec<&str> = ours.lines().collect();
    (0..mine.len().max(theirs.len())).find_map(|place| {
        let (have, standard) = (mine.get(place), theirs.get(place));
        (have != standard).then(|| {
            format!(
                "line {}: we have `{}`, the standard has `{}`",
                place + 1,
                have.copied().unwrap_or_default(),
                standard.copied().unwrap_or_default()
            )
        })
    })
}

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

fn kind_of_bits(elements: &Elements, position: Position) -> Kind {
    crate::domain::syntax::service::kind_of(kind_at(elements, position))
}

fn broken(source: &'static str) -> String {
    let (elements, _) = read(source);
    positions(&elements)
        .any(|position| kind_of_bits(&elements, position) == Kind::Node(RustNode::Fn))
        .to_string()
}

/// The sources the parser was probed with: the constructs the grammar names, nightly ones included, and the files with errors.
const PROBES: &[&str] = &[
    "fn main() {}",
    "fn f() { builtin # asm(\"\", out(reg) x, options(pure, att_syntax), clobber_abi(\"C\"), label { }) }",
    "fn f() { builtin # offset_of(S, f); builtin # format_args(\"{}\", x); let _ = builtin # include_bytes(\"f\"); let builtin # deref(x) = y; }",
    "type T = builtin # pattern_type(u32 is !null);",
    "---cargo\n[dependencies]\n---\nfn main() {}",
    "fn f() { do yeet 1; become g(); gen { yield 1 }; try { 1 }; let box x = y; match x { const { 1 } => {} } }",
    "impl(crate) trait T {}\nstruct S { pub mut(crate) x: u8 }\nauto trait A {}\nfn f() where T::foo(..): Send {}",
    "fn f( {\nlet abstract = 1;\n",
    "#!/usr/bin/env run\nmacro_rules! m { () => {} }\nunion U { a: u8 }\nunsafe extern \"C\" { safe fn g(); }\n",
];

/// The kinds the grammar defines and the parser never gives to a tree built from a file (their reasons are in `sample`).
const UNPRODUCED: [Kind; 6] = [
    Kind::Extra(Extra::Other),
    Kind::Node(RustNode::MacroItems),
    Kind::Node(RustNode::MacroStmts),
    Kind::Token(Token::Newline),
    Kind::Token(Token::BikeshedKw),
    Kind::Token(Token::NullKw),
];

/// How many of the kinds never produced are an element of a source anyway: of the probes, or of the sample of a kind.
fn unproduced_kinds_a_source_gives() -> usize {
    let adapter = SyntaxRaApSyntaxAdapter::new(expanding(Told::Standard));
    let samples = crate::domain::syntax::service::kinds().filter_map(|kind| match sample(kind) {
        Sample::Node(source) | Sample::Token(source) => Some(source.0),
        Sample::Unread(_) => None,
    });
    samples
        .chain(PROBES.iter().copied())
        .flat_map(|source| made_by(&adapter, source))
        .filter(|made| UNPRODUCED.contains(&made.element.kind))
        .count()
}
/// Whether every tree written by hand for a source of the domain's tests is the table this adapter makes of it: the first source for
/// which they differ, or `agreed`.
fn trees_agree() -> String {
    let adapter = SyntaxRaApSyntaxAdapter::new(expanding(Told::Nothing));
    for source in SOURCES {
        let made = made_by(&adapter, source);
        let written = made_by(
            &SyntaxFake(tree_of(source).expect("a tree is written for the source")),
            source,
        );
        if made != written {
            return format!("the tree of {source:?} differs from the adapter's");
        }
    }
    "agreed".to_owned()
}

/// Whether every kind of the grammar but the parser's three markers maps to the kind of its own name, which is written the way the
/// language writes its kinds: lower case, `-` for `_`.
fn kinds() -> String {
    let markers = [SyntaxKind::TOMBSTONE, SyntaxKind::EOF, SyntaxKind::__LAST];
    for number in 0..=u16::from(SyntaxKind::__LAST) {
        let library = SyntaxKind::from(number);
        let ours = kind_of(library);
        let right = if markers.contains(&library) {
            ours == Kind::Extra(Extra::Other)
        } else {
            kind_name(ours).0 == format!("{library:?}").to_lowercase().replace('_', "-")
        };
        if !right {
            return format!("{library:?} maps to the kind of another name");
        }
    }
    "mapped".to_owned()
}

/// A child an accessor names: the construct it is a child of, its place, and the role the accessor is called.
type Said = (
    ra_ap_syntax::SyntaxNode,
    ra_ap_syntax::TextRange,
    &'static str,
);

/// The children the accessors of the library name in `source`, each with the construct it is a child of and the role the accessor is
/// called.
fn accessor_roles(source: &'static str) -> Vec<Said> {
    use ra_ap_syntax::{AstNode as _, Edition, SourceFile};
    let mut said: Vec<Said> = Vec::new();
    let file = SourceFile::parse(source, Edition::CURRENT).tree();
    for node in file.syntax().descendants() {
        declared_roles(&node, &mut said);
        expression_roles(&node, &mut said);
    }
    said
}

/// The children the accessors of a function, an impl or a `let` name.
fn declared_roles(owner: &ra_ap_syntax::SyntaxNode, said: &mut Vec<Said>) {
    use ra_ap_syntax::AstNode as _;
    use ra_ap_syntax::ast::{self, HasGenericParams as _, HasName as _, HasVisibility as _};
    let node = owner;
    let mut say = |child: Option<ra_ap_syntax::SyntaxNode>, role: &'static str| {
        if let Some(child) = child {
            said.push((owner.clone(), child.text_range(), role));
        }
    };
    if let Some(each) = ast::Fn::cast(node.clone()) {
        say(each.name().map(|x| x.syntax().clone()), "name");
        say(each.param_list().map(|x| x.syntax().clone()), "param-list");
        say(each.ret_type().map(|x| x.syntax().clone()), "ret-type");
        say(each.body().map(|x| x.syntax().clone()), "body");
        say(
            each.generic_param_list().map(|x| x.syntax().clone()),
            "generic-param-list",
        );
        say(
            each.where_clause().map(|x| x.syntax().clone()),
            "where-clause",
        );
        say(each.visibility().map(|x| x.syntax().clone()), "visibility");
        return;
    }
    if let Some(each) = ast::Impl::cast(node.clone()) {
        say(each.trait_().map(|x| x.syntax().clone()), "trait");
        say(each.self_ty().map(|x| x.syntax().clone()), "self-ty");
        return;
    }
    if let Some(each) = ast::LetStmt::cast(node.clone()) {
        say(
            each.initializer().map(|x| x.syntax().clone()),
            "initializer",
        );
        say(each.pat().map(|x| x.syntax().clone()), "pat");
    }
}

/// The children the accessors of an expression, a loop, a match arm or a range name.
fn expression_roles(owner: &ra_ap_syntax::SyntaxNode, said: &mut Vec<Said>) {
    use ra_ap_syntax::AstNode as _;
    use ra_ap_syntax::ast::{self, HasLoopBody as _, RangeItem as _};
    let node = owner;
    let mut say = |child: Option<ra_ap_syntax::SyntaxNode>, role: &'static str| {
        if let Some(child) = child {
            said.push((owner.clone(), child.text_range(), role));
        }
    };
    if let Some(each) = ast::BinExpr::cast(node.clone()) {
        say(each.lhs().map(|x| x.syntax().clone()), "lhs");
        say(each.rhs().map(|x| x.syntax().clone()), "rhs");
        return;
    }
    if let Some(each) = ast::IfExpr::cast(node.clone()) {
        say(each.condition().map(|x| x.syntax().clone()), "condition");
        say(
            each.then_branch().map(|x| x.syntax().clone()),
            "then-branch",
        );
        say(
            each.else_branch().map(|x| x.syntax().clone()),
            "else-branch",
        );
        return;
    }
    if let Some(each) = ast::IndexExpr::cast(node.clone()) {
        say(each.base().map(|x| x.syntax().clone()), "base");
        say(each.index().map(|x| x.syntax().clone()), "index");
        return;
    }
    if let Some(each) = ast::MethodCallExpr::cast(node.clone()) {
        say(each.receiver().map(|x| x.syntax().clone()), "receiver");
        return;
    }
    if let Some(each) = ast::ForExpr::cast(node.clone()) {
        say(each.iterable().map(|x| x.syntax().clone()), "iterable");
        say(each.loop_body().map(|x| x.syntax().clone()), "loop-body");
        say(each.pat().map(|x| x.syntax().clone()), "pat");
        return;
    }
    if let Some(each) = ast::WhileExpr::cast(node.clone()) {
        say(each.condition().map(|x| x.syntax().clone()), "condition");
        say(each.loop_body().map(|x| x.syntax().clone()), "loop-body");
        return;
    }
    if let Some(each) = ast::MatchArm::cast(node.clone()) {
        say(each.guard().map(|x| x.syntax().clone()), "guard");
        say(each.pat().map(|x| x.syntax().clone()), "pat");
        return;
    }
    if let Some(each) = ast::RangeExpr::cast(node.clone()) {
        say(each.start().map(|x| x.syntax().clone()), "start");
        say(each.end().map(|x| x.syntax().clone()), "end");
    }
}

/// The roles the table gives the children of the constructs of `source`, against the accessors of the library: every child an accessor
/// names is in the role the accessor is called, and the answer lists the ones that are not.
fn roles(source: &'static str) -> String {
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
            fields: ryo::languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .expect("the file parses");
    let said = accessor_roles(source);
    let tree = ryo::languages::Tree {
        elements: &elements,
        source: ByteSlice(source.as_bytes()),
        file: ryo::languages::FileId(0),
    };
    let range_of = |at: Position| {
        (
            usize::try_from(start_at(&elements, at).0).unwrap_or(0),
            usize::try_from(end_at(&elements, at).0).unwrap_or(0),
        )
    };
    let mut disagreements = Vec::new();
    for (owner, range, role) in said {
        let owner_range = owner.text_range();
        let wanted = (
            usize::from(owner_range.start()),
            usize::from(owner_range.end()),
        );
        let child_range = (usize::from(range.start()), usize::from(range.end()));
        let found = positions(&elements).find(|at| {
            range_of(*at) == wanted
                && crate::domain::syntax::service::kind_of(kind_at(&elements, *at))
                    == kind_of_library(owner.kind())
        });
        let role_found = found.and_then(|owner_at| {
            ryo::languages::children(&elements, owner_at)
                .find(|each| range_of(*each) == child_range)
        });
        let name = role_found.and_then(|each| {
            text_of(
                &symbols,
                tree.source,
                ryo::languages::field_name_at(&elements, each),
            )
            .map(|text| text.0)
        });
        if name != Some(role) {
            disagreements.push(format!("{role}: {name:?}"));
        }
    }
    if disagreements.is_empty() {
        "agreed".to_owned()
    } else {
        disagreements.join("; ")
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
                    == Kind::Node(RustNode::MacroCall);
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

/// The kind the language has for a kind of the library.
fn kind_of_library(library: SyntaxKind) -> Kind {
    kind_of(library)
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
        kind_bits(Kind::Node(RustNode::SourceFile)),
        kind_at(&read("fn f() {}").0, Position(0))
    );
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
