//! Actual completion: nested provenance, resolved scope and no unrelated carrier parsing.
use super::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::contracts::parse::contract::ParseContract;
use ryo::languages::{self, ByteSlice};

pub(super) fn build(
    parse: &ParseContract,
    source: &[u8],
    elements: &mut languages::Elements,
    symbols: &mut languages::Symbols,
) {
    languages::contract::build(
        parse,
        languages::ParseRequest {
            path: languages::SourcePath("case.pyi"),
            file: languages::FileId(0),
            bytes: ByteSlice(source),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        elements,
        symbols,
    )
    .unwrap();
}
pub(super) fn name(report: &Reading, text: &str) -> bool {
    report.rows.iter().any(
        |row| matches!(*row,[_,Want::Name("name-token"),Want::Name(value),_,_] if *value==text),
    )
}
fn nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens() {
    let source = "Alias = list[\"'T'\"]\n";
    let literals = reading::<LiteralExpressionParseItem>(source);
    assert!(literals.causes.is_empty());
    assert_eq!(outcomes(&literals), ["parsed", "parsed"]);
    assert_eq!(
        literals.rows.iter().map(|row| row[4]).collect::<Vec<_>>(),
        [
            languages::contract::want_span((1, 14), (1, 19)),
            languages::contract::want_span((1, 15), (1, 18))
        ]
    );
    let embedded = reading::<EmbeddedExpressionItem>(source);
    assert_eq!(embedded.rows.len(), 2);
    assert_eq!(
        embedded.rows[1][1], embedded.rows[0][0],
        "second host is the first actual parsed literal, not a duplicate finding"
    );
    assert!(
        name(&reading::<SemanticSyntaxItem>(source), "T"),
        "actual name-token kind is a live positive control"
    );
}
fn values_and_shadowing_stay_negative_beside_live_type_controls() {
    for source in [
        "list = lambda value: value\nAlias = list[\"'Hidden'\"]\n",
        "def f(list):\n    return list[\"'Hidden'\"]\n",
        "Alias = unknown[\"'Hidden'\"]\n",
        "env = {}\nvalue = env[\"'Hidden'\"]\n",
        "value = list[\"'Hidden'\":]\n",
        "value = 'base'[\"'Hidden'\"]\n",
        "from typing import Literal\nAlias = tuple[Literal[\"'Hidden'\"], int]\n",
        "from typing import Annotated\nAlias = Annotated[int, \"'Hidden'\"]\n",
        "from typing import cast\nvalue = cast(object, \"'Hidden'\")\n",
    ] {
        let syntax = reading::<SemanticSyntaxItem>(source);
        assert!(!syntax.rows.is_empty());
        assert!(!name(&syntax, "Hidden"), "{source:?}");
        assert!(
            reading::<EmbeddedExpressionItem>(source).rows.is_empty(),
            "{source:?}"
        );
    }
    for source in [
        "Alias = list[\"'Visible'\"]\n",
        "foo(list[\"'Visible'\"])\n",
        "Alias = list[UnknownMapping[\"'Visible'\"]]\n",
        "from typing import Literal\nAlias = tuple[Literal[\"'Hidden'\"], \"'Visible'\"]\n",
    ] {
        let syntax = reading::<SemanticSyntaxItem>(source);
        assert!(name(&syntax, "Visible"), "{source:?}");
        assert!(
            !name(&syntax, "Hidden"),
            "Literal metadata stays a value inside an outer generic"
        );
    }
}
fn invalid_nested_expression_keeps_outer_success_without_guessed_inner_root() {
    let source = "Alias = list[\"'T +'\"]\n";
    let report = reading::<LiteralExpressionParseItem>(source);
    assert!(report.causes.is_empty());
    assert_eq!(outcomes(&report), ["parsed", "malformed"]);
    assert_eq!(reading::<EmbeddedExpressionItem>(source).rows.len(), 1);
    assert!(!name(&reading::<SemanticSyntaxItem>(source), "T"));
}

/// The cases of this file, one variant each.
#[derive(Clone, Copy)]
enum Body {
    /// The case `nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens`.
    NestedRuntimeAliasHasTwoRealHostsAndLiveSemanticTokens,
    /// The case `values_and_shadowing_stay_negative_beside_live_type_controls`.
    ValuesAndShadowingStayNegativeBesideLiveTypeControls,
    /// The case `invalid_nested_expression_keeps_outer_success_without_guessed_inner_root`.
    InvalidNestedExpressionKeepsOuterSuccessWithoutGuessedInnerRoot,
}

type Input = Body;
type Value = ();
type Failure = ();

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens"),
        word: Word::Succeeds,
        input: || Body::NestedRuntimeAliasHasTwoRealHostsAndLiveSemanticTokens,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("values_and_shadowing_stay_negative_beside_live_type_controls"),
        word: Word::Succeeds,
        input: || Body::ValuesAndShadowingStayNegativeBesideLiveTypeControls,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_nested_expression_keeps_outer_success_without_guessed_inner_root"),
        word: Word::Succeeds,
        input: || Body::InvalidNestedExpressionKeepsOuterSuccessWithoutGuessedInnerRoot,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Body::NestedRuntimeAliasHasTwoRealHostsAndLiveSemanticTokens => {
            nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens()
        }
        Body::ValuesAndShadowingStayNegativeBesideLiveTypeControls => {
            values_and_shadowing_stay_negative_beside_live_type_controls()
        }
        Body::InvalidNestedExpressionKeepsOuterSuccessWithoutGuessedInnerRoot => {
            invalid_nested_expression_keeps_outer_success_without_guessed_inner_root()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
