//! The `render` kernel's cases, through the plan and run use cases: one batch per node per file carrying each node's role,
//! no batch for no rows, and an unsupported rendering refusing the batch before any row.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::fixtures::{defines_rows, over_defines};
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_language::domain::semantics::types::{RenderContext, RenderOutcome, RenderedNode};
use ryo_relations::domain::relations::types::{Offset, Relation, Span};
use ryo_relations::domain::terms::types::UnknownReason;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Three rows, the second's node not applicable.
    Three,
    /// No row.
    Empty,
    /// Two rows, the second's rendering unsupported.
    Unsupported,
}
type Input = Scenario;
/// The rows and the calls the resources saw, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("render_asks_once_with_each_node_s_role"),
        word: Word::Succeeds,
        input: || Scenario::Three,
        expect: Expect::Value(Cow::Borrowed(
            "Ok([RowCount(2)]); [Rendered([RenderRequest { node: NodeId(0), context: AttributeReceiver }, RenderRequest { node: NodeId(1), context: AttributeReceiver }, RenderRequest { node: NodeId(2), context: AttributeReceiver }]), Minted(\"x\"), Minted(\"z\")]",
        )),
    },
    Case {
        name: CaseName("render_over_no_rows_asks_nothing"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Cow::Borrowed("Ok([RowCount(0)]); []")),
    },
    Case {
        name: CaseName("an_unsupported_rendering_refuses_the_batch_before_any_row"),
        word: Word::Succeeds,
        input: || Scenario::Unsupported,
        expect: Expect::Value(Cow::Borrowed(
            "Err(UnsupportedRender); [Rendered([RenderRequest { node: NodeId(0), context: AttributeReceiver }, RenderRequest { node: NodeId(1), context: AttributeReceiver }])]",
        )),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let known = |text: &str| RenderOutcome::Known(RenderedNode(text.into()));
    let (count, renderings) = match scenario {
        Scenario::Three => (
            3,
            vec![known("x"), RenderOutcome::NotApplicable, known("z")],
        ),
        Scenario::Empty => (0, vec![known("x")]),
        Scenario::Unsupported => (
            2,
            vec![known("x"), RenderOutcome::Unsupported(UnknownReason(7))],
        ),
    };
    let mut resources = TextResources::new(&["n"]).rendering(renderings);
    let empty = Span {
        start: Offset(0),
        end: Offset(0),
    };
    let rows: Vec<_> = (0..count).map(|_| (resources.symbol("n"), empty)).collect();
    let table = defines_rows(&rows);
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let steps = [over_defines(Operation::RenderSyntax {
        from: StepInput::Relation(Relation::Defines),
        node: ColumnIndex(0),
        context: RenderContext::AttributeReceiver,
    })];
    let answered = emitted(
        &steps,
        &[PlanNodeId(0)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    );
    Ok(Cow::Owned(format!(
        "{:?}; {:?}",
        answered.map(|tables| tables
            .into_iter()
            .map(|table| table.count)
            .collect::<Vec<_>>()),
        resources.calls()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
