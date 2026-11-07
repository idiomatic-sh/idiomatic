//! The `expressions` service's cases: the nodes `go/ast` makes of a union, an instantiation, a call, a conversion, a unary expression, an interface, a field and a composite literal. Each row runs the service over the nodes of a source's tree and says the nodes it finds, in order.

use super::service::{
    call, elements, field_declaration, generic_type, implicit_array, instantiation, interface,
    unary, union, variadic_parameter,
};
use super::types::Conversion;
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
        name: CaseName("a_union_in_an_interface_is_a_field_of_binary_exprs"),
        word: Word::Succeeds,
        input: || "package p\ntype N interface {\n\t~int | ~float64 | string\n}\n",
        expect: Expect::Value("interface-type field-list field binary-expr binary-expr"),
    },
    Case {
        name: CaseName("a_type_with_one_argument_is_an_index_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a Pair[int]\n",
        expect: Expect::Value("index-expr"),
    },
    Case {
        name: CaseName("a_type_with_several_arguments_is_an_index_list_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a Pair[int, string]\n",
        expect: Expect::Value("index-list-expr"),
    },
    Case {
        name: CaseName("a_call_with_type_arguments_has_an_index_list_expr_for_its_callee"),
        word: Word::Succeeds,
        input: || "package p\nvar a = m[int, string](x)\n",
        expect: Expect::Value("call-expr index-list-expr"),
    },
    Case {
        name: CaseName("a_conversion_to_a_pointer_is_the_star_of_the_conversion"),
        word: Word::Succeeds,
        input: || "package p\nvar a = *(*[8]byte)(s)\n",
        expect: Expect::Value("star-expr call-expr"),
    },
    Case {
        name: CaseName("a_dereference_is_a_star_expr_and_any_other_unary_a_unary_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a = *p\nvar b = -q\nvar c = !r\n",
        expect: Expect::Value("star-expr unary-expr unary-expr"),
    },
    Case {
        name: CaseName("an_interface_type_holds_a_field_list"),
        word: Word::Succeeds,
        input: || "package p\nvar a interface{ m() }\n",
        expect: Expect::Value("interface-type field-list"),
    },
    Case {
        name: CaseName("an_embedded_pointer_field_is_a_field_holding_a_star_expr"),
        word: Word::Succeeds,
        input: || "package p\ntype S struct {\n\t*Base\n\tname string\n}\n",
        expect: Expect::Value("field star-expr field"),
    },
    Case {
        name: CaseName("a_parameter_that_takes_the_rest_is_a_field_holding_an_ellipsis"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(rest ...string) {}\n",
        expect: Expect::Value("field ellipsis"),
    },
    Case {
        name: CaseName("an_array_of_implicit_length_has_an_ellipsis_for_its_length"),
        word: Word::Succeeds,
        input: || "package p\nvar a = [...]int{1, 2}\n",
        expect: Expect::Value("array-type ellipsis"),
    },
    Case {
        name: CaseName("the_elements_of_a_literal_in_a_literal_are_composite_literals"),
        word: Word::Succeeds,
        input: || "package p\nvar a = [][]int{{1}, {2}}\n",
        expect: Expect::Value("composite-lit composite-lit"),
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
                    Form::Union => union(tree, pieces, at),
                    Form::GenericType => generic_type(tree, pieces, at),
                    Form::Instantiation => instantiation(tree, pieces, at),
                    Form::Call => call(tree, pieces, at, Conversion::Call),
                    Form::Conversion => call(tree, pieces, at, Conversion::Written),
                    Form::Unary => unary(tree, pieces, at),
                    Form::Interface => interface(tree, pieces, at),
                    Form::FieldDeclaration => field_declaration(tree, pieces, at),
                    Form::Variadic => variadic_parameter(tree, pieces, at),
                    Form::ImplicitArray => implicit_array(tree, pieces, at),
                    Form::Elements => elements(tree, pieces, at),
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
