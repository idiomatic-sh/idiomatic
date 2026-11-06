//! The `fields` service's cases for lists: the role of an edge of the tree under its parent.

use super::fixtures::{Edge, edge, subject};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("generic_param_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<T>() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::GenericParamList),
                0,
            )
        },
        expect: Expect::Value("generic-param-list"),
    },
    Case {
        name: CaseName("generic_arg_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g::<u8>(); }",
                Kind::Node(Node::PathSegment),
                Kind::Node(Node::GenericArgList),
                0,
            )
        },
        expect: Expect::Value("generic-arg-list"),
    },
    Case {
        name: CaseName("param_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f(a: u8) {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::ParamList),
                0,
            )
        },
        expect: Expect::Value("param-list"),
    },
    Case {
        name: CaseName("arg_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g(1); }",
                Kind::Node(Node::CallExpr),
                Kind::Node(Node::ArgList),
                0,
            )
        },
        expect: Expect::Value("arg-list"),
    },
    Case {
        name: CaseName("item_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "mod m { fn f() {} }",
                Kind::Node(Node::Module),
                Kind::Node(Node::ItemList),
                0,
            )
        },
        expect: Expect::Value("item-list"),
    },
    Case {
        name: CaseName("assoc_item_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "impl T { fn f() {} }",
                Kind::Node(Node::Impl),
                Kind::Node(Node::AssocItemList),
                0,
            )
        },
        expect: Expect::Value("assoc-item-list"),
    },
    Case {
        name: CaseName("record_field_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "struct S { a: u8 }",
                Kind::Node(Node::Struct),
                Kind::Node(Node::RecordFieldList),
                0,
            )
        },
        expect: Expect::Value("record-field-list"),
    },
    Case {
        name: CaseName("tuple_field_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "struct S(u8);",
                Kind::Node(Node::Struct),
                Kind::Node(Node::TupleFieldList),
                0,
            )
        },
        expect: Expect::Value("tuple-field-list"),
    },
    Case {
        name: CaseName("variant_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "enum E { A }",
                Kind::Node(Node::Enum),
                Kind::Node(Node::VariantList),
                0,
            )
        },
        expect: Expect::Value("variant-list"),
    },
    Case {
        name: CaseName("match_arm_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { match x { _ => 1 } }",
                Kind::Node(Node::MatchExpr),
                Kind::Node(Node::MatchArmList),
                0,
            )
        },
        expect: Expect::Value("match-arm-list"),
    },
    Case {
        name: CaseName("stmt_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() {}",
                Kind::Node(Node::BlockExpr),
                Kind::Node(Node::StmtList),
                0,
            )
        },
        expect: Expect::Value("stmt-list"),
    },
    Case {
        name: CaseName("use_tree_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "use a::{b};",
                Kind::Node(Node::UseTree),
                Kind::Node(Node::UseTreeList),
                0,
            )
        },
        expect: Expect::Value("use-tree-list"),
    },
    Case {
        name: CaseName("record_expr_field_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { S { a: 1 }; }",
                Kind::Node(Node::RecordExpr),
                Kind::Node(Node::RecordExprFieldList),
                0,
            )
        },
        expect: Expect::Value("record-expr-field-list"),
    },
    Case {
        name: CaseName("record_pat_field_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { let S { a } = s; }",
                Kind::Node(Node::RecordPat),
                Kind::Node(Node::RecordPatFieldList),
                0,
            )
        },
        expect: Expect::Value("record-pat-field-list"),
    },
    Case {
        name: CaseName("items_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() {}",
                Kind::Node(Node::SourceFile),
                Kind::Node(Node::Fn),
                0,
            )
        },
        expect: Expect::Value("items"),
    },
    Case {
        name: CaseName("statements_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { let a = 1; }",
                Kind::Node(Node::StmtList),
                Kind::Node(Node::LetStmt),
                0,
            )
        },
        expect: Expect::Value("statements"),
    },
    Case {
        name: CaseName("fields_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "struct S { a: u8 }",
                Kind::Node(Node::RecordFieldList),
                Kind::Node(Node::RecordField),
                0,
            )
        },
        expect: Expect::Value("fields"),
    },
    Case {
        name: CaseName("variants_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "enum E { A }",
                Kind::Node(Node::VariantList),
                Kind::Node(Node::Variant),
                0,
            )
        },
        expect: Expect::Value("variants"),
    },
    Case {
        name: CaseName("params_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f(a: u8) {}",
                Kind::Node(Node::ParamList),
                Kind::Node(Node::Param),
                0,
            )
        },
        expect: Expect::Value("params"),
    },
    Case {
        name: CaseName("args_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g(1); }",
                Kind::Node(Node::ArgList),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("args"),
    },
    Case {
        name: CaseName("arms_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { match x { _ => 1 } }",
                Kind::Node(Node::MatchArmList),
                Kind::Node(Node::MatchArm),
                0,
            )
        },
        expect: Expect::Value("arms"),
    },
    Case {
        name: CaseName("bounds_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<T: A>() {}",
                Kind::Node(Node::TypeBoundList),
                Kind::Node(Node::TypeBound),
                0,
            )
        },
        expect: Expect::Value("bounds"),
    },
    Case {
        name: CaseName("predicates_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() where T: A {}",
                Kind::Node(Node::WhereClause),
                Kind::Node(Node::WherePred),
                0,
            )
        },
        expect: Expect::Value("predicates"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
