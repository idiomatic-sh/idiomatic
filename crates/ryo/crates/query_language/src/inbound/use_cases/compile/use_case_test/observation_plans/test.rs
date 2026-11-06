//! The plans of joins over the raw tree and of authored and bound observations, from the compile inbound API.

use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::observations::types::ObservationBindingName;
use ryo_values::domain::observations::types::ObservationColumn;
use ryo_values::domain::observations::types::ObservationColumnName;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plan_parts::types::StepScope;
use std::sync::Arc;
use crate::inbound::use_cases::compile::interface::CompileUseCase as _;
use crate::inbound::use_cases::compile::types::CompileRequest;
use crate::inbound::use_cases::compile::use_case::CompileUseCaseImpl;

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("raw_tree_file_joins_keep_single_and_and_or_filters_outside_the_join"),
        word: Word::Succeeds,
        input: || raw_tree_file_joins_keep_single_and_and_or_filters_outside_the_join,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("authored_observation_keeps_independent_root_through_optimization"),
        word: Word::Succeeds,
        input: || authored_observation_keeps_independent_root_through_optimization,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("bound_observation_preserves_declared_schema_and_phase"),
        word: Word::Succeeds,
        input: || bound_observation_preserves_declared_schema_and_phase,
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

fn raw_tree_file_joins_keep_single_and_and_or_filters_outside_the_join() {
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
    use ryo_values::domain::plans::types::Operation;
    use ryo_values::domain::program::types::QueryBundle;

    let compiler = CompileUseCaseImpl::new(
        Arc::new(crate::outbound::syntax::fixtures::winnow::winnow()),
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

fn authored_observation_keeps_independent_root_through_optimization() {
    use ryo_values::domain::plan_parts::types::Input;
    use ryo_values::domain::program::types::QueryBundle;
    let compiler = CompileUseCaseImpl::new(
        Arc::new(crate::outbound::syntax::fixtures::winnow::winnow()),
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

fn bound_observation_preserves_declared_schema_and_phase() {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
    };
    use ryo_values::domain::plan_parts::types::Input;
    use ryo_values::domain::plan_parts::types::StepScope;
    use ryo_values::domain::program::types::QueryBundle;
    let compiler = CompileUseCaseImpl::new(
        Arc::new(crate::outbound::syntax::fixtures::winnow::winnow()),
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
