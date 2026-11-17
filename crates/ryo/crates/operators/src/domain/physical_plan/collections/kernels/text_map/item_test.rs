//! The `text_map` item's cases: which lowerings run the kernel.
use super::item::TextMapItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{InputRole, WitnessColumns, Witnessed, Witnesses};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::lowering::value::{Lowering, ProjectNode, TextMapNode};
use crate::domain::physical_plan::types::{PlanNodeId, RowMap, RowSource};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, LiteralText, ParameterId,
};
use ryo_values::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_map_stage_runs_it"),
        word: Word::Succeeds,
        input: || {
            Lowering::TextMap(TextMapNode {
                source: RowSource::Node(PlanNodeId(0)),
                map: RowMap::SpanBounds(crate::domain::span_bounds::service::lower(ColumnIndex(0))),
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
    Ok(TextMapItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

impl Witnesses for TextMapNode {
    /// One instance per row map kind.
    fn witnesses(columns: WitnessColumns) -> Vec<Witnessed<Self>> {
        [
            // One slot: every text is one segment.
            RowMap::Capture(crate::domain::capture::service::lower(
                columns.text,
                &CapturePattern("{slot}".into()),
            )),
            RowMap::Replace(crate::domain::replace::service::lower(
                columns.text,
                &LiteralText::default(),
                &LiteralText::default(),
            )),
            RowMap::SourceMatches(crate::domain::source_matches::service::lower(
                columns.span,
                None,
                &LiteralText::default(),
                None,
                // No capture group, so no capture text: a search that appends text needs a group.
                TextOutput::Omitted,
                LineOutput::Omitted,
            )),
            RowMap::SpanBounds(crate::domain::span_bounds::service::lower(columns.span)),
            RowMap::SpanBetween(crate::domain::span_between::service::lower(
                columns.span,
                columns.span,
            )),
            RowMap::Words(crate::domain::words::service::lower(columns.text)),
            // A rank reads its layers from its binding: one layer of one text.
            RowMap::Rank(crate::domain::rank::service::lower(
                columns.text,
                ParameterId(0),
                &[ryo_values::domain::query_bindings::types::ParameterBinding::TextSets(vec![
                    vec![LiteralText(crate::domain::growth::types::WITNESS_TEXT.into())],
                ])],
            )),
            // The literals interleave the texts: one before and one after the one text.
            RowMap::FitsRendered(crate::domain::fits_rendered::service::lower(
                &[columns.text],
                &[LiteralText::default(), LiteralText::default()],
                columns.span,
                MaxLine(88),
                TabWidth(4),
            )),
        ]
        .into_iter()
        .map(|map| Witnessed {
            node: Self {
                source: RowSource::Node(PlanNodeId(0)),
                map,
            },
            roles: vec![InputRole::Rows],
        })
        .collect()
    }
}
