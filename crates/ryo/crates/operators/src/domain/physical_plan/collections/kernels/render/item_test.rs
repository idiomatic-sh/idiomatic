//! The `render` item's cases: which lowerings run the kernel.
use super::item::RenderItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{Lowering, ProjectNode, RenderNode};
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_render_runs_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::Render(RenderNode {
                source: RowSource::Node(PlanNodeId(0)),
                render: crate::domain::render_batch::service::lower(
                    ColumnIndex(0),
                    RenderContext::PositionalArgument,
                ),
            })
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_projection_does_not_run_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::Project(ProjectNode {
                source: RowSource::Node(PlanNodeId(0)),
                project: crate::domain::project::service::lower(
                    &[ColumnIndex(0)],
                    ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
                ),
            })
        },
        expect: Expect::Value(Reach::Skips),
    },
];

/// Calls the part on one input; the author states it.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; whether a lowering runs a kernel cannot fail"
)]
fn subject(lowering: Input) -> Result<Value, Failure> {
    Ok(RenderItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for RenderNode {
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        vec![Witnessed {
            node: Self {
                source: RowSource::Node(PlanNodeId(0)),
                render: crate::domain::render_batch::service::lower(
                    columns.id,
                    RenderContext::AttributeReceiver,
                ),
            },
            roles: vec![InputRole::Rows],
        }]
    }
}
