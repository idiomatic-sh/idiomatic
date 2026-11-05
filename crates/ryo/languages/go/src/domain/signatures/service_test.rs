//! The `signatures` service's cases: what the fields of a function's lists are.

use super::service::{callable, listing, parameters, signature};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::{asked, of_kind, parsed, spans};
use ryo::languages::{self, ByteSlice, Tree};

/// What a row asks of a source.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Each parameter in it, in order, as its word, its place and its name.
    Parameters,
    /// The parts of its first function declaration, each with the text it spans.
    Parts,
    /// What each list of fields of its first function is, and whether a function owns it.
    Lists,
}

/// A source and a question about it.
type Input = (&'static str, Question);
/// The answer, one line for each thing.
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_field_declares_a_parameter_for_each_name"),
        word: Word::Succeeds,
        input: || ("func f(a, b int, c ...string) {}\n", Question::Parameters),
        expect: Expect::Value(&["parameter 0 a", "parameter 1 b", "variadic 2 c"]),
    },
    Case {
        name: CaseName("a_method_has_a_receiver_and_named_results"),
        word: Word::Succeeds,
        input: || ("func (t T) f() (n int, error) {}\n", Question::Parameters),
        expect: Expect::Value(&["receiver 0 t", "result 0 n"]),
    },
    Case {
        name: CaseName("a_field_with_a_type_alone_is_a_parameter_with_no_name"),
        word: Word::Succeeds,
        input: || ("func f(int, ...string) {}\n", Question::Parameters),
        expect: Expect::Value(&["parameter 0 unnamed", "variadic 1 unnamed"]),
    },
    Case {
        name: CaseName("the_lists_of_a_struct_and_of_type_parameters_have_none"),
        word: Word::Succeeds,
        input: || {
            (
                "type T struct { a int }\nfunc f[E any]() {}\n",
                Question::Parameters,
            )
        },
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName(
            "a_declaration_has_its_receiver_name_lists_and_body_under_its_function_type",
        ),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc (t T) f(a int) (n int) { return }\n",
                Question::Parts,
            )
        },
        expect: Expect::Value(&[
            "receiver (t T)",
            "name f",
            "parameters (a int)",
            "results (n int)",
            "body { return }",
        ]),
    },
    Case {
        name: CaseName("a_generic_function_has_type_parameters_in_brackets"),
        word: Word::Succeeds,
        input: || ("package p\nfunc g[E any](a E) {}\n", Question::Parts),
        expect: Expect::Value(&[
            "name g",
            "type_parameters [E any]",
            "parameters (a E)",
            "body {}",
        ]),
    },
    Case {
        name: CaseName("a_function_literal_has_parameters_and_results_and_no_name"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nvar g = func(a int) string { return \"\" }\n",
                Question::Parts,
            )
        },
        expect: Expect::Value(&[
            "parameters (a int)",
            "results string",
            "body { return \"\" }",
        ]),
    },
    Case {
        name: CaseName("a_list_is_told_by_what_holds_it"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc (t T) f(a int) int { return 0 }\n",
                Question::Lists,
            )
        },
        expect: Expect::Value(&["Receiver owned", "Parameters owned", "Results owned"]),
    },
    Case {
        name: CaseName("the_list_of_a_struct_belongs_to_no_function"),
        word: Word::Succeeds,
        input: || ("package p\ntype T struct { a int }\n", Question::Lists),
        expect: Expect::Value(&[]),
    },
];

fn lists(source: &'static str) -> Vec<String> {
    asked(source, |tree| {
        of_kind(tree, Kind::Node(GoNode::FieldList))
            .into_iter()
            .filter_map(|list| {
                listing(tree, list)
                    .filter(|found| *found != super::types::Listing::Members)
                    .map(|found| {
                        let owned = callable(tree, list).map_or("alone", |_| "owned");
                        format!("{found:?} {owned}")
                    })
            })
            .collect()
    })
}

fn parts(source: &'static str) -> Vec<String> {
    asked(source, |tree| {
        let Some(function) = of_kind(tree, Kind::Node(GoNode::FuncDecl))
            .into_iter()
            .chain(of_kind(tree, Kind::Node(GoNode::FuncLit)))
            .next()
        else {
            return Vec::new();
        };
        let found = signature(tree, function);
        [
            ("receiver", found.receiver),
            ("name", found.name),
            ("type_parameters", found.type_parameters),
            ("parameters", found.parameters),
            ("results", found.results),
            ("body", found.body),
        ]
        .into_iter()
        .filter_map(|(word, at)| at.map(|place| format!("{word} {}", spans(tree, place))))
        .collect()
    })
}

fn subject((source, question): Input) -> Result<Value, Failure> {
    let leaked = |found: Vec<String>| -> Value {
        let each: Vec<&'static str> = found
            .into_iter()
            .map(|line| &*Box::leak(line.into_boxed_str()))
            .collect();
        Box::leak(each.into_boxed_slice())
    };
    match question {
        Question::Parts => return Ok(leaked(parts(source))),
        Question::Lists => return Ok(leaked(lists(source))),
        Question::Parameters => {}
    }
    let mut file = parsed(source.as_bytes()).map_err(drop)?;
    let elements = std::mem::take(&mut file.elements);
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let found: Vec<String> = languages::positions(&elements)
        .filter(|at| kind(&tree, *at) == Kind::Node(GoNode::Field))
        .flat_map(|at| parameters(&tree, at))
        .map(|each| {
            let name = languages::text_of(&file.symbols, tree.source, each.name)
                .map_or("unnamed", |text| text.0);
            format!("{} {} {}", each.word.0, each.ordinal, name)
        })
        .collect();
    let kept: Vec<&'static str> = found
        .into_iter()
        .map(|each| &*Box::leak(each.into_boxed_str()))
        .collect();
    Ok(&*Box::leak(kept.into_boxed_slice()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
