//! The compiler checks types and constants before either operator reads any rows.

use super::service::call_replace;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::calling::service::call_join;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::syntax::constants::operator_named;
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::QueryView;

type Input = &'static str;
type Value = bool;
type Failure = LoweringError;

fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("query fixture parses");
    let mut builder = Builder {
        catalog: &[Relation::Function, Relation::SourceLocation],
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds: Vec::new(),
        steps: Vec::new(),
        lowered: Vec::new(),
        span: program.emit.span,
        origin: None,
        comparison_origins: Vec::new(),
    };
    for step in &program.steps {
        let ryo_values::domain::program::types::Expression::Call(call) = &step.expression else {
            panic!("call fixture required")
        };
        if operator_named(&call.name.0)
            == Some(ryo_values::domain::plans::types::QueryOperator::Replace)
        {
            call_replace(&mut builder, &step.name, call)?;
        } else {
            call_join(&mut builder, &step.name, call)?;
        }
    }
    Ok(builder.steps.iter().any(|step| {
        matches!(
            step.operation,
            Operation::Replace { .. }
                | Operation::OrderedMatch { .. }
                | Operation::OrderedDfs { .. }
        )
    }))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ambiguous_output_map_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=[file], left-order="start-byte", right-order="end-byte", output={file: [left,file], file: [right,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("replace_appends_text"),
        word: Word::Succeeds,
        input: || r#"let a = replace(from="function", text="name", pattern="_", replacement="-", output="label"); emit a subject file key(label);"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_regex_and_replacement_are_constants"),
        word: Word::Succeeds,
        input: || r#"let a = replace(from="function", text="name", pattern="", replacement="", output="label"); emit a subject file key(label);"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("invalid_regex_refused_without_rows"),
        word: Word::Refuses,
        input: || r#"let a = replace(from="function", text="name", pattern="[", replacement="", output="label"); emit a subject file key(label);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::InvalidRegex { .. }))
        }),
    },
    Case {
        name: CaseName("numeric_text_refused"),
        word: Word::Refuses,
        input: || r#"let a = replace(from="source-location", text="start-byte", pattern="a", replacement="", output="label"); emit a subject file key(label);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("output_collision_refused"),
        word: Word::Refuses,
        input: || r#"let a = replace(from="function", text="name", pattern="a", replacement="", output="name"); emit a subject file key(name);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("explicit_projection_and_numeric_order"),
        word: Word::Succeeds,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=[file], left-order="start-byte", right-order="end-byte", output={file: [left,file], span: [right,span]}); emit a subject file key(span);"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("text_order_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="function", right="function", left-on=[file], right-on=[file], left-order="name", right-order="name", output={file: [left,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("unlike_group_keys_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=["start-byte"], left-order="start-byte", right-order="end-byte", output={file: [left,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("unequal_key_counts_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=[], left-order="start-byte", right-order="end-byte", output={file: [left,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
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
        modules: std::collections::BTreeMap::new(),
    };
    if let Some(text) = module {
        let previous = bundle.modules.insert(
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
    let program = crate::domain::linking::service::link(&bundle, parsed.root, &parsed.fragments)
        .map_err(|error| error.message)?;
    let names = hosted
        .iter()
        .map(|relation| relation.name.clone())
        .collect::<Vec<_>>();
    crate::domain::validation::service::validate(&program, &[Relation::Function], &names)
        .map_err(|error| error.to_string())?;
    crate::domain::assembling::service::lower(&program, &[Relation::Function], hosted)
        .map_err(|error| error.to_string())
}

#[test]
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

#[test]
fn span_between_checks_instantiated_fragment_scope() {
    use ryo_language::domain::relations::types::CellKind;
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
