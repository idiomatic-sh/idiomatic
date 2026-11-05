// one row for each role of the vocabulary, and the check that every role has one: the table is the proof, and splitting it would split that check
//! The `fields` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse. The roles are those the labels of `rust.ungram` give; the adapter of the syntax port holds them to the typed
//! accessors of the library.

use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use std::sync::Arc;

use super::service::{fields, role_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::types::Field;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::trees::tree_of;
use ryo::languages::{self, Position, Tree};

/// An edge of the tree of some source: the nth edge, in source order, from a node of a kind to an element of a kind.
#[derive(Clone, Copy)]
struct Edge {
    source: &'static str,
    parent: Kind,
    child: Kind,
    nth: usize,
}

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

const fn edge(source: &'static str, parent: Kind, child: Kind, nth: usize) -> Edge {
    Edge {
        source,
        parent,
        child,
        nth,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Token(Token::FnKw),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("fn f() {}", Kind::Node(Node::Fn), Kind::Node(Node::Name), 0),
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("name_ref_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g(); }",
                Kind::Node(Node::PathSegment),
                Kind::Node(Node::NameRef),
                0,
            )
        },
        expect: Expect::Value("name-ref"),
    },
    Case {
        name: CaseName("path_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g(); }",
                Kind::Node(Node::PathExpr),
                Kind::Node(Node::Path),
                0,
            )
        },
        expect: Expect::Value("path"),
    },
    Case {
        name: CaseName("segment_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a::b(); }",
                Kind::Node(Node::Path),
                Kind::Node(Node::PathSegment),
                0,
            )
        },
        expect: Expect::Value("segment"),
    },
    Case {
        name: CaseName("qualifier_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a::b(); }",
                Kind::Node(Node::Path),
                Kind::Node(Node::Path),
                0,
            )
        },
        expect: Expect::Value("qualifier"),
    },
    Case {
        name: CaseName("visibility_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "pub fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::Visibility),
                0,
            )
        },
        expect: Expect::Value("visibility"),
    },
    Case {
        name: CaseName("attrs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "#[a] fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::Attr),
                0,
            )
        },
        expect: Expect::Value("attrs"),
    },
    Case {
        name: CaseName("lifetime_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<'a>() {}",
                Kind::Node(Node::LifetimeParam),
                Kind::Node(Node::Lifetime),
                0,
            )
        },
        expect: Expect::Value("lifetime"),
    },
    Case {
        name: CaseName("label_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { 'a: loop {} }",
                Kind::Node(Node::LoopExpr),
                Kind::Node(Node::Label),
                0,
            )
        },
        expect: Expect::Value("label"),
    },
    Case {
        name: CaseName("abi_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "extern \"C\" fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::Abi),
                0,
            )
        },
        expect: Expect::Value("abi"),
    },
    Case {
        name: CaseName("rename_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "use a as b;",
                Kind::Node(Node::UseTree),
                Kind::Node(Node::Rename),
                0,
            )
        },
        expect: Expect::Value("rename"),
    },
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
        name: CaseName("where_clause_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() where T: A {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::WhereClause),
                0,
            )
        },
        expect: Expect::Value("where-clause"),
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
        name: CaseName("ret_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() -> u8 {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::RetType),
                0,
            )
        },
        expect: Expect::Value("ret-type"),
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
        name: CaseName("use_tree_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "use a;",
                Kind::Node(Node::Use),
                Kind::Node(Node::UseTree),
                0,
            )
        },
        expect: Expect::Value("use-tree"),
    },
    Case {
        name: CaseName("let_else_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { let a = b else { return }; }",
                Kind::Node(Node::LetStmt),
                Kind::Node(Node::LetElse),
                0,
            )
        },
        expect: Expect::Value("let-else"),
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
        name: CaseName("tail_expr_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { 1 }",
                Kind::Node(Node::StmtList),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("tail-expr"),
    },
    Case {
        name: CaseName("expr_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { x.y; }",
                Kind::Node(Node::FieldExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("expr"),
    },
    Case {
        name: CaseName("pat_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f(a: u8) {}",
                Kind::Node(Node::Param),
                Kind::Node(Node::IdentPat),
                0,
            )
        },
        expect: Expect::Value("pat"),
    },
    Case {
        name: CaseName("ty_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f(a: u8) {}",
                Kind::Node(Node::Param),
                Kind::Node(Node::PathType),
                0,
            )
        },
        expect: Expect::Value("ty"),
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
    Case {
        name: CaseName("condition_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { if a {} }",
                Kind::Node(Node::IfExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("condition"),
    },
    Case {
        name: CaseName("then_branch_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { if a {} }",
                Kind::Node(Node::IfExpr),
                Kind::Node(Node::BlockExpr),
                0,
            )
        },
        expect: Expect::Value("then-branch"),
    },
    Case {
        name: CaseName("else_branch_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { if a {} else {} }",
                Kind::Node(Node::IfExpr),
                Kind::Node(Node::BlockExpr),
                1,
            )
        },
        expect: Expect::Value("else-branch"),
    },
    Case {
        name: CaseName("loop_body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { loop {} }",
                Kind::Node(Node::LoopExpr),
                Kind::Node(Node::BlockExpr),
                0,
            )
        },
        expect: Expect::Value("loop-body"),
    },
    Case {
        name: CaseName("iterable_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { for a in b {} }",
                Kind::Node(Node::ForExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("iterable"),
    },
    Case {
        name: CaseName("receiver_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a.b(); }",
                Kind::Node(Node::MethodCallExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("receiver"),
    },
    Case {
        name: CaseName("lhs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a + b; }",
                Kind::Node(Node::BinExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("lhs"),
    },
    Case {
        name: CaseName("rhs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a + b; }",
                Kind::Node(Node::BinExpr),
                Kind::Node(Node::PathExpr),
                1,
            )
        },
        expect: Expect::Value("rhs"),
    },
    Case {
        name: CaseName("op_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a + b; }",
                Kind::Node(Node::BinExpr),
                Kind::Token(Token::Plus),
                0,
            )
        },
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("base_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a[b]; }",
                Kind::Node(Node::IndexExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("base"),
    },
    Case {
        name: CaseName("index_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a[b]; }",
                Kind::Node(Node::IndexExpr),
                Kind::Node(Node::PathExpr),
                1,
            )
        },
        expect: Expect::Value("index"),
    },
    Case {
        name: CaseName("start_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a..b; }",
                Kind::Node(Node::RangeExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("start"),
    },
    Case {
        name: CaseName("end_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a..b; }",
                Kind::Node(Node::RangeExpr),
                Kind::Node(Node::PathExpr),
                1,
            )
        },
        expect: Expect::Value("end"),
    },
    Case {
        name: CaseName("spread_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { S { ..a }; }",
                Kind::Node(Node::RecordExprFieldList),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("spread"),
    },
    Case {
        name: CaseName("initializer_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { let a = 1; }",
                Kind::Node(Node::LetStmt),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("initializer"),
    },
    Case {
        name: CaseName("guard_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { match x { _ if y => 1 } }",
                Kind::Node(Node::MatchArm),
                Kind::Node(Node::MatchGuard),
                0,
            )
        },
        expect: Expect::Value("guard"),
    },
    Case {
        name: CaseName("body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::BlockExpr),
                0,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("self_ty_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "impl A for B {}",
                Kind::Node(Node::Impl),
                Kind::Node(Node::PathType),
                1,
            )
        },
        expect: Expect::Value("self-ty"),
    },
    Case {
        name: CaseName("trait_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "impl A for B {}",
                Kind::Node(Node::Impl),
                Kind::Node(Node::PathType),
                0,
            )
        },
        expect: Expect::Value("trait"),
    },
    Case {
        name: CaseName("default_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<T = u8>() {}",
                Kind::Node(Node::TypeParam),
                Kind::Node(Node::PathType),
                0,
            )
        },
        expect: Expect::Value("default-type"),
    },
    Case {
        name: CaseName("default_val_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<const N: usize = 1>() {}",
                Kind::Node(Node::ConstParam),
                Kind::Node(Node::ConstArg),
                0,
            )
        },
        expect: Expect::Value("default-val"),
    },
    Case {
        name: CaseName("keyword_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "#[cfg(unix)] fn f() {}",
                Kind::Node(Node::CfgAtom),
                Kind::Token(Token::Ident),
                0,
            )
        },
        expect: Expect::Value("keyword"),
    },
];

/// The role the edge plays in the table the parse builds of its source.
fn subject(given: Input) -> Result<Value, Failure> {
    let language = ParseContract::new(
        Arc::new(SyntaxFake(
            tree_of(given.source).expect("a tree is written for the source"),
        )),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("src/lib.rs"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(given.source.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(given.source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == given.parent && kind(&tree, *at) == given.child
        })
        .nth(given.nth)
        .ok_or(())?;
    assert_eq!(role_of(&tree, found), languages::field_at(&elements, found));
    let name = languages::text_of(
        &symbols,
        tree.source,
        languages::field_name_at(&elements, found),
    )
    .ok_or(())?;
    Ok(Box::leak(name.0.to_owned().into_boxed_str()))
}

/// Every role of the vocabulary has a row of its own.
fn every_role_has_a_row() -> usize {
    fields()
        .filter(|field| {
            let name = super::service::field_name(*field).0;
            !CASES
                .iter()
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .count()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(every_role_has_a_row(), 0, "{:?}", Field::None);
}
