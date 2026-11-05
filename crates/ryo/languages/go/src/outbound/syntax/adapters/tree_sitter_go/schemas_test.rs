//! The schemas' cases: every kind of the grammar is mapped, or named as dropped, so a kind a new version of the grammar gains is a
//! failing test until it is; and a kind the adapter maps is a kind the grammar has.

use super::{FieldSchema, KindSchema, spelling};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::grammar::types::{Form, Role};
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// What a row asks about the grammar's kinds.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Every named kind of the grammar: those the adapter has no form for.
    Named,
    /// Every anonymous kind of the grammar: those that spell no token of `go/token` and are not dropped.
    Anonymous,
    /// A named kind: its form.
    Kind(&'static str),
    /// A spelling: its form.
    Spelling(&'static str),
    /// A field of the grammar: the part it plays.
    Field(&'static str),
    /// Whether the grammar has a kind of that name.
    Has(&'static str),
    /// Every kind the grammar names: each is mapped or dropped.
    Every,
}

type Input = Given;
type Value = Seen;
type Failure = ();

/// What a case sees.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kinds, in the grammar's order.
    Kinds(&'static [&'static str]),
    /// A form, or none for a kind the adapter does not know.
    Form(Option<Form>),
    /// A part, or none for a field the language reads nothing from.
    Role(Option<Role>),
    /// Whether the grammar has the kind.
    Has(bool),
    /// Whether every kind of the grammar is mapped to a kind of the language or dropped with a reason.
    Accounted(bool),
}

/// The anonymous kinds the adapter drops: the quotes a literal is written with, which the literal's token holds, and the line end and
/// the end of the file the library writes as a terminator, which are white space to `go/scanner`.
const DROPPED: &[&str] = &["\"", "`", "\n", "\0", ""];

/// The visible kinds of the grammar: its named nodes and its anonymous ones, with whether each is named.
fn library_kinds() -> Vec<(String, bool)> {
    let language: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();
    (0..u16::try_from(language.node_kind_count()).unwrap_or(u16::MAX))
        .filter(|id| language.node_kind_is_visible(*id))
        .filter_map(|id| {
            language
                .node_kind_for_id(id)
                .map(|kind| (kind.to_owned(), language.node_kind_is_named(id)))
        })
        .collect()
}

/// Why the language has no kind for a kind of the grammar, or none when it has one: a hidden rule of the grammar, or one of the
/// anonymous kinds the adapter drops.
fn dropped(kind: &str, named: bool) -> Option<&'static str> {
    if named && kind.starts_with('_') {
        Some("a hidden rule of the grammar, which no node of a tree is")
    } else if !named && DROPPED.contains(&kind) {
        Some(
            "a quote or a terminator the grammar writes, which the literal's token or white space holds",
        )
    } else {
        None
    }
}

/// Whether the language answers for a kind of the grammar: it is dropped with a reason, or it is mapped to a form or a token.
fn accounted(kind: &str, named: bool) -> bool {
    dropped(kind, named).map_or_else(
        || {
            if named {
                Form::try_from(KindSchema(kind)).err().is_none()
            } else {
                Token::try_from(kind).err().is_none()
            }
        },
        |reason| !reason.is_empty(),
    )
}

/// The kinds, kept for the life of the process: the value of a row is `'static`, and a test is short.
fn leaked(kinds: Vec<String>) -> Seen {
    Seen::Kinds(Box::leak(
        kinds
            .into_iter()
            .map(|kind| &*Box::leak(kind.into_boxed_str()))
            .collect::<Vec<&'static str>>()
            .into_boxed_slice(),
    ))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
    Case {
        name: CaseName("every_named_kind_of_the_grammar_has_a_form"),
        word: Word::Succeeds,
        input: || Given::Named,
        expect: Expect::Value(Seen::Kinds(&[])),
    },
    Case {
        name: CaseName("every_anonymous_kind_of_the_grammar_is_a_token_of_go_token_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Anonymous,
        expect: Expect::Value(Seen::Kinds(&[])),
    },
    Case {
        name: CaseName("a_function_declaration_is_a_function"),
        word: Word::Succeeds,
        input: || Given::Kind("function_declaration"),
        expect: Expect::Value(Seen::Form(Some(Form::Function))),
    },
    Case {
        name: CaseName("a_name_is_a_name"),
        word: Word::Succeeds,
        input: || Given::Kind("identifier"),
        expect: Expect::Value(Seen::Form(Some(Form::Name))),
    },
    Case {
        name: CaseName("the_name_of_a_type_is_the_name_of_a_type"),
        word: Word::Succeeds,
        input: || Given::Kind("type_identifier"),
        expect: Expect::Value(Seen::Form(Some(Form::TypeName))),
    },
    Case {
        name: CaseName("a_loop_is_a_loop_whether_or_not_it_ranges"),
        word: Word::Succeeds,
        input: || Given::Kind("for_statement"),
        expect: Expect::Value(Seen::Form(Some(Form::Loop))),
    },
    Case {
        name: CaseName("a_dereference_is_a_unary_expression_which_go_ast_splits"),
        word: Word::Succeeds,
        input: || Given::Kind("unary_expression"),
        expect: Expect::Value(Seen::Form(Some(Form::Unary))),
    },
    Case {
        name: CaseName("a_pointer_type_is_a_star_expr"),
        word: Word::Succeeds,
        input: || Given::Kind("pointer_type"),
        expect: Expect::Value(Seen::Form(Some(Form::Node(Node::StarExpr)))),
    },
    Case {
        name: CaseName("a_list_the_library_makes_a_node_of_has_none"),
        word: Word::Succeeds,
        input: || Given::Kind("argument_list"),
        expect: Expect::Value(Seen::Form(Some(Form::Transparent))),
    },
    Case {
        name: CaseName("a_break_is_a_branch_of_its_keyword"),
        word: Word::Succeeds,
        input: || Given::Kind("break_statement"),
        expect: Expect::Value(Seen::Form(Some(Form::Branch(Token::Break)))),
    },
    Case {
        name: CaseName("a_kind_no_one_wrote_is_unknown"),
        word: Word::Succeeds,
        input: || Given::Kind("not_a_kind_of_go"),
        expect: Expect::Value(Seen::Form(None)),
    },
    Case {
        name: CaseName("an_operator_is_its_token"),
        word: Word::Succeeds,
        input: || Given::Spelling("&^="),
        expect: Expect::Value(Seen::Form(Some(Form::Spelled(Token::AndNotAssign)))),
    },
    Case {
        name: CaseName("a_spelling_go_token_has_no_constant_for_is_unspelled"),
        word: Word::Succeeds,
        input: || Given::Spelling("@"),
        expect: Expect::Value(Seen::Form(Some(Form::Unspelled))),
    },
    Case {
        name: CaseName("the_field_name_is_the_part_name"),
        word: Word::Succeeds,
        input: || Given::Field("name"),
        expect: Expect::Value(Seen::Role(Some(Role::Name))),
    },
    Case {
        name: CaseName("a_field_the_language_reads_nothing_from_is_no_part"),
        word: Word::Succeeds,
        input: || Given::Field("tag"),
        expect: Expect::Value(Seen::Role(None)),
    },
    Case {
        name: CaseName("the_source_file_is_a_kind_of_the_grammar"),
        word: Word::Succeeds,
        input: || Given::Has("source_file"),
        expect: Expect::Value(Seen::Has(true)),
    },
    Case {
        name: CaseName("a_func_literal_is_a_kind_of_the_grammar"),
        word: Word::Succeeds,
        input: || Given::Has("func_literal"),
        expect: Expect::Value(Seen::Has(true)),
    },
    Case {
        name: CaseName("a_type_switch_statement_is_a_kind_of_the_grammar"),
        word: Word::Succeeds,
        input: || Given::Has("type_switch_statement"),
        expect: Expect::Value(Seen::Has(true)),
    },
    Case {
        name: CaseName("a_short_var_declaration_is_a_kind_of_the_grammar"),
        word: Word::Succeeds,
        input: || Given::Has("short_var_declaration"),
        expect: Expect::Value(Seen::Has(true)),
    },
    Case {
        name: CaseName("an_interpreted_string_literal_is_a_kind_of_the_grammar"),
        word: Word::Succeeds,
        input: || Given::Has("interpreted_string_literal"),
        expect: Expect::Value(Seen::Has(true)),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the subject of a case returns a Result for the rows that refuse, whether or not a row of this part's cases does"
)]
fn subject(given: Input) -> Result<Value, Failure> {
    Ok(match given {
        Given::Named => leaked(
            library_kinds()
                .into_iter()
                .filter(|(kind, named)| *named && !kind.starts_with('_'))
                .filter_map(|(kind, _)| {
                    Form::try_from(KindSchema(&kind))
                        .err()
                        .map(|unseen| unseen.0.to_owned())
                })
                .collect(),
        ),
        Given::Anonymous => leaked(
            library_kinds()
                .into_iter()
                .filter(|(_, named)| !*named)
                .filter_map(|(kind, _)| {
                    Token::try_from(kind.as_str())
                        .err()
                        .map(|unspelled| unspelled.0.to_owned())
                })
                .filter(|kind| !DROPPED.contains(&kind.as_str()))
                .collect(),
        ),
        Given::Kind(kind) => Seen::Form(Form::try_from(KindSchema(kind)).ok()),
        Given::Spelling(text) => Seen::Form(Some(spelling(text))),
        Given::Field(name) => Seen::Role(Option::<Role>::from(FieldSchema(name))),
        Given::Has(kind) => Seen::Has(library_kinds().iter().any(|(each, _)| each == kind)),
        Given::Every => Seen::Accounted(
            library_kinds()
                .iter()
                .all(|(kind, named)| accounted(kind, *named)),
        ),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
