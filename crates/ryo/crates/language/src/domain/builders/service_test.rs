//! The `builders` service's cases, on values alone.

use super::service::{
    diagnose, diagnose_with_metadata, enter_implied_node, enter_node, leave_node, node_property,
    node_property_fmt, table_builder, token,
};
use crate::domain::completion::service::intern_decoded;
use crate::domain::decoded::service::property_at;
use crate::domain::diagnostics::constants::UNSPECIFIED_CODE;
use crate::domain::diagnostics::types::{
    DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, SyntaxCompleteness, TreeImpact,
};
use crate::domain::elements::service::{
    diagnostic_count, diagnostics, end_at, engine, has_incomplete_syntax, origin_at, positions,
    source_order, start_at, text_at,
};
use crate::domain::elements::types::{Elements, SourceOrder};
use crate::domain::symbols::service::text_of;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::elements::types::ElementOrigin;
use ryo_relations::domain::relations::types::PropertyId;
use ryo_relations::domain::relations::types::{KindBits, Offset, Position, ReportText};
use ryo_relations::domain::terms::types::NameText;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

struct RefusingScalar;
impl std::fmt::Display for RefusingScalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("must roll back")?;
        Err(std::fmt::Error)
    }
}
fn formatted_properties_are_atomic_and_reuse_storage() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    node_property_fmt(
        &mut builder,
        PropertyId(2),
        format_args!("{}:{}", 17_u8, 23_u8),
    )
    .expect("memory format");
    assert_eq!(
        node_property_fmt(
            &mut builder,
            PropertyId(2),
            format_args!("{RefusingScalar}"),
        )
        .expect_err("the scalar refuses to format"),
        std::fmt::Error
    );
    node_property_fmt(&mut builder, PropertyId(3), format_args!("{}", 42_u8))
        .expect("next memory format");
    leave_node(&mut builder, Offset(1));
    usize::from(
        property_at(&elements, Position(0), PropertyId(2)) == Some(NameText("17:23"))
            && property_at(&elements, Position(0), PropertyId(3)) == Some(NameText("42"))
            && engine(&elements).decoded_text.0.as_slice() == b"17:2342",
    )
}
fn implied_nodes_keep_properties_without_claiming_bytes() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = ByteSlice(b"<p>&amp;");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    enter_implied_node(&mut builder, KindBits(2));
    node_property(&mut builder, PropertyId(2), NameText("html"));
    node_property(&mut builder, PropertyId(0), NameText("decoded &"));
    leave_node(&mut builder, Offset(8));
    token(&mut builder, KindBits(3), Offset(0), Offset(3));
    token(&mut builder, KindBits(3), Offset(3), Offset(8));
    leave_node(&mut builder, Offset(8));
    intern_decoded(&mut elements, &mut symbols, source);
    usize::from(
        start_at(&elements, Position(1)) == NO_SPAN.start
            && end_at(&elements, Position(1)) == NO_SPAN.end
            && origin_at(&elements, Position(1)) == ElementOrigin::Implied
            && source_order(&elements) == SourceOrder::Reordered
            && property_at(&elements, Position(1), PropertyId(2)) == Some(NameText("html"))
            && text_of(&symbols, source, text_at(&elements, Position(1)))
                == Some(NameText("decoded &"))
            && text_of(&symbols, source, text_at(&elements, Position(2))) == Some(NameText("<p>"))
            && positions(&elements).count() == 4,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("failed scalar formatter preserves old property and retained bytes"),
        word: Word::Succeeds,
        input: || formatted_properties_are_atomic_and_reuse_storage,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("implied_nodes_keep_properties_without_claiming_bytes"),
        word: Word::Succeeds,
        input: || implied_nodes_keep_properties_without_claiming_bytes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("diagnostics_retain_metadata_and_reset_between_sources"),
        word: Word::Succeeds,
        input: || diagnostics_retain_metadata_and_reset_between_sources,
        expect: Expect::Value(1),
    },
];

/// Diagnostics retain typed metadata and clear completely before the next source uses the table.
fn diagnostics_retain_metadata_and_reset_between_sources() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
    diagnose(
        &mut builder,
        ReportText("old provider"),
        Offset(0),
        Offset(1),
    );
    diagnose_with_metadata(
        &mut builder,
        DiagnosticMetadata {
            phase: DiagnosticPhase::Validation,
            impact: TreeImpact::Preserved,
            code: DiagnosticCode("fixture.semantic"),
        },
        ReportText("complete tree"),
        Offset(0),
        Offset(1),
    );
    let before = diagnostics(&elements).collect::<Vec<_>>();
    let retained = before.len() == 2
        && before[0].phase == DiagnosticPhase::Parse
        && before[0].impact == TreeImpact::Incomplete
        && before[0].code == UNSPECIFIED_CODE
        && before[1].phase == DiagnosticPhase::Validation
        && before[1].impact == TreeImpact::Preserved
        && before[1].code.0 == "fixture.semantic"
        && has_incomplete_syntax(&elements) == SyntaxCompleteness::Incomplete;

    let (mut next_symbols, next_source) = (Symbols::default(), ByteSlice(b"valid"));
    let _next_builder = table_builder(&mut elements, &mut next_symbols, next_source);
    let cleared = has_incomplete_syntax(&elements) == SyntaxCompleteness::Complete
        && diagnostic_count(&elements).0 == 0
        && diagnostics(&elements).next().is_none();

    let mut validation_builder = table_builder(&mut elements, &mut next_symbols, next_source);
    diagnose_with_metadata(
        &mut validation_builder,
        DiagnosticMetadata {
            phase: DiagnosticPhase::Validation,
            impact: TreeImpact::Preserved,
            code: DiagnosticCode("fixture.validation"),
        },
        ReportText("still complete"),
        Offset(0),
        Offset(5),
    );
    usize::from(
        retained
            && cleared
            && has_incomplete_syntax(&elements) == SyntaxCompleteness::Complete
            && diagnostic_count(&elements).0 == 1
            && diagnostics(&elements).next().is_some_and(|diagnostic| {
                diagnostic.code.0 == "fixture.validation"
                    && diagnostic.phase == DiagnosticPhase::Validation
                    && diagnostic.impact == TreeImpact::Preserved
            }),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
