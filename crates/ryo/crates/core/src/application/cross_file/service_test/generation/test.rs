//! Ordinary project rules share one preparation and refuse another generation.
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::evaluation::count_findings;
use crate::application::cross_file::fixtures::evaluation::evaluate_fixture;
use crate::application::cross_file::fixtures::fed_steps;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::project_calls::ProjectCalls;
use crate::application::cross_file::fixtures::todo;
use crate::application::cross_file::fixtures::world_with;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::sync::Arc;

fn ordinary_project_rules_share_one_preparation_and_reject_another_generation()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
    use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
    use std::sync::atomic::Ordering;
    for order in [[0, 1], [1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, Vec::new());
        let mut rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let mut first_rendered = None;
        let mut first_payload = None;
        for rule in order {
            let declared = Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            };
            let result = evaluate_fixture(
                &rules,
                &declared,
                &frontier_rows(),
                &rules.covered,
                &mut symbols,
                &phase,
            )
            .unwrap();
            seen.push(format!("{:?}", fed_steps(&rules, rule)));
            let rendered = result.rendered.expect("actual core project rendering");
            let (shown, payload) = count_findings(&rendered);
            seen.push(shown);
            if let Some(first) = &first_payload {
                seen.push(format!("{:?}", (&payload) == (first)));
            } else {
                first_payload = Some(payload);
            }
            // Rendering carries the actual logical rule id even when its physical
            // publication is shared. Keep rule 0's full snapshot for its fresh request.
            if rule == 0 {
                first_rendered = Some(rendered);
            }
            seen.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
            seen.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        }
        // A changed physical coverage declaration cannot consume a cached publication.
        let changed = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        }];
        let second = Todo {
            rule: RuleIndex(1),
            id: rules.ids[1].clone(),
        };
        let result = evaluate_fixture(
            &rules,
            &second,
            &frontier_rows(),
            &changed,
            &mut symbols,
            &phase,
        );
        seen.push(format!("{:?}", matches!(result, Err(error) if error == ryo_values::domain::rule_requests::service::failed(
                &rules.ids[1], &OperatingError::WrongOperation,
            ))));
        seen.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        seen.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        phase.finish().unwrap();

        rules.request =
            ExecutionUseCaseImpl::new().execution(&rules.program, &rules.files, None.into());
        let phase = rules.request.project_phase().unwrap();
        let result = evaluate_fixture(
            &rules,
            &todo(0),
            &frontier_rows(),
            &rules.covered,
            &mut symbols,
            &phase,
        )
        .unwrap();
        seen.push(format!("{:?}", (result.rendered) == (first_rendered)));
        seen.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        seen.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        phase.finish().unwrap();
    }
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `ordinary_project_rules_share_one_preparation_and_reject_another_generation` scenario.
    OrdinaryProjectRulesShareOnePreparationAndRejectAnotherGeneration,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("ordinary_project_rules_share_one_preparation_and_reject_another_generation"),
    word: Word::Succeeds,
    input: || Given::OrdinaryProjectRulesShareOnePreparationAndRejectAnotherGeneration,
    expect: Expect::Value(
        "2|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|1|2|2|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|true|1|2|true|1|2|true|2|4|2|rule-1,2,0,-,-,rule-1,1,0,-,- rule-1rule-1 [5, 10] 1:0:0:1,1:0:0:0|1|2|2|rule,2,0,-,-,rule,1,0,-,- rulerule [5, 10] 0:0:0:1,0:0:0:0|true|1|2|true|1|2|true|2|4",
    ),
}];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::OrdinaryProjectRulesShareOnePreparationAndRejectAnotherGeneration => {
            ordinary_project_rules_share_one_preparation_and_reject_another_generation()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
