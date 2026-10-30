//! The `shapes` service's cases: the nodes of `go/ast` the library's nodes are, whole, with the nodes the other services of the domain find. Each row finds the nodes of a source's tree, in order.

use super::service::find;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::pieces::service::arrange;
use crate::fixtures::reading::found_by;

/// A source.
type Input = &'static str;
/// The kinds of the nodes found, in order.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_is_an_ident"),
        word: Word::Succeeds,
        input: || "package p\nvar a = b\n",
        expect: Expect::Value("ident gen-decl value-spec ident ident"),
    },
    Case {
        name: CaseName("a_literal_is_a_basic_lit"),
        word: Word::Succeeds,
        input: || "package p\nvar a = 1 + 2.5 + 'c' + \"s\" + 1i\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident binary-expr binary-expr binary-expr binary-expr basic-lit basic-lit basic-lit basic-lit basic-lit",
        ),
    },
    Case {
        name: CaseName("a_selection_is_a_selector_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a = x.y.z\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident selector-expr selector-expr ident ident ident",
        ),
    },
    Case {
        name: CaseName("the_dot_of_a_dot_import_is_an_ident"),
        word: Word::Succeeds,
        input: || "package p\nimport . \"m\"\n",
        expect: Expect::Value("ident gen-decl import-spec ident basic-lit"),
    },
    Case {
        name: CaseName("a_composite_literal_with_keys_has_key_value_exprs"),
        word: Word::Succeeds,
        input: || "package p\nvar a = T{k: v}\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident composite-lit ident key-value-expr ident ident",
        ),
    },
    Case {
        name: CaseName("the_blocks_of_a_function_are_block_stmts"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tif x {\n\t} else {\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt if-stmt ident block-stmt block-stmt",
        ),
    },
    Case {
        name: CaseName("text_the_library_could_not_read_has_no_node_of_its_own"),
        word: Word::Succeeds,
        input: || "package p\n@\n",
        expect: Expect::Value("ident"),
    },
    Case {
        name: CaseName("a_function_is_a_func_decl_over_a_func_type"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a int) {}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list field ident ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_function_literal_is_a_func_lit_over_a_func_type"),
        word: Word::Succeeds,
        input: || "package p\nvar g = func(a int) {}\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident func-lit func-type field-list field ident ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_result_without_parentheses_is_a_field_list_of_a_field"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() int {}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list field-list field ident block-stmt",
        ),
    },
    Case {
        name: CaseName("results_in_parentheses_are_one_field_list"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() (n int, err error) {}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list field-list field ident ident field ident ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_function_type_is_a_func_type_with_its_results"),
        word: Word::Succeeds,
        input: || "package p\nvar f func(int) string\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident func-type field-list field ident field-list field ident",
        ),
    },
    Case {
        name: CaseName("a_method_of_an_interface_is_a_field_holding_a_func_type"),
        word: Word::Succeeds,
        input: || "package p\ntype I interface {\n\tm(a int) error\n}\n",
        expect: Expect::Value(
            "ident gen-decl type-spec ident interface-type field-list field ident func-type field-list field ident ident field-list field ident",
        ),
    },
    Case {
        name: CaseName("names_that_share_a_type_are_one_field"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a, b, c int) {}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list field ident ident ident ident block-stmt",
        ),
    },
    Case {
        name: CaseName("types_alone_are_a_field_each"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(int, string) {}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list field ident field ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_parameter_that_takes_the_rest_holds_an_ellipsis"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a int, rest ...string) {}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list field ident ident field ident ellipsis ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_declaration_in_a_function_is_in_a_decl_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tvar a int\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt decl-stmt gen-decl value-spec ident ident",
        ),
    },
    Case {
        name: CaseName("a_declaration_at_the_top_is_a_gen_decl_alone"),
        word: Word::Succeeds,
        input: || "package p\nvar a int\nimport \"x\"\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident ident gen-decl import-spec basic-lit",
        ),
    },
    Case {
        name: CaseName("a_union_in_an_interface_is_a_field_of_binary_exprs"),
        word: Word::Succeeds,
        input: || "package p\ntype N interface {\n\t~int | ~float64 | string\n}\n",
        expect: Expect::Value(
            "ident gen-decl type-spec ident interface-type field-list field binary-expr binary-expr unary-expr ident unary-expr ident ident",
        ),
    },
    Case {
        name: CaseName("a_type_with_one_argument_is_an_index_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a Pair[int]\n",
        expect: Expect::Value("ident gen-decl value-spec ident index-expr ident ident"),
    },
    Case {
        name: CaseName("a_type_with_several_arguments_is_an_index_list_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a Pair[int, string]\n",
        expect: Expect::Value("ident gen-decl value-spec ident index-list-expr ident ident ident"),
    },
    Case {
        name: CaseName("a_call_with_type_arguments_has_an_index_list_expr_for_its_callee"),
        word: Word::Succeeds,
        input: || "package p\nvar a = m[int, string](x)\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident call-expr index-list-expr ident ident ident ident",
        ),
    },
    Case {
        name: CaseName("a_conversion_to_a_pointer_is_the_star_of_the_conversion"),
        word: Word::Succeeds,
        input: || "package p\nvar a = *(*[8]byte)(s)\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident star-expr call-expr paren-expr star-expr array-type basic-lit ident ident",
        ),
    },
    Case {
        name: CaseName("a_dereference_is_a_star_expr_and_any_other_unary_a_unary_expr"),
        word: Word::Succeeds,
        input: || "package p\nvar a = *p\nvar b = -q\nvar c = !r\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident star-expr ident gen-decl value-spec ident unary-expr ident gen-decl value-spec ident unary-expr ident",
        ),
    },
    Case {
        name: CaseName("an_interface_type_holds_a_field_list"),
        word: Word::Succeeds,
        input: || "package p\nvar a interface{ m() }\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident interface-type field-list field ident func-type field-list",
        ),
    },
    Case {
        name: CaseName("an_embedded_pointer_field_is_a_field_holding_a_star_expr"),
        word: Word::Succeeds,
        input: || "package p\ntype S struct {\n\t*Base\n\tname string\n}\n",
        expect: Expect::Value(
            "ident gen-decl type-spec ident struct-type field-list field star-expr ident field ident ident",
        ),
    },
    Case {
        name: CaseName("an_array_of_implicit_length_has_an_ellipsis_for_its_length"),
        word: Word::Succeeds,
        input: || "package p\nvar a = [...]int{1, 2}\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident composite-lit array-type ellipsis ident basic-lit basic-lit",
        ),
    },
    Case {
        name: CaseName("the_elements_of_a_literal_in_a_literal_are_composite_list"),
        word: Word::Succeeds,
        input: || "package p\nvar a = [][]int{{1}, {2}}\n",
        expect: Expect::Value(
            "ident gen-decl value-spec ident composite-lit array-type array-type ident composite-lit basic-lit composite-lit basic-lit",
        ),
    },
    Case {
        name: CaseName("a_switch_holds_its_clauses_in_a_block"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tswitch x {\n\tcase 1:\n\tdefault:\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt switch-stmt ident block-stmt case-clause basic-lit case-clause",
        ),
    },
    Case {
        name: CaseName("a_select_holds_its_clauses_in_a_block_and_its_default_is_a_comm_clause"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tselect {\n\tcase <-c:\n\tdefault:\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt select-stmt block-stmt comm-clause expr-stmt unary-expr ident comm-clause",
        ),
    },
    Case {
        name: CaseName("a_type_switch_has_the_assignment_and_the_assertion"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tswitch t := x.(type) {\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt type-switch-stmt assign-stmt ident type-assert-expr ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_type_switch_with_no_name_has_an_expression_statement"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tswitch x.(type) {\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt type-switch-stmt expr-stmt type-assert-expr ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_loop_is_a_for_stmt_and_a_range_is_a_range_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tfor i := 0; i < 1; i++ {\n\t}\n\tfor range x {\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt for-stmt assign-stmt ident basic-lit binary-expr ident basic-lit inc-dec-stmt ident block-stmt range-stmt ident block-stmt",
        ),
    },
    Case {
        name: CaseName("a_receive_that_assigns_is_an_assign_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tselect {\n\tcase v := <-c:\n\tcase <-d:\n\t}\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt select-stmt block-stmt comm-clause assign-stmt ident unary-expr ident comm-clause expr-stmt unary-expr ident",
        ),
    },
    Case {
        name: CaseName("a_label_with_no_statement_has_an_empty_statement"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\nL:\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt labeled-stmt ident empty-stmt",
        ),
    },
    Case {
        name: CaseName("a_label_holds_the_statement_after_it"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\nL:\n\tx++\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt labeled-stmt ident inc-dec-stmt ident",
        ),
    },
    Case {
        name: CaseName("a_jump_is_a_branch_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tfor {\n\t\tbreak\n\t\tcontinue\n\t}\n\tgoto L\n}\n",
        expect: Expect::Value(
            "ident func-decl func-type ident field-list block-stmt for-stmt block-stmt branch-stmt branch-stmt branch-stmt ident",
        ),
    },
];

fn subject(source: Input) -> Value {
    Box::leak(
        found_by(source, |tree, pieces| {
            find(tree, pieces);
            arrange(pieces);
        })
        .into_boxed_str(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
