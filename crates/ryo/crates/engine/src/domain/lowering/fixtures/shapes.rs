//! The plans of the shapes the lowering's fifth step names, each the smallest of its shape: a step only projections read, and
//! the operations whose side is a relation where the scenes before them gave a step.

use super::parts::{joined, kept, nearest_step, of, plan, projected};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input, Side, StepId};
use ryo_values::domain::query_plans::types::Plan;

/// The plans of the fifth step's shapes; none for another name.
pub(super) fn shape_scene(scene: &str) -> Option<Plan> {
    Some(match scene {
        // The elements, read by two projections alone: one keeps the element and its kind, the other the element and its
        // span. No reader asks the text or the parent, so the step's definition holds three of its five columns.
        "the elements two projections read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            projected(0, &[0, 1]),
            projected(0, &[0, 4]),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[0], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 1)],
            ),
        ]),
        // The same elements, read by a projection and by a step that keeps every row whole: the definition holds every
        // column, since one reader reads the whole row.
        "the elements a projection and a whole read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            projected(0, &[0, 1]),
            kept(Input::Step(StepId(0))),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[0], &[0]),
                vec![of(Side::Left, 0), of(Side::Left, 1), of(Side::Right, 4)],
            ),
        ]),
        // The nearest element above each element, read by one projection that keeps what is reached alone. The walk's own
        // pattern names the start by its column, so the definition holds every column though its reader keeps one.
        "the nearest elements above, projected" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            nearest_step(Input::Relation(Relation::SyntaxChild)),
            projected(1, &[5]),
        ]),
        _ => return None,
    })
}
