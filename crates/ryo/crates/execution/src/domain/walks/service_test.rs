//! The `walks` service's cases, over the small tree.

use super::service::axis;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::types::ElementPart;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::positions::service_test::{
    SMALL_SOURCE, SmallTree, lowering_of, room_of, rows_over, small_tree,
};
use crate::domain::positions::types::{Slots, Work};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::types::Operation;

/// What a row of the cases does: which way the walk goes, and from which element.
#[derive(Clone, Copy)]
struct Scenario {
    direction: Direction,
    from: u32,
}

/// What a row of the cases does.
type Input = Scenario;
/// The rows that remain, shown as the position walked from and the one reached.
type Value = &'static str;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let rows = rows_over(&elements);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let operation = Operation::Axis {
        from: PlanInput::Step(ryo_values::domain::plan_parts::types::StepId(0)),
        edges: PlanInput::Relation(Relation::SyntaxChild),
        domain: PlanInput::Relation(Relation::SyntaxTree),
        direction: scenario.direction,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    };
    let mut derived = crate::application::file_dag::testing::raw_scope()?;
    let mut out = Slots::default();
    axis(
        &rows,
        &lowering_of(&[ElementPart::Node]),
        &operation,
        Work {
            derived: &mut derived,
            room: &mut room_of(&[&[scenario.from]]),
            context: &mut context,
        },
        &mut out,
    )?;
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Ok(Box::leak(
        out.0
            .chunks(2)
            .map(|pair| match pair {
                [from, to] => format!("{from}>{to}"),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    ))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_walk_down_reaches_every_descendant"),
        word: Word::Succeeds,
        input: || Scenario {
            direction: Direction::Descendant,
            from: 0,
        },
        expect: Expect::Value("0>1 0>2 0>3"),
    },
    Case {
        name: CaseName("a_walk_to_the_children_reaches_the_children"),
        word: Word::Succeeds,
        input: || Scenario {
            direction: Direction::Child,
            from: 0,
        },
        expect: Expect::Value("0>1 0>2 0>3"),
    },
    Case {
        name: CaseName("a_walk_up_reaches_the_parent"),
        word: Word::Succeeds,
        input: || Scenario {
            direction: Direction::Parent,
            from: 3,
        },
        expect: Expect::Value("3>0"),
    },
    Case {
        name: CaseName("a_walk_up_to_the_nearest_ancestor_stops_at_the_first"),
        word: Word::Succeeds,
        input: || Scenario {
            direction: Direction::NearestAncestor,
            from: 2,
        },
        expect: Expect::Value("2>0"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
