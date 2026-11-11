//! One rule evaluated beside the project: its result, its skipped source and a foreign request.
use super::super::fixtures::evaluate_outcome;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::world;
use crate::domain::crossing::service::partials_for;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

pub(in super::super) fn current_project_roots_keep_the_original_two_group_four_cell_result()
-> Result<(), RuleRequestsError> {
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
    let RootOutcome::Complete(table) = done.emit else {
        panic!("complete group root")
    };
    assert_eq!([table.count.0, table.cells.len()], [2, 4]);
    Ok(())
}

pub(in super::super) fn rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics()
-> Result<(), RuleRequestsError> {
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
    assert!(partials.entries.is_empty(), "no provider partial invented");
    assert_eq!(
        partials.unavailable_files,
        [FileId(0)],
        "skipped source remains unavailable for absence checks"
    );
    Ok(())
}
