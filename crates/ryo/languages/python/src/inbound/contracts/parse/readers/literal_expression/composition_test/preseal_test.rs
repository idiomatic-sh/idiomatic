//! Actual completion: nested provenance, resolved scope, no unrelated carrier parsing and the single mapping writer.
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
/// The hosts, their outcomes and spans, and the live name token of a string nested in a runtime alias.
fn nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens() -> String {
    let source = "Alias = list[\"'T'\"]\n";
    let literals = reading::<LiteralExpressionParseItem>(source);
    let embedded = reading::<EmbeddedExpressionItem>(source);
    format!(
        "{}; spans {:?}; embedded rows {}; second host is the first parsed literal {}; name token T {}",
        shown(&literals),
        spans(&literals),
        embedded.rows.len(),
        embedded.rows.get(1).map(|row| row[1]) == embedded.rows.first().map(|row| row[0]),
        name(&reading::<SemanticSyntaxItem>(source), "T")
    )
}
/// Per source, whether the hidden string reaches a name token or an embedded expression, and the
/// visible one beside it does.
fn values_and_shadowing_stay_negative_beside_live_type_controls() -> String {
    let hidden = [
        "list = lambda value: value\nAlias = list[\"'Hidden'\"]\n",
        "def f(list):\n    return list[\"'Hidden'\"]\n",
        "Alias = unknown[\"'Hidden'\"]\n",
        "env = {}\nvalue = env[\"'Hidden'\"]\n",
        "value = list[\"'Hidden'\":]\n",
        "value = 'base'[\"'Hidden'\"]\n",
        "from typing import Literal\nAlias = tuple[Literal[\"'Hidden'\"], int]\n",
        "from typing import Annotated\nAlias = Annotated[int, \"'Hidden'\"]\n",
        "from typing import cast\nvalue = cast(object, \"'Hidden'\")\n",
    ]
    .map(|source| {
        let syntax = reading::<SemanticSyntaxItem>(source);
        format!(
            "{source:?}: syntax rows {}, name Hidden {}, embedded rows {}",
            !syntax.rows.is_empty(),
            name(&syntax, "Hidden"),
            reading::<EmbeddedExpressionItem>(source).rows.len()
        )
    });
    let visible = [
        "Alias = list[\"'Visible'\"]\n",
        "foo(list[\"'Visible'\"])\n",
        "Alias = list[UnknownMapping[\"'Visible'\"]]\n",
        "from typing import Literal\nAlias = tuple[Literal[\"'Hidden'\"], \"'Visible'\"]\n",
    ]
    .map(|source| {
        let syntax = reading::<SemanticSyntaxItem>(source);
        format!(
            "{source:?}: name Visible {}, name Hidden {}",
            name(&syntax, "Visible"),
            name(&syntax, "Hidden")
        )
    });
    format!(
        "hidden {}; visible {}",
        hidden.join("; "),
        visible.join("; ")
    )
}
/// The outer and inner outcomes of a malformed nested expression, its embedded rows and whether
/// a name token was guessed.
fn invalid_nested_expression_keeps_outer_success_without_guessed_inner_root() -> String {
    let source = "Alias = list[\"'T +'\"]\n";
    let report = reading::<LiteralExpressionParseItem>(source);
    format!(
        "{}; embedded rows {}; name token T {}",
        shown(&report),
        reading::<EmbeddedExpressionItem>(source).rows.len(),
        name(&reading::<SemanticSyntaxItem>(source), "T")
    )
}

/// Per source, whether the logical text of a unicode or escaped carrier reaches its name token and
/// its two hosts, through the one mapping writer.
fn unicode_and_escaped_carriers_use_the_existing_single_mapping_writer() -> String {
    ["Alias = list[\"'λ'\"]\n", "Alias = list[\"'\\u03bb'\"]\n"]
        .map(|source| {
            let semantic = reading::<SemanticSyntaxItem>(source);
            let embedded = reading::<EmbeddedExpressionItem>(source);
            format!(
                "{source:?}: causes {:?}, name token λ {}, embedded rows {}, second host is the first {}",
                semantic.causes,
                name(&semantic, "λ"),
                embedded.rows.len(),
                embedded.rows.get(1).map(|row| row[1]) == embedded.rows.first().map(|row| row[0])
            )
        })
        .join("; ")
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
    /// The case `unicode_and_escaped_carriers_use_the_existing_single_mapping_writer`.
    UnicodeAndEscapedCarriersUseTheExistingSingleMappingWriter,
}

type Input = Body;
/// What the case observed, rendered.
type Value = &'static str;
type Failure = ();

/// The rows of this file's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens"),
        word: Word::Succeeds,
        input: || Body::NestedRuntimeAliasHasTwoRealHostsAndLiveSemanticTokens,
        expect: Expect::Value(
            "causes []; outcomes [\"parsed\", \"parsed\"]; spans [\"Span(Point { line: 1, column: 14 }, Point { line: 1, column: 19 })\", \"Span(Point { line: 1, column: 15 }, Point { line: 1, column: 18 })\"]; embedded rows 2; second host is the first parsed literal true; name token T true",
        ),
    },
    Case {
        name: CaseName("values_and_shadowing_stay_negative_beside_live_type_controls"),
        word: Word::Succeeds,
        input: || Body::ValuesAndShadowingStayNegativeBesideLiveTypeControls,
        expect: Expect::Value(
            "hidden \"list = lambda value: value\\nAlias = list[\\\"'Hidden'\\\"]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"def f(list):\\n    return list[\\\"'Hidden'\\\"]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"Alias = unknown[\\\"'Hidden'\\\"]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"env = {}\\nvalue = env[\\\"'Hidden'\\\"]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"value = list[\\\"'Hidden'\\\":]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"value = 'base'[\\\"'Hidden'\\\"]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"from typing import Literal\\nAlias = tuple[Literal[\\\"'Hidden'\\\"], int]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"from typing import Annotated\\nAlias = Annotated[int, \\\"'Hidden'\\\"]\\n\": syntax rows true, name Hidden false, embedded rows 0; \"from typing import cast\\nvalue = cast(object, \\\"'Hidden'\\\")\\n\": syntax rows true, name Hidden false, embedded rows 0; visible \"Alias = list[\\\"'Visible'\\\"]\\n\": name Visible true, name Hidden false; \"foo(list[\\\"'Visible'\\\"])\\n\": name Visible true, name Hidden false; \"Alias = list[UnknownMapping[\\\"'Visible'\\\"]]\\n\": name Visible true, name Hidden false; \"from typing import Literal\\nAlias = tuple[Literal[\\\"'Hidden'\\\"], \\\"'Visible'\\\"]\\n\": name Visible true, name Hidden false",
        ),
    },
    Case {
        name: CaseName("invalid_nested_expression_keeps_outer_success_without_guessed_inner_root"),
        word: Word::Succeeds,
        input: || Body::InvalidNestedExpressionKeepsOuterSuccessWithoutGuessedInnerRoot,
        expect: Expect::Value(
            "causes []; outcomes [\"parsed\", \"malformed\"]; embedded rows 1; name token T false",
        ),
    },
    Case {
        name: CaseName("unicode_and_escaped_carriers_use_the_existing_single_mapping_writer"),
        word: Word::Succeeds,
        input: || Body::UnicodeAndEscapedCarriersUseTheExistingSingleMappingWriter,
        expect: Expect::Value(
            "\"Alias = list[\\\"'λ'\\\"]\\n\": causes [], name token λ true, embedded rows 2, second host is the first true; \"Alias = list[\\\"'\\\\u03bb'\\\"]\\n\": causes [], name token λ true, embedded rows 2, second host is the first true",
        ),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Body::NestedRuntimeAliasHasTwoRealHostsAndLiveSemanticTokens => {
            nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens()
        }
        Body::ValuesAndShadowingStayNegativeBesideLiveTypeControls => {
            values_and_shadowing_stay_negative_beside_live_type_controls()
        }
        Body::InvalidNestedExpressionKeepsOuterSuccessWithoutGuessedInnerRoot => {
            invalid_nested_expression_keeps_outer_success_without_guessed_inner_root()
        }
        Body::UnicodeAndEscapedCarriersUseTheExistingSingleMappingWriter => {
            unicode_and_escaped_carriers_use_the_existing_single_mapping_writer()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
