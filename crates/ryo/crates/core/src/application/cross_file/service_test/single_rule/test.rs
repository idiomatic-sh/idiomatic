//! One rule evaluated beside the project: its result, its skipped source and a foreign request.
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::evaluation::evaluate_outcome;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::world;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::crossing::service::partials_for;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

fn current_project_roots_keep_the_original_two_group_four_cell_result()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
    use ryo_execution::inbound::use_cases::execution::interface::Partials;
    use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
    use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
    let world = world(counting_plan(1, Vec::new()));
    let mut symbols = Symbols::default();
    let phase = world.request.project_phase().expect("one project phase");
    let outcome = evaluate_outcome(
        &world,
        &phase,
        RuleIndex(0),
        &frontier_rows(),
        &world.covered,
        || {
            Ok(PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
            })
        },
        &mut symbols,
    );
    let mut done = vec![outcome];
    let done = done
        .remove(0)
        .expect("actual source-bound frontier and rule");
    let table = match done.emit {
        RootOutcome::Complete(table) => table,
        _ => return Ok("the group root is not complete".to_owned()),
    };
    seen.push(format!("{:?}", [table.count.0, table.cells.len()]));
    Ok(seen.join("|"))
}

fn rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
    use ryo_values::domain::runs::types::{SkipReason, Skipped};
    let mut world = world(counting_plan(0, Vec::new()));
    let path = ryo_language::domain::files::types::FilePath("rejected.py".into());
    world.paths = vec![path.clone()];
    world.needs[0].cross.push(RelationRead {
        relation: Relation::Function,
        kinds: vec![],
    });
    let skipped = [Skipped {
        rule: world.ids[0].clone(),
        path,
        reason: SkipReason::SyntaxRejected,
    }];
    let mut input = crossing(&world);
    input.domain.skipped = &skipped;
    let partials = partials_for(&MetadataUseCaseImpl, &input, RuleIndex(0));
    seen.push(format!("{:?}", partials.entries.is_empty()));
    seen.push(format!("{:?}", partials.unavailable_files));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `current_project_roots_keep_the_original_two_group_four_cell_result` scenario.
    CurrentProjectRootsKeepTheOriginalTwoGroupFourCellResult,
    /// The `rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics` scenario.
    RejectedSyntaxBlocksCrossFileComplementsWithoutInventingPartialDiagnostics,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("current_project_roots_keep_the_original_two_group_four_cell_result"),
        word: Word::Succeeds,
        input: || Given::CurrentProjectRootsKeepTheOriginalTwoGroupFourCellResult,
        expect: Expect::Value("[2, 4]"),
    },
    Case {
        name: CaseName(
            "rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics",
        ),
        word: Word::Succeeds,
        input: || Given::RejectedSyntaxBlocksCrossFileComplementsWithoutInventingPartialDiagnostics,
        expect: Expect::Value("true|[FileId(0)]"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::CurrentProjectRootsKeepTheOriginalTwoGroupFourCellResult => {
            current_project_roots_keep_the_original_two_group_four_cell_result()?
        }
        Given::RejectedSyntaxBlocksCrossFileComplementsWithoutInventingPartialDiagnostics => {
            rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
