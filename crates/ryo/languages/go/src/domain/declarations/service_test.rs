//! The `declarations` service's cases: the nodes `go/ast` makes of a function, a function type, a list of parameters and a declaration. Each row runs the service over the nodes of a source's tree and says the nodes it finds, in order.

use super::service::{closure, declaration, fields, function, method_element, signature};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::grammar::service::form;
use crate::domain::grammar::types::{Form, Index};
use crate::fixtures::reading::found_by;

/// A source.
type Input = &'static str;
/// The kinds of the nodes the service finds over the nodes of its tree, in order.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_is_a_func_decl_over_a_func_type"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a int) {}\n",
        expect: Expect::Value("func-decl func-type field"),
    },
    Case {
        name: CaseName("a_function_literal_is_a_func_lit_over_a_func_type"),
        word: Word::Succeeds,
        input: || "package p\nvar g = func(a int) {}\n",
        expect: Expect::Value("gen-decl func-lit func-type field"),
    },
    Case {
        name: CaseName("a_result_without_parentheses_is_a_field_list_of_a_field"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() int {}\n",
        expect: Expect::Value("func-decl func-type field-list field"),
    },
    Case {
        name: CaseName("results_in_parentheses_are_one_field_each"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() (n int, err error) {}\n",
        expect: Expect::Value("func-decl func-type field field"),
    },
    Case {
        name: CaseName("a_function_type_is_a_func_type_with_its_results"),
        word: Word::Succeeds,
        input: || "package p\nvar f func(int) string\n",
        expect: Expect::Value("gen-decl func-type field-list field"),
    },
    Case {
        name: CaseName("a_method_of_an_interface_is_a_field_holding_a_func_type"),
        word: Word::Succeeds,
        input: || "package p\ntype I interface {\n\tm(a int) error\n}\n",
        expect: Expect::Value("gen-decl field func-type field-list field field"),
    },
    Case {
        name: CaseName("names_that_share_a_type_are_one_field"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a, b, c int) {}\n",
        expect: Expect::Value("func-decl func-type field"),
    },
    Case {
        name: CaseName("types_alone_find_no_field_for_the_list"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(int, string) {}\n",
        expect: Expect::Value("func-decl func-type"),
    },
    Case {
        name: CaseName("a_parameter_that_takes_the_rest_holds_an_ellipsis"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a int, rest ...string) {}\n",
        expect: Expect::Value("func-decl func-type field field ellipsis"),
    },
    Case {
        name: CaseName("a_declaration_in_a_function_is_in_a_decl_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tvar a int\n}\n",
        expect: Expect::Value("func-decl func-type decl-stmt gen-decl"),
    },
    Case {
        name: CaseName("a_declaration_at_the_top_is_a_gen_decl_alone"),
        word: Word::Succeeds,
        input: || "package p\nvar a int\nimport \"x\"\n",
        expect: Expect::Value("gen-decl gen-decl"),
    },
];

/// The nodes the service finds over each node of the tree of `source`, as the walk asks them of it: the service of a form and no other.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the forms this service finds nodes for; every other form is another service's"
)]
fn subject(source: Input) -> Value {
    Box::leak(
        found_by(source, |tree, pieces| {
            for at in (0..tree.nodes.len()).map(Index) {
                match form(tree, at) {
                    Form::Function | Form::Method => function(tree, pieces, at),
                    Form::Closure => closure(tree, pieces, at),
                    Form::Signature => signature(tree, pieces, at),
                    Form::MethodElement => method_element(tree, pieces, at),
                    Form::Declaration | Form::Import => declaration(tree, pieces, at),
                    Form::Parameters => {
                        let _grouping = fields(tree, pieces, at);
                    }
                    _ => {}
                }
            }
        })
        .into_boxed_str(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
