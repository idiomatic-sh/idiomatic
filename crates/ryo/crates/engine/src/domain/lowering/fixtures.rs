//! The test support of the `lowering` concept: the plans the engine's seam scenarios name, built by hand (the engine reads no query
//! text). Each is the smallest plan of its shape; the lowering's track makes each faithful to the shape it names.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, Input, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::{Emit, Plan};

/// A step that keeps every row of `from`.
fn kept(from: Input) -> PlanStep {
    PlanStep {
        operation: Operation::Select {
            from,
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }
}

/// A plan of `steps` emitting the last one's rows.
fn plan(steps: Vec<PlanStep>) -> Plan {
    let last = u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX);
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(last),
            subject: ColumnIndex(0),
            key: vec![ColumnIndex(0)],
            headings: vec![Heading("node".into())],
        },
        frontier: Vec::new(),
    }
}

/// The plan a seam scenario names; an unknown name is the one tree rule.
#[must_use]
pub fn named(scene: &str) -> Plan {
    match scene {
        // An antijoin whose right side reads a variable only the left binds.
        "unbound correlated sub-plan" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            kept(Input::Relation(Relation::SyntaxChild)),
            PlanStep {
                operation: Operation::Antijoin {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(1)],
                },
                scope: StepScope::PerFile,
            },
        ]),
        // A chain longer than the lowering's 4,096-instruction budget.
        "program over budget" => plan(
            std::iter::once(kept(Input::Relation(Relation::SyntaxTree)))
                .chain((0..4_096_u32).map(|step| kept(Input::Step(StepId(step)))))
                .collect(),
        ),
        // A rule reading two relations, one of which no enabled program reads in the file.
        "two relations, one read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxError)),
            kept(Input::Relation(Relation::Function)),
        ]),
        // A rule reading the relation the fixture language fills only in part.
        "partial relation" => plan(vec![kept(Input::Relation(Relation::SyntaxError))]),
        _ => plan(vec![kept(Input::Relation(Relation::SyntaxTree))]),
    }
}
