//! Fragment relation contracts compiled through the real parser, from the compile inbound API.

use super::super::compiler_over;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::errors::CompileError;
use crate::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_values::domain::plans::types::QueryRequest;
use ryo_relations::domain::relations::types::Relation;

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fragment_contract_compiles_against_real_relation_columns"),
        word: Word::Succeeds,
        input: || fragment_contract_compiles_against_real_relation_columns,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("fragment_relation_contracts_reject_input_and_output_shape_mismatches"),
        word: Word::Succeeds,
        input: || fragment_relation_contracts_reject_input_and_output_shape_mismatches,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("nested_fragment_validation_error_keeps_the_inner_source_and_import_trace"),
        word: Word::Succeeds,
        input: || nested_fragment_validation_error_keeps_the_inner_source_and_import_trace,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("fragment_calls_cannot_capture_caller_or_nested_private_steps"),
        word: Word::Succeeds,
        input: || fragment_calls_cannot_capture_caller_or_nested_private_steps,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_fused_fragment_comparison_keeps_its_original_source_span"),
        word: Word::Succeeds,
        input: || a_fused_fragment_comparison_keeps_its_original_source_span,
        expect: Expect::Value(()),
    },
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn fragment_contract_compiles_against_real_relation_columns() {
    let root = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("root")).unwrap(),
            "import identity from \"query/identity\";\nlet nodes = select id from \"syntax-tree\";\nlet result = identity(source=nodes);\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("query/identity")).unwrap(),
            "input source: relation<id: node>;\noutput relation<id: node>;\nlet selected = select id from source;\nreturn selected;",
        )],
    )
    .unwrap();
    let use_case = compiler_over(vec![Relation::SyntaxTree]);
    let report = use_case
        .compile(QueryRequest {
            observations: Vec::new(),
            text: root,
            hosted: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        })
        .expect("the fragment input and output contracts match syntax-tree.id");
    assert_eq!(report.plan.emit.headings.len(), 1);
}

fn fragment_relation_contracts_reject_input_and_output_shape_mismatches() {
    let use_case = compiler_over(vec![Relation::SyntaxTree]);
    let root_text = "import identity from \"query/identity\";\nlet nodes = select id from \"syntax-tree\";\nlet result = identity(source=nodes);\nemit result subject id key(id);";
    for module_text in [
        "input source: relation<id: file>;\noutput relation<id: node>;\nlet selected = select id from source;\nreturn selected;",
        "input source: relation<id: node>;\noutput relation<id: file>;\nlet selected = select id from source;\nreturn selected;",
    ] {
        let bundle = ryo_values::domain::program::service::query_bundle(
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(
                    ryo_values::domain::query_tokens::types::QuerySpelling("root"),
                )
                .unwrap(),
                root_text,
            ),
            [ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(
                    ryo_values::domain::query_tokens::types::QuerySpelling("query/identity"),
                )
                .unwrap(),
                module_text,
            )],
        )
        .unwrap();
        let error = use_case
            .compile(QueryRequest {
                observations: Vec::new(),
                text: bundle,
                hosted: Vec::new(),
                modules: ryo_values::domain::program::types::ParsedModules::default(),
            })
            .expect_err("a mismatched relation signature is refused");
        assert!(
            matches!(error, CompileError::Located { origin, .. } if origin.source_id.0.as_ref() == "query/identity")
        );
    }
    let invalid_body = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("root")).unwrap(),
            root_text,
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("query/identity")).unwrap(),
            "input source: relation<id: node>;\noutput relation<id: node>;\nlet selected = select missing from source;\nreturn selected;",
        )],
    )
    .unwrap();
    let error = use_case
        .compile(QueryRequest {
            observations: Vec::new(),
            text: invalid_body,
            hosted: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        })
        .expect_err("module body name errors retain their source location");
    assert!(
        matches!(error, CompileError::Located { origin, .. } if origin.source_id.0.as_ref() == "query/identity" && !origin.trace.is_empty())
    );
}

fn nested_fragment_validation_error_keeps_the_inner_source_and_import_trace() {
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("root")).unwrap(),
            "import outer from \"query/outer\";\nlet nodes = select id from \"syntax-tree\";\nlet result = outer(source=nodes);\nemit result subject id key(id);",
        ),
        [
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("query/outer")).unwrap(),
                "import inner from \"query/inner\";\ninput source: relation<id: node>;\noutput relation<id: node>;\nlet result = inner(source=source);\nreturn result;",
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("query/inner")).unwrap(),
                "input source: relation<id: node>;\noutput relation<id: node>;\nlet result = select missing from source;\nreturn result;",
            ),
        ],
    )
    .unwrap();
    let use_case = compiler_over(vec![Relation::SyntaxTree]);
    let error = use_case
        .compile(QueryRequest {
            observations: Vec::new(),
            text: bundle,
            hosted: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        })
        .expect_err("the inner fragment reads an unknown column");
    assert!(matches!(
        error,
        CompileError::Located { origin, .. }
            if origin.source_id.0.as_ref() == "query/inner" && origin.trace.len() >= 4
    ));
}

fn fragment_calls_cannot_capture_caller_or_nested_private_steps() {
    let root_with_caller = "import query from \"query\";\nlet secret = select id from \"syntax-tree\";\nlet result = query();\nemit result subject id key(id);";
    let caller_capture = "output relation<id: node, n: number>;\nlet counted = group-count(from=secret, keys=[\"id\"], count=\"n\");\nreturn counted;";
    let root_with_nested =
        "import outer from \"outer\";\nlet result = outer();\nemit result subject id key(id);";
    let outer = "import inner from \"inner\";\noutput relation<id: node>;\nlet result = inner();\nlet counted = group-count(from=hidden, keys=[\"id\"], count=\"n\");\nreturn result;";
    let inner =
        "output relation<id: node>;\nlet hidden = select id from \"syntax-tree\";\nreturn hidden;";
    let bundles = [
        (
            ryo_values::domain::program::service::query_bundle(
                ryo_values::domain::program::service::document_source(
                    ryo_values::domain::program::service::source_id(
                        ryo_values::domain::query_tokens::types::QuerySpelling("root"),
                    )
                    .unwrap(),
                    root_with_caller,
                ),
                [ryo_values::domain::program::service::document_source(
                    ryo_values::domain::program::service::source_id(
                        ryo_values::domain::query_tokens::types::QuerySpelling("query"),
                    )
                    .unwrap(),
                    caller_capture,
                )],
            )
            .unwrap(),
            "query",
        ),
        (
            ryo_values::domain::program::service::query_bundle(
                ryo_values::domain::program::service::document_source(
                    ryo_values::domain::program::service::source_id(
                        ryo_values::domain::query_tokens::types::QuerySpelling("root"),
                    )
                    .unwrap(),
                    root_with_nested,
                ),
                [
                    ryo_values::domain::program::service::document_source(
                        ryo_values::domain::program::service::source_id(
                            ryo_values::domain::query_tokens::types::QuerySpelling("outer"),
                        )
                        .unwrap(),
                        outer,
                    ),
                    ryo_values::domain::program::service::document_source(
                        ryo_values::domain::program::service::source_id(
                            ryo_values::domain::query_tokens::types::QuerySpelling("inner"),
                        )
                        .unwrap(),
                        inner,
                    ),
                ],
            )
            .unwrap(),
            "outer",
        ),
    ];
    let use_case = compiler_over(vec![Relation::SyntaxTree]);
    for (bundle, source) in bundles {
        let error = use_case
            .compile(QueryRequest {
                observations: Vec::new(),
                text: bundle,
                hosted: Vec::new(),
                modules: ryo_values::domain::program::types::ParsedModules::default(),
            })
            .expect_err("caller and nested private step spellings are not visible");
        assert!(
            matches!(error, CompileError::Located { origin, .. } if origin.source_id.0.as_ref() == source)
        );
    }
}

fn a_fused_fragment_comparison_keeps_its_original_source_span() {
    use ryo_values::domain::plans::types::Operation;

    let module = "input nodes: relation<id: node, kind: symbol>;\noutput relation<id: node, kind: symbol>;\nlet selected = select id, kind from nodes where kind = \"totally-unknown\";\nlet projected = select id, kind from selected;\nreturn projected;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("root")).unwrap(),
            "import filter from \"filter\";\nlet nodes = select id, kind from \"syntax-tree\";\nlet result = filter(nodes=nodes);\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("filter")).unwrap(),
            module,
        )],
    )
    .unwrap();
    let use_case = compiler_over(vec![Relation::SyntaxTree]);
    let plan = use_case
        .compile(QueryRequest {
            observations: Vec::new(),
            text: bundle,
            hosted: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        })
        .expect("the filter lowers to a plan")
        .plan;
    let source = plan.steps.iter().find_map(|step| match &step.operation {
        Operation::Select { comparisons, .. }
        | Operation::Join {
            filter: comparisons,
            ..
        } => comparisons
            .iter()
            .find_map(|comparison| match comparison.origin {
                ryo_values::domain::plan_parts::types::ComparisonOrigin::Recorded(id) => Some(id),
                ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded => None,
            }),
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::Hosted { .. } => None,
    });
    let origin = source
        .and_then(|id| {
            let Ok(place) = usize::try_from(id.0) else {
                return None;
            };
            plan.comparison_origins.get(place)
        })
        .expect("the optimized comparison points into its owning plan table");
    assert_eq!(origin.source_id.0.as_ref(), "filter");
    assert!(!origin.trace.is_empty());
    assert_eq!(
        module.get(origin.span.start.0..origin.span.end.0),
        Some("kind = \"totally-unknown\"")
    );
}
