//! The `definitions` service's cases: what an element declares, the types that annotate names, and the strings assignments bind.

use super::service::{annotations, assignments, declarations, shape};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::{asked, of_kind, spans};

/// What a row asks of every element of the kind in the source.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// What each declares, as its word and its name.
    Declarations,
    /// Each type and the name it annotates.
    Annotations,
    /// Each name and the string it is bound to.
    Assignments,
    /// What each is, by its shape.
    Shape,
}

type Input = (&'static str, GoNode, Question);
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_and_a_method_are_declared_by_their_names"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() {}\nfunc (t T) m() {}\n",
                GoNode::FuncDecl,
                Question::Declarations,
            )
        },
        expect: Expect::Value("function f, method m"),
    },
    Case {
        name: CaseName("the_blank_identifier_declares_nothing"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nvar _, b int\n",
                GoNode::ValueSpec,
                Question::Declarations,
            )
        },
        expect: Expect::Value("var b"),
    },
    Case {
        name: CaseName("a_struct_an_alias_and_a_defined_type_are_told_apart"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\ntype S struct{}\ntype A = S\ntype N int\n",
                GoNode::TypeSpec,
                Question::Declarations,
            )
        },
        expect: Expect::Value("struct S, alias A, type N"),
    },
    Case {
        name: CaseName("an_embedded_field_is_declared_by_its_type"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\ntype S struct {\n\t*Base\n\tpkg.Other\n}\n",
                GoNode::Field,
                Question::Declarations,
            )
        },
        expect: Expect::Value("embedded-field Base, embedded-field Other"),
    },
    Case {
        name: CaseName("a_name_a_range_clause_declares_is_a_range_variable"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { for k, _ := range m {} }\n",
                GoNode::RangeStmt,
                Question::Declarations,
            )
        },
        expect: Expect::Value("range-var k"),
    },
    Case {
        name: CaseName("a_field_with_names_annotates_each_with_its_type"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\ntype S struct {\n\tx, y int\n}\n",
                GoNode::Field,
                Question::Annotations,
            )
        },
        expect: Expect::Value("int on x, int on y"),
    },
    Case {
        name: CaseName("a_declaration_with_no_type_annotates_nothing"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nvar a = 1\n",
                GoNode::ValueSpec,
                Question::Annotations,
            )
        },
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_name_is_bound_to_a_string_by_each_way_of_binding"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nconst a = \"x\"\n",
                GoNode::ValueSpec,
                Question::Assignments,
            )
        },
        expect: Expect::Value("a = \"x\""),
    },
    Case {
        name: CaseName("an_operator_that_computes_binds_no_string"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { e += \"w\"\n\tg = \"z\" }\n",
                GoNode::AssignStmt,
                Question::Assignments,
            )
        },
        expect: Expect::Value("g = \"z\""),
    },
    Case {
        name: CaseName("a_function_a_type_and_a_field_have_their_shapes"),
        word: Word::Succeeds,
        input: || ("package p\nfunc f() {}\n", GoNode::FuncDecl, Question::Shape),
        expect: Expect::Value("Function"),
    },
];

fn subject((source, node, question): Input) -> Value {
    let answer = asked(source, |tree| {
        of_kind(tree, Kind::Node(node))
            .into_iter()
            .flat_map(|at| -> Vec<String> {
                match question {
                    Question::Declarations => declarations(tree, at)
                        .filter_map(|each| {
                            crate::domain::definitions::service::declared_name(tree, &each).map(
                                |_| {
                                    format!(
                                        "{} {}",
                                        each.defined.word.0,
                                        spans(tree, each.named)
                                            .trim_start_matches('*')
                                            .rsplit('.')
                                            .next()
                                            .unwrap_or_default()
                                    )
                                },
                            )
                        })
                        .collect(),
                    Question::Annotations => annotations(tree, at)
                        .map(|each| {
                            format!(
                                "{} on {}",
                                spans(tree, each.annotation),
                                spans(tree, each.target)
                            )
                        })
                        .collect(),
                    Question::Assignments => assignments(tree, at)
                        .map(|each| {
                            format!(
                                "{} = {}",
                                spans(tree, each.binding),
                                spans(tree, each.value)
                            )
                        })
                        .collect(),
                    Question::Shape => vec![format!("{:?}", shape(tree, at))],
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    });
    Box::leak(answer.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
