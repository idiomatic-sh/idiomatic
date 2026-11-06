//! The span, source and projection calls over fragment scopes, from the compile inbound API.

use crate::cases::run;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::hosted::types::ColumnLabel;
use ryo_values::domain::hosted::types::HostedColumn;
use ryo_values::domain::hosted::types::HostedName;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plans::types::Operation;

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("span_between_types_and_arguments"),
        word: Word::Succeeds,
        input: || span_between_types_and_arguments,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("span_between_checks_instantiated_fragment_scope"),
        word: Word::Succeeds,
        input: || span_between_checks_instantiated_fragment_scope,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_types_arguments_pattern_and_scope"),
        word: Word::Succeeds,
        input: || source_matches_types_arguments_pattern_and_scope,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_named_capture_is_typed_and_validated"),
        word: Word::Succeeds,
        input: || source_matches_named_capture_is_typed_and_validated,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("span_bounds_types_outputs_and_file_scope"),
        word: Word::Succeeds,
        input: || span_bounds_types_outputs_and_file_scope,
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

/// Compile real syntax through validation, linking, scope resolution and optimization.
fn span_plan(
    root: &str,
    module: Option<&str>,
    hosted: &[ryo_values::domain::hosted::types::HostedRelation],
) -> Result<ryo_values::domain::plans::types::Plan, String> {
    use ryo_values::domain::program::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
    let mut bundle = QueryBundle {
        root: QueryDocumentSource {
            id: QuerySourceId("root".into()),
            text: root.into(),
        },
        modules: Default::default(),
    };
    if let Some(text) = module {
        let previous = std::sync::Arc::make_mut(&mut bundle.modules).insert(
            QuerySourceId("span-fragment".into()),
            QueryDocumentSource {
                id: QuerySourceId("span-fragment".into()),
                text: text.into(),
            },
        );
        assert!(previous.is_none());
    }
    let parsed = winnow()
        .read_bundle(&bundle)
        .map_err(|error| error.to_string())?;
    let program = crate::domain::expanding::service::link(&bundle, parsed.root, &parsed.fragments)
        .map_err(|error| error.to_string())?;
    let names = hosted
        .iter()
        .map(|relation| relation.name.clone())
        .collect::<Vec<_>>();
    crate::domain::validation::service::validate_with_observations(
        &program,
        &[Relation::Function],
        &names,
        &[],
    )
    .map_err(|error| error.to_string())?;
    crate::domain::assembling::service::lower(&program, &[Relation::Function], hosted)
        .map_err(|error| error.to_string())
}

fn span_between_types_and_arguments() {
    let valid = r#"let a = span-between(from="function", start="span", end="span", output="diagnostic"); emit a subject file key(diagnostic);"#;
    let plan = span_plan(valid, None, &[]).expect("typed span projection");
    assert!(
        plan.steps
            .iter()
            .any(|step| matches!(step.operation, Operation::SpanBetween { .. }))
    );
    for (bad, expected) in [
        (
            valid.replace("start=\"span\"", "start=\"name\""),
            "cannot compare",
        ),
        (
            valid.replace("end=\"span\"", "end=\"id\""),
            "cannot compare",
        ),
        (
            valid.replace("output=\"diagnostic\"", "output=\"span\""),
            "already exists",
        ),
        (
            valid.replace("start=\"span\"", "start=\"missing\""),
            "no column",
        ),
        (valid.replace("start=\"span\", ", ""), "needs the argument"),
        (valid.replace("end=\"span\", ", ""), "needs the argument"),
        (
            valid.replace(
                "output=\"diagnostic\"",
                "output=\"diagnostic\", extra=\"span\"",
            ),
            "extra",
        ),
    ] {
        let error = span_plan(&bad, None, &[]).expect_err("bad call must fail");
        assert!(error.contains(expected), "{bad}: {error}");
    }
}

fn span_between_checks_instantiated_fragment_scope() {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::hosted::types::{
        ColumnLabel, HostedColumn, HostedName, HostedRelation,
    };
    let module = r#"input rows: relation<file: file, span: span>;
output relation<file: file, span: span, diagnostic: span>;
let result = span-between(from=rows, start="span", end="span", output="diagnostic");
return result;"#;
    let root = r#"import compose from "span-fragment";
let rows = select file, span from "function";
let a = compose(rows=rows); emit a subject file key(diagnostic);"#;
    let _plan = span_plan(root, Some(module), &[]).expect("span fragment lowers");
    let hosted = [HostedRelation {
        name: HostedName("global".into()),
        columns: vec![
            HostedColumn {
                name: ColumnLabel("file".into()),
                kind: CellKind::File,
            },
            HostedColumn {
                name: ColumnLabel("span".into()),
                kind: CellKind::Span,
            },
        ],
    }];
    let cross = root.replace("from \"function\"", "from \"global\"");
    assert!(
        span_plan(&cross, Some(module), &hosted)
            .expect_err("file column is not per-file proof")
            .contains("requires per-file")
    );
    let direct = r#"let a = span-between(from="global", start="span", end="span", output="diagnostic"); emit a subject file key(diagnostic);"#;
    assert!(
        span_plan(direct, None, &hosted)
            .expect_err("cross file refused")
            .contains("requires per-file")
    );
}

fn source_matches_types_arguments_pattern_and_scope() {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::hosted::types::{
        ColumnLabel, HostedColumn, HostedName, HostedRelation,
    };
    let valid = r#"let a = source-matches(from="function", span="span", pattern="fn", output="hit"); emit a subject file key(hit);"#;
    let plan = span_plan(valid, None, &[]).expect("typed source occurrence projection");
    assert!(
        plan.steps
            .iter()
            .any(|step| matches!(step.operation, Operation::SourceMatches { .. }))
    );
    for (bad, expected) in [
        (
            valid.replace("span=\"span\"", "span=\"name\""),
            "cannot compare",
        ),
        (
            valid.replace("span=\"span\"", "span=\"missing\""),
            "no column",
        ),
        (
            valid.replace("output=\"hit\"", "output=\"span\""),
            "already exists",
        ),
        (
            valid.replace("pattern=\"fn\"", "pattern=\"[\""),
            "invalid regular expression",
        ),
        (valid.replace("span=\"span\", ", ""), "needs the argument"),
        (
            valid.replace("output=\"hit\"", "output=\"hit\", extra=\"x\""),
            "extra",
        ),
    ] {
        let error = span_plan(&bad, None, &[]).expect_err("bad source-matches call must fail");
        assert!(error.contains(expected), "{bad}: {error}");
    }
    let hosted = [HostedRelation {
        name: HostedName("global".into()),
        columns: vec![
            HostedColumn {
                name: ColumnLabel("file".into()),
                kind: CellKind::File,
            },
            HostedColumn {
                name: ColumnLabel("span".into()),
                kind: CellKind::Span,
            },
        ],
    }];
    let cross = valid.replace("from=\"function\"", "from=\"global\"");
    assert!(
        span_plan(&cross, None, &hosted)
            .expect_err("cross-file source refused")
            .contains("requires per-file")
    );
    let with_line = r#"let base = extend(from="function", values={base_line:1});
let a = source-matches(from=base, span="span", base-line="base_line", pattern="fn", output="hit", line-output="hit_line");
emit a subject file key(hit);"#;
    let _plan = span_plan(with_line, None, &[]).expect("paired line outputs lower");
    for bad in [
        with_line.replace(", line-output=\"hit_line\"", ""),
        with_line.replace("base-line=\"base_line\", ", ""),
    ] {
        assert!(
            span_plan(&bad, None, &[])
                .expect_err("unpaired line arguments refuse")
                .contains("both `base-line` and `line-output`")
        );
    }
}

fn source_matches_named_capture_is_typed_and_validated() {
    let valid = r#"let a = source-matches(from="function", span="span", pattern="(?P<run>\\\\+)", output="hit", capture="run", capture-output="run_span", capture-text-output="run_text"); emit a subject file key(hit);"#;
    let plan = span_plan(valid, None, &[]).expect("named capture lowers");
    assert!(plan.steps.iter().any(|step| matches!(
        step.operation,
        Operation::SourceMatches {
            capture: Some(_),
            capture_text: ryo_values::domain::plans::types::TextOutput::Appended,
            capture_line: ryo_values::domain::plans::types::LineOutput::Omitted,
            ..
        }
    )));
    for (bad, expected) in [
        (
            valid.replace(", capture-output=\"run_span\"", ""),
            "requires `capture`",
        ),
        (
            valid.replace("capture=\"run\"", "capture=\"absent\""),
            "no named capture",
        ),
        (
            valid.replace("capture-output=\"run_span\"", "capture-output=\"span\""),
            "already exists",
        ),
        (valid.replace("capture=\"run\", ", ""), "requires `capture`"),
    ] {
        let error = span_plan(&bad, None, &[]).expect_err("invalid named capture must fail");
        assert!(error.contains(expected), "{bad}: {error}");
    }
}

fn span_bounds_types_outputs_and_file_scope() {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::hosted::types::{
        ColumnLabel, HostedColumn, HostedName, HostedRelation,
    };
    let valid = r#"let a = span-bounds(from="function", span="span", start-output="begin", end-output="finish"); emit a subject file key(span);"#;
    let plan = span_plan(valid, None, &[]).expect("typed span endpoints");
    assert!(
        plan.steps
            .iter()
            .any(|step| matches!(step.operation, Operation::SpanBounds { .. }))
    );
    for (bad, expected) in [
        (
            valid.replace("span=\"span\"", "span=\"name\""),
            "cannot compare",
        ),
        (
            valid.replace("span=\"span\"", "span=\"missing\""),
            "no column",
        ),
        (
            valid.replace("start-output=\"begin\"", "start-output=\"span\""),
            "already exists",
        ),
        (
            valid.replace("end-output=\"finish\"", "end-output=\"begin\""),
            "already exists",
        ),
        (
            valid.replace(
                "end-output=\"finish\"",
                "end-output=\"finish\", extra=\"x\"",
            ),
            "extra",
        ),
    ] {
        let error = span_plan(&bad, None, &[]).expect_err("invalid span-bounds call");
        assert!(error.contains(expected), "{bad}: {error}");
    }
    let hosted = [HostedRelation {
        name: HostedName("global".into()),
        columns: vec![
            HostedColumn {
                name: ColumnLabel("span".into()),
                kind: CellKind::Span,
            },
            HostedColumn {
                name: ColumnLabel("file".into()),
                kind: CellKind::File,
            },
        ],
    }];
    let cross = valid.replace("from=\"function\"", "from=\"global\"");
    let cross_plan = span_plan(&cross, None, &hosted)
        .expect("pure endpoint projection is valid after files are joined");
    let scopes: Vec<_> = cross_plan
        .steps
        .iter()
        .filter_map(|step| {
            matches!(step.operation, Operation::SpanBounds { .. }).then_some(step.scope)
        })
        .collect();
    assert_eq!(
        scopes,
        [ryo_values::domain::plan_parts::types::StepScope::CrossFile]
    );
    assert!(
        cross_plan.frontier.is_empty(),
        "hosted-only input needs no file frontier"
    );
}
