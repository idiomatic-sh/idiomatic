//! Embedded origins join to actual expression nodes, even when source trivia precedes them.
use super::super::fixtures::contract::check;
use super::item::EmbeddedExpressionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::{
    semantic_child::item::SemanticChildItem, semantic_syntax::item::SemanticSyntaxItem,
};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Want, want_name, want_node, want_span};
type Input = &'static str;
type Value = (Want, Want, Want, bool, bool);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordinary name root remains its exact expression"),
        word: Word::Succeeds,
        input: || "x: \"T\"\n",
        expect: Expect::Value((
            want_node(10),
            want_name("name"),
            want_span((1, 5), (1, 6)),
            true,
            true,
        )),
    },
    Case {
        name: CaseName("parenthesis before a name is not an embedded root"),
        word: Word::Succeeds,
        input: || "x: \"(T)\"\n",
        expect: Expect::Value((
            want_node(10),
            want_name("name"),
            want_span((1, 6), (1, 7)),
            true,
            true,
        )),
    },
    Case {
        name: CaseName("parenthesized union root joins to its expression children"),
        word: Word::Succeeds,
        input: || "x: \"(A | B)\"\n",
        expect: Expect::Value((
            want_node(10),
            want_name("bin-op"),
            want_span((1, 6), (1, 11)),
            true,
            true,
        )),
    },
    Case {
        name: CaseName("leading newline in a triple quoted annotation is not a root"),
        word: Word::Succeeds,
        input: || "x: \"\"\"\nT\n\"\"\"\n",
        expect: Expect::Value((
            want_node(10),
            want_name("name"),
            want_span((2, 1), (2, 2)),
            true,
            true,
        )),
    },
    Case {
        name: CaseName("nested parentheses and multiline expression retain exact AST span"),
        word: Word::Succeeds,
        input: || "x: \"\"\"\n((A |\n B))\n\"\"\"\n",
        expect: Expect::Value((
            want_node(10),
            want_name("bin-op"),
            want_span((2, 3), (3, 3)),
            true,
            true,
        )),
    },
];
fn subject(source: Input) -> Result<Value, Failure> {
    let origins = reading::<EmbeddedExpressionItem>(source);
    let syntax = reading::<SemanticSyntaxItem>(source);
    let children = reading::<SemanticChildItem>(source);
    assert!(origins.causes.is_empty() && syntax.causes.is_empty() && children.causes.is_empty());
    assert_eq!(origins.rows.len(), 1);
    let origin = origins.rows.first().ok_or(())?;
    let root = *origin.first().ok_or(())?;
    let host = *origin.get(1).ok_or(())?;
    let span = *origin.get(5).ok_or(())?;
    let expression = syntax
        .rows
        .iter()
        .find(|row| row.first() == Some(&root))
        .ok_or(())?;
    let kind = *expression.get(1).ok_or(())?;
    let attached = children
        .rows
        .iter()
        .any(|row| row.first() == Some(&host) && row.get(1) == Some(&root));
    let has_children = children.rows.iter().any(|row| row.first() == Some(&root));
    Ok((
        root,
        kind,
        span,
        attached && has_children,
        expression.get(4) == Some(&span),
    ))
}
#[test]
fn succeeds() {
    check::<EmbeddedExpressionItem>();
    run(CASES, Word::Succeeds, subject);
    run(CHECKS, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

fn semantic_child_ordinals_are_global_across_call_argument_fields() {
    let children = reading::<SemanticChildItem>("call(first, key=second)\n");
    assert!(children.causes.is_empty());
    let argument = children.rows.iter().find(|row| {
        row.get(3) == Some(&Want::Name("args")) && row.get(2) == Some(&Want::Number(1))
    });
    let keyword = children.rows.iter().find(|row| {
        row.get(3) == Some(&Want::Name("keywords")) && row.get(2) == Some(&Want::Number(4))
    });
    assert!(argument.is_some(), "the args edge has global ordinal one");
    assert!(
        keyword.is_some(),
        "the keyword edge has global ordinal four"
    );
    assert_eq!(
        argument.and_then(|row| row.first()),
        keyword.and_then(|row| row.first())
    );
}

/// Each check below states its own source and asserts its own outcome.
const CHECKS: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("semantic_child_ordinals_are_global_across_call_argument_fields"),
        word: Word::Succeeds,
        input: || semantic_child_ordinals_are_global_across_call_argument_fields,
        expect: Expect::Value(()),
    },
];
