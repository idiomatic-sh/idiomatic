//! A step that compares rows on spans alone, from the compile inbound API: kept over per-file inputs, refused after the files.

use super::super::spans::test::span_plan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::ColumnLabel;
use ryo_values::domain::hosted::types::HostedColumn;
use ryo_values::domain::hosted::types::HostedName;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_step_on_spans_alone_needs_per_file_input"),
    word: Word::Succeeds,
    input: || a_step_on_spans_alone_needs_per_file_input,
    expect: Expect::Value(()),
}];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// A span states no file, so a step that compares rows on spans alone is kept only when every input is per-file and it runs inside
/// one file. Over the rows of a hosted relation, made after the files, it is refused with a request for a file key; a file key
/// beside the span is kept.
fn a_step_on_spans_alone_needs_per_file_input() {
    let global = [HostedRelation {
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
    let per_file = r#"let rows = select l.id as left, r.id as right, l.file as file, l.span as span from "function" as l join "function" as r on l.span = r.span; emit rows subject file key(left,right,span);"#;
    let plan = span_plan(per_file, None, &[]).expect("a join on spans of per-file inputs is kept");
    assert!(
        plan.steps
            .iter()
            .any(|step| matches!(step.operation, Operation::Join { .. })
                && step.scope == StepScope::PerFile),
        "the join on spans runs inside one file: {:?}",
        plan.steps
    );
    let refusal = "a step keyed on spans alone needs per-file input";
    let cross = r#"let rows = select l.id as left, l.file as file, l.span as span, r.file as other from "function" as l join "global" as r on l.span = r.span; emit rows subject file key(left,other);"#;
    let error = span_plan(cross, None, &global)
        .expect_err("a join on spans alone over rows after the files is refused");
    assert!(error.contains(refusal), "{error}");
    let keyed = r#"let rows = select l.id as left, l.file as file, l.span as span, r.file as other from "function" as l join "global" as r on l.file = r.file and l.span = r.span; emit rows subject file key(left,other);"#;
    let _plan = span_plan(keyed, None, &global).expect("a file key beside the span is kept");
    let count = r#"let c = group-count(from="global", keys=["span"], count="n"); emit c subject span key(n);"#;
    let error = span_plan(count, None, &global)
        .expect_err("a count on spans alone over rows after the files is refused");
    assert!(error.contains(refusal), "{error}");
}
