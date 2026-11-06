//! What the `compile` use case promises, against the fakes of the ports it calls.

use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plans::types::StepScope;
use std::sync::Arc;

use super::errors::CompileError;
use super::interface::CompileUseCase as _;
use super::types::CompileRequest;
use super::use_case::CompileUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::{
    Alias, ByteOffset, ColumnName, ColumnReference, Columns, Condition, Expression, Filter,
    Joining, LetStep, Program, QuerySource, QuerySpan, Select, SourceReference, StepName,
    WrittenEmit, WrittenLiteral, WrittenOperand, WrittenRelation, WrittenTest,
};

/// What a row gives the use case.
type Input = fn() -> Given;
/// What `compile` answers: how many steps the plan has.
type Value = usize;
type Failure = CompileError;

const NOWHERE: QuerySpan = QuerySpan {
    start: ByteOffset(0),
    end: ByteOffset(0),
};

/// What a row gives the use case: the ports it calls, and the text.
struct Given {
    syntax: SyntaxFake,
    catalog: CatalogFake,
    text: &'static str,
    observations: Vec<BoundObservationInput>,
    hosted: Vec<HostedRelation>,
}

fn read_function(filters: Vec<Filter>) -> LetStep {
    LetStep {
        name: StepName("functions".into(), None),
        expression: Expression::Select(Select {
            columns: Columns::Every,
            from: QuerySource {
                reference: SourceReference::Relation(WrittenRelation("function".into())),
                alias: Alias::Unwritten,
                span: NOWHERE,
            },
            join: Joining::Alone,
            filters: filters.into_iter().map(Condition::Test).collect(),
        }),
        span: NOWHERE,
    }
}

fn program_with(filters: Vec<Filter>) -> Program {
    Program {
        linked: false,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: None,
        parameters: Vec::new(),
        steps: vec![read_function(filters)],
        emit: WrittenEmit {
            step: StepName("functions".into(), None),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: NOWHERE,
        },
    }
}

fn text_the_fake_reads() -> Given {
    Given {
        syntax: SyntaxFake {
            program: program_with(Vec::new()),
        },
        catalog: CatalogFake::default(),
        text: "emit",
        observations: Vec::new(),
        hosted: Vec::new(),
    }
}

fn text_the_fake_refuses() -> Given {
    Given {
        syntax: SyntaxFake::default(),
        catalog: CatalogFake::default(),
        text: "let a = ;",
        observations: Vec::new(),
        hosted: Vec::new(),
    }
}

fn catalog_without_relations() -> Given {
    Given {
        catalog: CatalogFake {
            relations: Vec::new(),
            fails: false,
        },
        ..text_the_fake_reads()
    }
}

fn failing_catalog() -> Given {
    Given {
        catalog: CatalogFake {
            relations: Vec::new(),
            fails: true,
        },
        ..text_the_fake_reads()
    }
}

fn comparing_text_with_a_number() -> Given {
    let filter = Filter {
        left: WrittenOperand::Text(WrittenLiteral("a".into())),
        test: WrittenTest::Equal,
        right: WrittenOperand::Number(Integer(1)),
        span: NOWHERE,
    };
    Given {
        syntax: SyntaxFake {
            program: program_with(vec![filter]),
        },
        ..text_the_fake_reads()
    }
}

/// A query that compares the `kind` of `defines` with the text it is given.
fn defines_where_kind_is(word: &str) -> Given {
    let filter = Filter {
        left: WrittenOperand::Column(ColumnReference {
            alias: Alias::Unwritten,
            column: ColumnName("kind".into()),
        }),
        test: WrittenTest::Equal,
        right: WrittenOperand::Text(WrittenLiteral(word.into())),
        span: NOWHERE,
    };
    let mut step = read_function(vec![filter]);
    if let Expression::Select(select) = &mut step.expression {
        select.from.reference = SourceReference::Relation(WrittenRelation("defines".into()));
    }
    Given {
        syntax: SyntaxFake {
            program: Program {
                steps: vec![step],
                ..program_with(Vec::new())
            },
        },
        catalog: CatalogFake {
            relations: vec![Relation::Defines],
            fails: false,
        },
        text: "emit",
        hosted: vec![],
        observations: vec![],
    }
}

fn a_kind_of_defines_that_is_misspelled() -> Given {
    defines_where_kind_is("headline")
}

fn a_kind_of_defines_that_exists() -> Given {
    defines_where_kind_is("section")
}

fn emitting_a_step_nobody_defined() -> Given {
    let mut program = program_with(Vec::new());
    program.emit.step = StepName("later".into(), None);
    Given {
        syntax: SyntaxFake { program },
        ..text_the_fake_reads()
    }
}

fn same_text_twice() -> Given {
    text_the_fake_reads()
}

#[test]
fn fragment_contract_compiles_against_real_relation_columns() {
    use crate::outbound::syntax::factory::winnow;
    let root = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            "import identity from \"query/identity\";\nlet nodes = select id from \"syntax-tree\";\nlet result = identity(source=nodes);\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("query/identity").unwrap(),
            "input source: relation<id: node>;\noutput relation<id: node>;\nlet selected = select id from source;\nreturn selected;",
        )],
    )
    .unwrap();
    let use_case = CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(CatalogFake {
            relations: vec![Relation::SyntaxTree],
            fails: false,
        }),
    );
    let report = use_case
        .compile(CompileRequest {
            observations: Vec::new(),
            text: root,
            hosted: Vec::new(),
        })
        .expect("the fragment input and output contracts match syntax-tree.id");
    assert_eq!(report.plan.emit.headings.len(), 1);
}

#[test]
fn fragment_relation_contracts_reject_input_and_output_shape_mismatches() {
    use crate::outbound::syntax::factory::winnow;

    let use_case = CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(CatalogFake {
            relations: vec![Relation::SyntaxTree],
            fails: false,
        }),
    );
    let root_text = "import identity from \"query/identity\";\nlet nodes = select id from \"syntax-tree\";\nlet result = identity(source=nodes);\nemit result subject id key(id);";
    for module_text in [
        "input source: relation<id: file>;\noutput relation<id: node>;\nlet selected = select id from source;\nreturn selected;",
        "input source: relation<id: node>;\noutput relation<id: file>;\nlet selected = select id from source;\nreturn selected;",
    ] {
        let bundle = ryo_values::domain::program::service::query_bundle(
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("root").unwrap(),
                root_text,
            ),
            [ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("query/identity").unwrap(),
                module_text,
            )],
        )
        .unwrap();
        let error = use_case
            .compile(CompileRequest {
                observations: Vec::new(),
                text: bundle,
                hosted: Vec::new(),
            })
            .expect_err("a mismatched relation signature is refused");
        assert!(
            matches!(error, CompileError::Located { origin, .. } if origin.source_id.0.as_ref() == "query/identity")
        );
    }
    let invalid_body = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            root_text,
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("query/identity").unwrap(),
            "input source: relation<id: node>;\noutput relation<id: node>;\nlet selected = select missing from source;\nreturn selected;",
        )],
    )
    .unwrap();
    let error = use_case
        .compile(CompileRequest {
            observations: Vec::new(),
            text: invalid_body,
            hosted: Vec::new(),
        })
        .expect_err("module body name errors retain their source location");
    assert!(
        matches!(error, CompileError::Located { origin, .. } if origin.source_id.0.as_ref() == "query/identity" && !origin.trace.is_empty())
    );
}

#[test]
fn nested_fragment_validation_error_keeps_the_inner_source_and_import_trace() {
    use crate::outbound::syntax::factory::winnow;

    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            "import outer from \"query/outer\";\nlet nodes = select id from \"syntax-tree\";\nlet result = outer(source=nodes);\nemit result subject id key(id);",
        ),
        [
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("query/outer").unwrap(),
                "import inner from \"query/inner\";\ninput source: relation<id: node>;\noutput relation<id: node>;\nlet result = inner(source=source);\nreturn result;",
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("query/inner").unwrap(),
                "input source: relation<id: node>;\noutput relation<id: node>;\nlet result = select missing from source;\nreturn result;",
            ),
        ],
    )
    .unwrap();
    let use_case = CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(CatalogFake {
            relations: vec![Relation::SyntaxTree],
            fails: false,
        }),
    );
    let error = use_case
        .compile(CompileRequest {
            observations: Vec::new(),
            text: bundle,
            hosted: Vec::new(),
        })
        .expect_err("the inner fragment reads an unknown column");
    assert!(matches!(
        error,
        CompileError::Located { origin, .. }
            if origin.source_id.0.as_ref() == "query/inner" && origin.trace.len() >= 4
    ));
}

#[test]
fn fragment_calls_cannot_capture_caller_or_nested_private_steps() {
    use crate::outbound::syntax::factory::winnow;

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
                    ryo_values::domain::program::service::source_id("root").unwrap(),
                    root_with_caller,
                ),
                [ryo_values::domain::program::service::document_source(
                    ryo_values::domain::program::service::source_id("query").unwrap(),
                    caller_capture,
                )],
            )
            .unwrap(),
            "query",
        ),
        (
            ryo_values::domain::program::service::query_bundle(
                ryo_values::domain::program::service::document_source(
                    ryo_values::domain::program::service::source_id("root").unwrap(),
                    root_with_nested,
                ),
                [
                    ryo_values::domain::program::service::document_source(
                        ryo_values::domain::program::service::source_id("outer").unwrap(),
                        outer,
                    ),
                    ryo_values::domain::program::service::document_source(
                        ryo_values::domain::program::service::source_id("inner").unwrap(),
                        inner,
                    ),
                ],
            )
            .unwrap(),
            "outer",
        ),
    ];
    let use_case = CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(CatalogFake {
            relations: vec![Relation::SyntaxTree],
            fails: false,
        }),
    );
    for (bundle, source) in bundles {
        let error = use_case
            .compile(CompileRequest {
                observations: Vec::new(),
                text: bundle,
                hosted: Vec::new(),
            })
            .expect_err("caller and nested private step spellings are not visible");
        assert!(
            matches!(error, CompileError::Located { origin, .. } if origin.source_id.0.as_ref() == source)
        );
    }
}

#[test]
fn a_fused_fragment_comparison_keeps_its_original_source_span() {
    use crate::outbound::syntax::factory::winnow;
    use ryo_values::domain::plans::types::Operation;

    let module = "input nodes: relation<id: node, kind: symbol>;\noutput relation<id: node, kind: symbol>;\nlet selected = select id, kind from nodes where kind = \"totally-unknown\";\nlet projected = select id, kind from selected;\nreturn projected;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            "import filter from \"filter\";\nlet nodes = select id, kind from \"syntax-tree\";\nlet result = filter(nodes=nodes);\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("filter").unwrap(),
            module,
        )],
    )
    .unwrap();
    let use_case = CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(CatalogFake {
            relations: vec![Relation::SyntaxTree],
            fails: false,
        }),
    );
    let plan = use_case
        .compile(CompileRequest {
            observations: Vec::new(),
            text: bundle,
            hosted: Vec::new(),
        })
        .expect("the filter lowers to a plan")
        .plan;
    let source = plan.steps.iter().find_map(|step| match &step.operation {
        Operation::Select { comparisons, .. }
        | Operation::Join {
            filter: comparisons,
            ..
        } => comparisons.iter().find_map(|comparison| comparison.origin),
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
        .and_then(|id| plan.comparison_origins.get(usize::try_from(id.0).ok()?))
        .expect("the optimized comparison points into its owning plan table");
    assert_eq!(origin.source_id.0.as_ref(), "filter");
    assert!(!origin.trace.is_empty());
    assert_eq!(
        module.get(origin.span.start.0..origin.span.end.0),
        Some("kind = \"totally-unknown\"")
    );
}

/// An external schema supplied at the real inbound API, without supplying any facts.
fn observed(name: &str, columns: &[&str]) -> BoundObservationInput {
    BoundObservationInput {
        name: ObservationBindingName(name.into()),
        columns: columns
            .iter()
            .map(|name| ObservationColumn {
                name: ObservationColumnName((*name).into()),
                kind: CellKind::File,
            })
            .collect(),
        scope: StepScope::PerFile,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_distinct_observation_schema_is_admitted"),
        word: Word::Succeeds,
        input: || {
            || Given {
                observations: vec![observed("external", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("blank_observation_name_is_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed(" ", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("observation_cannot_shadow_catalog_relation"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("function", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_sources_are_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![
                    observed("external", &["file"]),
                    observed("external", &["file"]),
                ],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("blank_observation_column_is_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &[" "])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_columns_are_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &["file", "file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("observation_cannot_shadow_hosted_relation"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &["file"])],
                hosted: vec![HostedRelation {
                    name: HostedName("external".into()),
                    columns: vec![HostedColumn {
                        name: ColumnLabel("file".into()),
                        kind: CellKind::File,
                    }],
                }],
                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("compiles_valid_text_to_a_plan"),
        word: Word::Succeeds,
        input: || text_the_fake_reads,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("same_text_gives_the_same_plan"),
        word: Word::Succeeds,
        input: || same_text_twice,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("invalid_text_is_refused"),
        word: Word::Refuses,
        input: || text_the_fake_refuses,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::Located { origin, error }
                if !origin.source_id.0.is_empty()
                    && origin.trace.is_empty()
                    && matches!(error.as_ref(), CompileError::Syntax { span, .. } if *span == origin.span)))
        }),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || catalog_without_relations,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, CompileError::UnknownRelation { .. })
                    || matches!(error, CompileError::Located { error, .. } if matches!(error.as_ref(), CompileError::UnknownRelation { .. })),
            )
        }),
    },
    Case {
        name: CaseName("type_mismatch_is_refused"),
        word: Word::Refuses,
        input: || comparing_text_with_a_number,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, CompileError::Type { .. })
                    || matches!(error, CompileError::Located { error, .. } if matches!(error.as_ref(), CompileError::Type { .. })),
            )
        }),
    },
    Case {
        name: CaseName("a_word_that_is_no_kind_of_the_column_it_is_compared_with_is_refused"),
        word: Word::Refuses,
        input: || a_kind_of_defines_that_is_misspelled,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(
                    error,
                    CompileError::Type { message, .. }
                        if message == "`headline` is not a kind of defines; the kinds are function, method, class, struct, enum, enum-member, trait, interface, type, type-alias, type-parameter, namespace, package, import, variable, constant, parameter, property, macro, section, label, implementation"
                ) || matches!(error, CompileError::Located { error, .. }
                    if matches!(error.as_ref(), CompileError::Type { message, .. }
                        if message == "`headline` is not a kind of defines; the kinds are function, method, class, struct, enum, enum-member, trait, interface, type, type-alias, type-parameter, namespace, package, import, variable, constant, parameter, property, macro, section, label, implementation")),
            )
        }),
    },
    Case {
        name: CaseName("a_word_that_is_a_kind_of_the_column_compiles"),
        word: Word::Succeeds,
        input: || a_kind_of_defines_that_exists,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_step_nobody_defined_is_refused_as_invalid"),
        word: Word::Refuses,
        input: || emitting_a_step_nobody_defined,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, CompileError::Invalid { .. })
                    || matches!(error, CompileError::Located { error, .. } if matches!(error.as_ref(), CompileError::Invalid { .. })),
            )
        }),
    },
    Case {
        name: CaseName("fails_when_catalog_port_fails"),
        word: Word::Fails,
        input: || failing_catalog,
        expect: Expect::Refusal(|error| selects!(matches!(error, CompileError::Catalog { .. }))),
    },
];

fn subject(given: fn() -> Given) -> Result<Value, Failure> {
    let given = given();
    let use_case = CompileUseCaseImpl::new(Arc::new(given.syntax), Arc::new(given.catalog));
    let request = CompileRequest {
        observations: given.observations,
        text: ryo_values::domain::program::types::QueryBundle::from(QueryText(given.text.into())),
        hosted: given.hosted,
    };
    let first = use_case.compile(request.clone())?;
    let second = use_case.compile(request)?;
    assert_eq!(first, second, "compilation is deterministic");
    Ok(first.plan.steps.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

/// A file-key join uses the generic executor, which needs a separate predicate step.
#[test]
fn raw_tree_file_joins_keep_single_and_and_or_filters_outside_the_join() {
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
    use ryo_values::domain::plans::types::Operation;
    use ryo_values::domain::program::types::QueryBundle;

    let compiler = CompileUseCaseImpl::new(
        Arc::new(crate::outbound::syntax::factory::winnow()),
        Arc::new(crate::outbound::catalog::factory::language()),
    );
    for predicate in [
        "r.text = \":\"",
        "r.text = \":\" and r.text != \"!\"",
        "r.text = \"!\" or r.text = \"=\" or r.text = \":\"",
    ] {
        for prefiltered in [false, true] {
            let (prefix, right, after) = if prefiltered {
                let condition = predicate.replace("r.", "");
                (
                    format!("let tokens = select * from \"syntax-tree\" where {condition};"),
                    "tokens",
                    String::new(),
                )
            } else {
                (
                    String::new(),
                    "\"syntax-tree\"",
                    format!(" where {predicate}"),
                )
            };
            let text = format!(
                "{prefix} let inputs = select id, file, span from \"function\"; \
                 let result = select r.id as id, l.file as file, r.span as span \
                 from inputs as l join {right} as r on l.file = r.file{after}; \
                 emit result subject file key(id);"
            );
            let report = compiler
                .compile(CompileRequest {
                    observations: Vec::new(),
                    text: QueryBundle::from(QueryText(text.into())),
                    hosted: Vec::new(),
                })
                .expect("both equivalent predicate spellings compile");
            let mut joins = 0_usize;
            for step in &report.plan.steps {
                if matches!(step.operation, Operation::Join { .. }) {
                    joins += 1;
                }
                if let Operation::Join {
                    right: Input::Relation(Relation::SyntaxTree),
                    right_on,
                    filter,
                    view,
                    ..
                } = &step.operation
                    && right_on == &[ColumnIndex(3)]
                {
                    assert!(filter.is_empty() && view.is_empty(), "{predicate}");
                }
            }
            assert!(joins > 0, "the regression must retain its file-key join");
        }
    }
}

#[test]
fn authored_observation_keeps_independent_root_through_optimization() {
    use ryo_values::domain::plan_parts::types::Input;
    use ryo_values::domain::program::types::QueryBundle;
    let compiler = CompileUseCaseImpl::new(
        Arc::new(crate::outbound::syntax::factory::winnow()),
        Arc::new(crate::outbound::catalog::factory::language()),
    );
    let report = compiler
        .compile(CompileRequest {
            observations: Vec::new(),
            hosted: Vec::new(),
            text: QueryBundle::from(QueryText(
                "let diagnostic = select file, span from \"function\"; \
             let usage = select file, id from \"function\"; \
             observe usage as used; emit diagnostic subject file key(span);"
                    .into(),
            )),
        })
        .expect("independent named and diagnostic roots compile");
    assert_eq!(report.plan.observations.len(), 1);
    let observed = &report.plan.observations[0];
    assert_eq!(&*observed.name.0, "used");
    assert_eq!(
        observed
            .columns
            .iter()
            .map(|column| &*column.name.0)
            .collect::<Vec<_>>(),
        ["file", "id"]
    );
    let Input::Step(root) = observed.source else {
        panic!("export has a real producer step");
    };
    assert!(usize::try_from(root.0).expect("step fits usize") < report.plan.steps.len());
    assert_ne!(
        root, report.plan.emit.step,
        "different output schemas retain separate roots"
    );
}

#[test]
fn bound_observation_preserves_declared_schema_and_phase() {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
    };
    use ryo_values::domain::plan_parts::types::Input;
    use ryo_values::domain::plans::types::StepScope;
    use ryo_values::domain::program::types::QueryBundle;
    let compiler = CompileUseCaseImpl::new(
        Arc::new(crate::outbound::syntax::factory::winnow()),
        Arc::new(crate::outbound::catalog::factory::language()),
    );
    let report = compiler.compile(CompileRequest {
        hosted: Vec::new(),
        observations: vec![BoundObservationInput {
            name: ObservationBindingName("published".into()),
            columns: vec![ObservationColumn { name: ObservationColumnName("file".into()), kind: CellKind::File }, ObservationColumn { name: ObservationColumnName("span".into()), kind: CellKind::Span }],
            scope: StepScope::CrossFile,
        }],
        text: QueryBundle::from(QueryText("let result = select file, span from published; emit result subject file key(span);".into())),
    }).expect("typed bound input compiles independently of the closed catalog");
    assert!(
        report
            .plan
            .steps
            .iter()
            .all(|step| step.scope == StepScope::CrossFile)
    );
    assert!(
        report
            .plan
            .frontier
            .iter()
            .any(|input| matches!(input, Input::BoundObservation(_)))
    );
}

#[cfg(test)]
mod fragment_hygiene;
#[cfg(test)]
mod fragment_refusal;

#[cfg(test)]
mod validation;

#[cfg(test)]
mod assembling;

#[cfg(test)]
mod calling;

#[cfg(test)]
mod combining;

#[cfg(test)]
mod comparing;

#[cfg(test)]
mod idioms;

#[cfg(test)]
mod joining;

#[cfg(test)]
mod planning;

#[cfg(test)]
mod reshaping;

#[cfg(test)]
mod resolving;

#[cfg(test)]
mod selecting;

#[cfg(test)]
mod substituting;

#[cfg(test)]
mod traversing;
