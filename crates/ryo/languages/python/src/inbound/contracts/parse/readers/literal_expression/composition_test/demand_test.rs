//! Actual completion under repeated demands: native ids, one model preparation and zero parses.
use super::preseal_test::build;
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
fn a_demand_that_reads_no_expanded_element_expands_no_annotation() -> String {
    let source = b"Alias = list[\"'T'\"]\n";
    // The outer annotation and the string nested in it: one carrier parse, two embedded expressions (the preseal row
    // `nested_runtime_alias_has_two_real_hosts_and_live_semantic_tokens` reads the same two).
    format!(
        "comment demand (parses, embedded) {:?}; embedded expression demand {:?}",
        read_with_demand(source, languages::Relation::Comment),
        read_with_demand(source, languages::Relation::EmbeddedExpression)
    )
}

fn native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill() -> String {
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
    let built = format!(
        "built: preparations {}, parses {}",
        parse.model_preparations.load(Ordering::Relaxed),
        count.load(Ordering::Relaxed)
    );
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
    let end = languages::end_of(&elements);
    let mut demands = Vec::new();
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
        demands.push(format!(
            "{wanted:?}: rows {}, partials {}, preparations {}, parses {}, end unchanged {}",
            !out.rows().is_empty(),
            out.partials.len(),
            parse.model_preparations.load(Ordering::Relaxed),
            count.load(Ordering::Relaxed),
            languages::end_of(&elements) == end
        ));
    }
    build(
        &parse,
        b"Alias = list[\"'U'\"]\n",
        &mut elements,
        &mut symbols,
    );
    // A same-sized refill is a new generation.
    format!(
        "{built}; native ids unchanged {}; {}; refilled: preparations {}, parses {}",
        original == after,
        demands.join("; "),
        parse.model_preparations.load(Ordering::Relaxed),
        count.load(Ordering::Relaxed)
    )
}
fn unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes() -> String {
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
    let native_end = u32::try_from(languages::positions(&elements).count()).unwrap();
    let added = (native_end..languages::end_of(&elements).0)
        .map(languages::Position)
        .collect::<Vec<_>>();
    let retained_bytes = added
        .iter()
        .filter_map(|&at| languages::decoded_at(&elements, at))
        .map(|text| text.0.len())
        .sum::<usize>();
    // The actual appended decoded payload stays empty.
    format!(
        "preparations {}, parses {}, embedded {}, added positions {}, decoded bytes {retained_bytes}",
        parse.model_preparations.load(Ordering::Relaxed),
        count.load(Ordering::Relaxed),
        languages::embedded_expressions(&elements).count(),
        added.len()
    )
}

/// The rows of each relation, and what each demand saw, as one order reads them.
type Demanded = (String, Vec<(languages::Relation, Vec<Vec<String>>)>);

fn complete_relation_rows_do_not_depend_on_demand_order() -> String {
    fn run(order: &[languages::Relation]) -> Demanded {
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
        let mut seen = Vec::new();
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
            let (partials, partials_at) = (out.partials.len(), out.partials_at.len());
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
            let collected = out.rows();
            let foreign = collected
                .iter()
                .filter(|(actual, _)| *actual != relation)
                .count();
            let mut rows = collected
                .into_iter()
                .map(|(_, row)| {
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
            seen.push(format!(
                "{relation:?}: rows {}, of another relation {foreign}, partials {partials}, partials at {partials_at}",
                rows.len()
            ));
            rows.sort();
            result.push((relation, rows));
        }
        seen.push(format!(
            "preparations {}, parses {}",
            parse.model_preparations.load(Ordering::Relaxed),
            count.load(Ordering::Relaxed)
        ));
        result.sort_by_key(|(relation, _)| *relation);
        (seen.join("; "), result)
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
    let (forward_seen, forward_rows) = run(&forward);
    let (reverse_seen, reverse_rows) = run(&reverse);
    format!(
        "forward {forward_seen}; reverse {reverse_seen}; rows equal {}",
        forward_rows == reverse_rows
    )
}

/// The cases of this file, one variant each.
#[derive(Clone, Copy)]
enum Body {
    /// The case `native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill`.
    NativeIdsAndOneActualModelPreparationSurviveRepeatedDemandsAndRefill,
    /// The case `unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes`.
    UnrelatedRuntimeSubscriptsAddZeroParsesAndZeroLogicalPayloadBytes,
    /// The case `complete_relation_rows_do_not_depend_on_demand_order`.
    CompleteRelationRowsDoNotDependOnDemandOrder,
    /// The case `a_demand_that_reads_no_expanded_element_expands_no_annotation`.
    ADemandThatReadsNoExpandedElementExpandsNoAnnotation,
}

type Input = Body;
/// What the case observed, rendered.
type Value = &'static str;
type Failure = ();

/// The rows of this file's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill",
        ),
        word: Word::Succeeds,
        input: || Body::NativeIdsAndOneActualModelPreparationSurviveRepeatedDemandsAndRefill,
        expect: Expect::Value(
            "built: preparations 1, parses 1; native ids unchanged true; LiteralExpressionParse: rows true, partials 0, preparations 1, parses 1, end unchanged true; SemanticSyntax: rows true, partials 0, preparations 1, parses 1, end unchanged true; EmbeddedExpression: rows true, partials 0, preparations 1, parses 1, end unchanged true; SemanticSyntax: rows true, partials 0, preparations 1, parses 1, end unchanged true; refilled: preparations 2, parses 2",
        ),
    },
    Case {
        name: CaseName(
            "unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes",
        ),
        word: Word::Succeeds,
        input: || Body::UnrelatedRuntimeSubscriptsAddZeroParsesAndZeroLogicalPayloadBytes,
        expect: Expect::Value(
            "preparations 1, parses 0, embedded 0, added positions 0, decoded bytes 0",
        ),
    },
    Case {
        name: CaseName("complete_relation_rows_do_not_depend_on_demand_order"),
        word: Word::Succeeds,
        input: || Body::CompleteRelationRowsDoNotDependOnDemandOrder,
        expect: Expect::Value(
            "forward LiteralExpressionParse: rows 3, of another relation 0, partials 0, partials at 0; SemanticSyntax: rows 36, of another relation 0, partials 0, partials at 0; EmbeddedExpression: rows 2, of another relation 0, partials 0, partials at 0; preparations 1, parses 1; reverse EmbeddedExpression: rows 2, of another relation 0, partials 0, partials at 0; SemanticSyntax: rows 36, of another relation 0, partials 0, partials at 0; LiteralExpressionParse: rows 3, of another relation 0, partials 0, partials at 0; preparations 1, parses 1; rows equal true",
        ),
    },
    Case {
        name: CaseName("a_demand_that_reads_no_expanded_element_expands_no_annotation"),
        word: Word::Succeeds,
        input: || Body::ADemandThatReadsNoExpandedElementExpandsNoAnnotation,
        expect: Expect::Value(
            "comment demand (parses, embedded) (0, 0); embedded expression demand (1, 2)",
        ),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Body::NativeIdsAndOneActualModelPreparationSurviveRepeatedDemandsAndRefill => {
            native_ids_and_one_actual_model_preparation_survive_repeated_demands_and_refill()
        }
        Body::UnrelatedRuntimeSubscriptsAddZeroParsesAndZeroLogicalPayloadBytes => {
            unrelated_runtime_subscripts_add_zero_parses_and_zero_logical_payload_bytes()
        }
        Body::CompleteRelationRowsDoNotDependOnDemandOrder => {
            complete_relation_rows_do_not_depend_on_demand_order()
        }
        Body::ADemandThatReadsNoExpandedElementExpandsNoAnnotation => {
            a_demand_that_reads_no_expanded_element_expands_no_annotation()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
