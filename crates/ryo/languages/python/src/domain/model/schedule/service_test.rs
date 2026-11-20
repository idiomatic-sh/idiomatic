//! The `schedule` service's cases: the steps of a parsed source, in the language's evaluation order.

use super::service::build;
use super::types::ScheduleSource;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::types::{BindingAction, BindingAnnotationTiming, BindingSchedule};
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::FileId;

/// The source and when its annotations are evaluated.
type Input = (&'static str, BindingAnnotationTiming);
/// What each step does, in order.
type Value = &'static [BindingAction];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_value_is_read_before_its_target_is_bound"),
        word: Word::Succeeds,
        input: || ("x = y\n", BindingAnnotationTiming::Immediate),
        expect: Expect::Value(&[
            BindingAction::BeginPhase,
            BindingAction::Read,
            BindingAction::Bind,
            BindingAction::EndPhase,
        ]),
    },
    Case {
        name: CaseName("a_function_body_runs_in_a_later_phase_after_its_default"),
        word: Word::Succeeds,
        input: || {
            (
                "def f(a=d):\n    return a\n",
                BindingAnnotationTiming::Immediate,
            )
        },
        expect: Expect::Value(&[
            BindingAction::BeginPhase,
            BindingAction::Read,
            BindingAction::Bind,
            BindingAction::EndPhase,
            BindingAction::BeginPhase,
            BindingAction::Bind,
            BindingAction::Read,
            BindingAction::EndPhase,
        ]),
    },
    Case {
        name: CaseName("the_arms_of_a_condition_are_alternatives"),
        word: Word::Succeeds,
        input: || {
            (
                "if c:\n    x = 1\nelse:\n    x = 2\n",
                BindingAnnotationTiming::Immediate,
            )
        },
        expect: Expect::Value(&[
            BindingAction::BeginPhase,
            BindingAction::Read,
            BindingAction::BeginAlternatives,
            BindingAction::Bind,
            BindingAction::NextAlternative,
            BindingAction::Bind,
            BindingAction::EndAlternatives,
            BindingAction::EndPhase,
        ]),
    },
    Case {
        name: CaseName("an_immediate_annotation_is_read_where_it_stands"),
        word: Word::Succeeds,
        input: || ("x: T = v\n", BindingAnnotationTiming::Immediate),
        expect: Expect::Value(&[
            BindingAction::BeginPhase,
            BindingAction::Read,
            BindingAction::Read,
            BindingAction::Bind,
            BindingAction::EndPhase,
        ]),
    },
    Case {
        name: CaseName("a_deferred_annotation_is_read_in_a_phase_of_its_own"),
        word: Word::Succeeds,
        input: || ("x: T = v\n", BindingAnnotationTiming::Deferred),
        expect: Expect::Value(&[
            BindingAction::BeginPhase,
            BindingAction::Read,
            BindingAction::Bind,
            BindingAction::EndPhase,
            BindingAction::BeginPhase,
            BindingAction::Read,
            BindingAction::EndPhase,
        ]),
    },
];

fn subject((source, timing): Input) -> Value {
    let mut schedule = BindingSchedule::default();
    viewed(
        &mut ModelFixture::default(),
        source,
        FileId(0),
        &mut |view| {
            let tree = TypedTree {
                table: view.table,
                nodes: view.nodes,
            };
            let model = view.semantics(ModelPart::Lexical).model;
            build(
                &mut schedule,
                ScheduleSource {
                    lexical: &model.lexical,
                    definitions: &model.definitions,
                    tree,
                    timing,
                },
            );
        },
    );
    Box::leak(schedule.steps.iter().map(|step| step.action).collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
