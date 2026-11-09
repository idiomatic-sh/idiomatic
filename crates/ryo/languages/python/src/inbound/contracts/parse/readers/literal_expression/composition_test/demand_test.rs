//! Actual completion under repeated demands: native ids, one model preparation, zero parses and the single mapping writer.
use super::preseal_test::{build, name};
use super::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::{
    inbound::contracts::parse::contract::ParseContract,
    outbound::{
        notes::factory::postcard,
        syntax::{
            fixtures::library::observed,
            interface::SyntaxPort as _,
            types::{PythonSourceKind, SourceText},
        },
    },
    settings::{PythonParserSettings, PythonTargetVersion},
};
use ryo::languages::{self, ByteSlice};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Reads `source` as a stub with `wanted` alone and answers how many annotation carriers the syntax adapter parsed and how many
/// embedded expressions the table holds.
fn read_with_demand(source: &[u8], wanted: languages::Relation) -> (usize, usize) {
    let settings = PythonParserSettings::new(PythonTargetVersion::Py310);
    let count = Arc::new(AtomicUsize::new(0));
    let parse = ParseContract::new(
        settings,
        Arc::new(observed(settings, Arc::clone(&count))),
        Arc::new(postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let mut out = languages::contract::Collect::default();
    languages::contract::read(
        &parse,
        languages::ParseRequest {
            path: languages::SourcePath("case.pyi"),
            file: languages::FileId(0),
            bytes: ByteSlice(source),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
        &[wanted],
        &mut out,
    )
    .unwrap();
    (
        count.load(Ordering::Relaxed),
        languages::embedded_expressions(&elements).count(),
    )
}

/// A demand whose readers never look at expanded elements leaves the annotation carriers unparsed: completion classifies no
/// carrier, so it asks nothing of the binding model. The negative control is the same file read for its embedded expressions.
fn a_demand_that_reads_no_expanded_element_expands_no_annotation() {
    let source = b"Alias = list[\"'T'\"]\n";
    assert_eq!(
        read_with_demand(source, languages::Relation::Comment),
        (0, 0),
        "no wanted reader reads what completion adds"
    );
    // The outer annotation and the string nested in it: one carrier parse, two embedded expressions (the preseal row
    // `nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens` reads the same two).
    assert_eq!(
        read_with_demand(source, languages::Relation::EmbeddedExpression),
        (1, 2),
        "a reader of the embedded expressions has the carrier expanded"
    );
}

fn native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill() {
    let source = b"Alias = list[\"'T'\"]\n";
    let settings = PythonParserSettings::new(PythonTargetVersion::Py310);
    let count = Arc::new(AtomicUsize::new(0));
    let syntax = observed(settings, Arc::clone(&count));
    let (mut native, mut native_symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    syntax.read(
        SourceText(source),
        PythonSourceKind::Stub,
        &mut languages::table_builder(&mut native, &mut native_symbols, ByteSlice(source)),
    );
    let original = languages::positions(&native)
        .map(|at| {
            (
                at,
                languages::kind_at(&native, at),
                languages::start_at(&native, at),
                languages::end_at(&native, at),
                languages::parent_at(&native, at),
            )
        })
        .collect::<Vec<_>>();
    let parse = ParseContract::new(settings, Arc::new(syntax), Arc::new(postcard()));
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    build(&parse, source, &mut elements, &mut symbols);
    assert_eq!(parse.model_preparations.load(Ordering::Relaxed), 1);
    assert_eq!(count.load(Ordering::Relaxed), 1);
    let after = languages::positions(&elements)
        .map(|at| {
            (
                at,
                languages::kind_at(&elements, at),
                languages::start_at(&elements, at),
                languages::end_at(&elements, at),
                languages::parent_at(&elements, at),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(original, after);
    let end = languages::end_of(&elements);
    for wanted in [
        languages::Relation::LiteralExpressionParse,
        languages::Relation::SemanticSyntax,
        languages::Relation::EmbeddedExpression,
        languages::Relation::SemanticSyntax,
    ] {
        let mut out = languages::contract::Collect::default();
        languages::contract::walk(
            &parse,
            &mut elements,
            &mut symbols,
            ByteSlice(source),
            languages::FileId(0),
            &[wanted],
            &mut out,
        );
        assert!(!out.rows().is_empty());
        assert!(out.partials.is_empty());
        assert_eq!(parse.model_preparations.load(Ordering::Relaxed), 1);
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(languages::end_of(&elements), end);
    }
    build(
        &parse,
        b"Alias = list[\"'U'\"]\n",
        &mut elements,
        &mut symbols,
    );
    assert_eq!(
        parse.model_preparations.load(Ordering::Relaxed),
        2,
        "same-sized refill is a new generation"
    );
    assert_eq!(count.load(Ordering::Relaxed), 2);
}
fn unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes() {
    let source = (0..512)
        .map(|i| format!("value_{i} = env_{i}[\"'runtime_key_{i}'\"]\n"))
        .collect::<String>();
    let count = Arc::new(AtomicUsize::new(0));
    let syntax = observed(
        PythonParserSettings::new(PythonTargetVersion::Py310),
        Arc::clone(&count),
    );
    let parse = ParseContract::new(
        PythonParserSettings::new(PythonTargetVersion::Py310),
        Arc::new(syntax),
        Arc::new(postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    build(&parse, source.as_bytes(), &mut elements, &mut symbols);
    assert_eq!(parse.model_preparations.load(Ordering::Relaxed), 1);
    assert_eq!(count.load(Ordering::Relaxed), 0);
    assert_eq!(languages::embedded_expressions(&elements).count(), 0);
    let native_end = u32::try_from(languages::positions(&elements).count()).unwrap();
    let added = (native_end..languages::end_of(&elements).0)
        .map(languages::Position)
        .collect::<Vec<_>>();
    let retained_bytes = added
        .iter()
        .filter_map(|&at| languages::decoded_at(&elements, at))
        .map(|text| text.0.len())
        .sum::<usize>();
    assert_eq!(added.len(), 0);
    assert_eq!(
        retained_bytes, 0,
        "actual appended decoded payload stays empty"
    );
}
fn unicode_and_escaped_carriers_use_the_existing_single_mapping_writer() {
    for source in ["Alias = list[\"'λ'\"]\n", "Alias = list[\"'\\u03bb'\"]\n"] {
        let semantic = reading::<SemanticSyntaxItem>(source);
        assert!(semantic.causes.is_empty());
        assert!(name(&semantic, "λ"));
        let embedded = reading::<EmbeddedExpressionItem>(source);
        assert_eq!(embedded.rows.len(), 2);
        assert_eq!(embedded.rows[1][1], embedded.rows[0][0]);
    }
}

fn complete_relation_rows_do_not_depend_on_demand_order() {
    fn run(order: &[languages::Relation]) -> Vec<(languages::Relation, Vec<Vec<String>>)> {
        let source = b"Alias = list[\"'T'\"]\nvalue = env[\"ordinary_key\"]\n";
        let count = Arc::new(AtomicUsize::new(0));
        let syntax = observed(
            PythonParserSettings::new(PythonTargetVersion::Py310),
            Arc::clone(&count),
        );
        let parse = ParseContract::new(
            PythonParserSettings::new(PythonTargetVersion::Py310),
            Arc::new(syntax),
            Arc::new(postcard()),
        );
        let (mut elements, mut symbols) = (
            languages::Elements::default(),
            languages::Symbols::default(),
        );
        build(&parse, source, &mut elements, &mut symbols);
        let mut result = Vec::new();
        for &relation in order {
            let mut out = languages::contract::Collect::default();
            languages::contract::walk(
                &parse,
                &mut elements,
                &mut symbols,
                ByteSlice(source),
                languages::FileId(0),
                &[relation],
                &mut out,
            );
            assert!(out.partials.is_empty());
            assert!(out.partials_at.is_empty());
            // These are the Symbol columns of the three actual typed row schemas.
            // Normalize only symbol handles; retain every node/file/span/number cell.
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "this fixture demands exactly three relations; any other is the failure the test reports"
            )]
            let symbol_columns: &[usize] = match relation {
                languages::Relation::SemanticSyntax
                | languages::Relation::LiteralExpressionParse => &[1, 2],
                languages::Relation::EmbeddedExpression => &[2, 3],
                _ => unreachable!("this fixture demands exactly three relations"),
            };
            let mut rows = out
                .rows()
                .into_iter()
                .map(|(actual, row)| {
                    assert_eq!(actual, relation);
                    row.into_iter()
                        .enumerate()
                        .map(|(column, word)| {
                            if symbol_columns.contains(&column) {
                                languages::text_of(
                                    &symbols,
                                    ByteSlice(source),
                                    languages::Symbol(u32::try_from(word).unwrap()),
                                )
                                .map_or_else(
                                    || "absent-symbol".to_owned(),
                                    |text| format!("text:{}", text.0),
                                )
                            } else {
                                format!("bits:{word}")
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert!(!rows.is_empty(), "{relation:?}");
            rows.sort();
            result.push((relation, rows));
        }
        assert_eq!(parse.model_preparations.load(Ordering::Relaxed), 1);
        assert_eq!(count.load(Ordering::Relaxed), 1);
        result.sort_by_key(|(relation, _)| *relation);
        result
    }
    let forward = [
        languages::Relation::LiteralExpressionParse,
        languages::Relation::SemanticSyntax,
        languages::Relation::EmbeddedExpression,
    ];
    let reverse = [
        languages::Relation::EmbeddedExpression,
        languages::Relation::SemanticSyntax,
        languages::Relation::LiteralExpressionParse,
    ];
    assert_eq!(run(&forward), run(&reverse));
}

/// The cases of this file, one variant each.
#[derive(Clone, Copy)]
enum Body {
    /// The case `native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill`.
    NativeIdsAndOneActualModelPreparationSurviveRepeatedDemandsAndRefill,
    /// The case `unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes`.
    UnrelatedRuntimeSubscriptsAddZeroParsesAndZeroLogicalPayloadBytes,
    /// The case `unicode_and_escaped_carriers_use_the_existing_single_mapping_writer`.
    UnicodeAndEscapedCarriersUseTheExistingSingleMappingWriter,
    /// The case `complete_relation_rows_do_not_depend_on_demand_order`.
    CompleteRelationRowsDoNotDependOnDemandOrder,
    /// The case `a_demand_that_reads_no_expanded_element_expands_no_annotation`.
    ADemandThatReadsNoExpandedElementExpandsNoAnnotation,
}

type Input = Body;
type Value = ();
type Failure = ();

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill",
        ),
        word: Word::Succeeds,
        input: || Body::NativeIdsAndOneActualModelPreparationSurviveRepeatedDemandsAndRefill,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes",
        ),
        word: Word::Succeeds,
        input: || Body::UnrelatedRuntimeSubscriptsAddZeroParsesAndZeroLogicalPayloadBytes,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unicode_and_escaped_carriers_use_the_existing_single_mapping_writer"),
        word: Word::Succeeds,
        input: || Body::UnicodeAndEscapedCarriersUseTheExistingSingleMappingWriter,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("complete_relation_rows_do_not_depend_on_demand_order"),
        word: Word::Succeeds,
        input: || Body::CompleteRelationRowsDoNotDependOnDemandOrder,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_demand_that_reads_no_expanded_element_expands_no_annotation"),
        word: Word::Succeeds,
        input: || Body::ADemandThatReadsNoExpandedElementExpandsNoAnnotation,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Body::NativeIdsAndOneActualModelPreparationSurviveRepeatedDemandsAndRefill => {
            native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill()
        }
        Body::UnrelatedRuntimeSubscriptsAddZeroParsesAndZeroLogicalPayloadBytes => {
            unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes()
        }
        Body::UnicodeAndEscapedCarriersUseTheExistingSingleMappingWriter => {
            unicode_and_escaped_carriers_use_the_existing_single_mapping_writer()
        }
        Body::CompleteRelationRowsDoNotDependOnDemandOrder => {
            complete_relation_rows_do_not_depend_on_demand_order()
        }
        Body::ADemandThatReadsNoExpandedElementExpandsNoAnnotation => {
            a_demand_that_reads_no_expanded_element_expands_no_annotation()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
