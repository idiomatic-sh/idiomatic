//! What each step and each stage touches: one row for each instruction and each stage, and one more where an operand may be
//! absent or of another form. A body's own steps are never among a step's reads: every inline body here reads register 30.

use super::fixtures::{Scenario, Stage, Step, stage, step};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::steps::traversal::service::{stage_touched, step_touched};
use crate::domain::steps::traversal::types::{Touched, TouchedBody};
use std::borrow::Cow;

/// A body as a row names it: an inline body by how many steps it holds, a definition by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    Inline(usize),
    Definition(u32),
}

/// What a step or stage touches, by number.
#[derive(Debug, PartialEq, Eq)]
struct Seen {
    reads: Cow<'static, [u32]>,
    writes: Cow<'static, [u32]>,
    bodies: Cow<'static, [Body]>,
}

type Input = Scenario;
type Value = Seen;
type Failure = ();

/// The value a row expects.
const fn seen(
    reads: &'static [u32],
    writes: &'static [u32],
    bodies: &'static [Body],
) -> Expect<Value, Failure> {
    Expect::Value(Seen {
        reads: Cow::Borrowed(reads),
        writes: Cow::Borrowed(writes),
        bodies: Cow::Borrowed(bodies),
    })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_kind_writes_its_out"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Kind),
        expect: seen(&[], &[10], &[]),
    },
    Case {
        name: CaseName(
            "each_match_reads_its_register_values_and_names_the_definition_whose_table_it_reads",
        ),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Match),
        expect: seen(&[1, 2], &[10], &[Body::Definition(7)]),
    },
    Case {
        name: CaseName("each_range_reads_its_values_and_both_bounds_and_writes_the_range_value"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Range),
        expect: seen(&[1, 2, 3], &[10, 11], &[]),
    },
    Case {
        name: CaseName("each_range_with_no_low_and_a_constant_high_reads_its_values_alone"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::RangeOpen),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("each_row_writes_its_out"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Row),
        expect: seen(&[], &[10], &[]),
    },
    Case {
        name: CaseName("each_row_of_a_definitions_table_names_the_definition"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::RowDefinition),
        expect: seen(&[], &[10], &[Body::Definition(7)]),
    },
    Case {
        name: CaseName("children_read_the_parent"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Children),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("an_axis_reads_its_start"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Axis),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("within_reads_both_nodes"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Within),
        expect: seen(&[1, 2], &[], &[]),
    },
    Case {
        name: CaseName("nearest_over_the_tree_reads_its_start_and_memo_keys_and_sets_the_element"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::NearestTree),
        expect: seen(&[1, 20, 21], &[12, 10], &[Body::Inline(1)]),
    },
    Case {
        name: CaseName("nearest_over_derived_edges_reads_what_both_uses_bind_and_sets_from"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::NearestDerived),
        expect: seen(
            &[1, 2, 3],
            &[13, 12, 10],
            &[Body::Definition(7), Body::Definition(8)],
        ),
    },
    Case {
        name: CaseName("a_closure_reads_its_seed_and_sets_from"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Closure),
        expect: seen(&[1], &[13, 10], &[Body::Inline(1)]),
    },
    Case {
        name: CaseName("a_read_of_a_definitions_row_reads_the_handle_and_runs_nothing"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Read),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("a_test_reads_both_sides"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Test),
        expect: seen(&[1, 2], &[], &[]),
    },
    Case {
        name: CaseName("a_call_reads_its_register_arguments_and_writes_its_outputs"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Call),
        expect: seen(&[1, 2], &[10, 11], &[]),
    },
    Case {
        name: CaseName("a_render_reads_its_node"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Render),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("a_let_reads_its_source"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Let),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("exists_over_a_use_reads_what_the_use_binds_and_its_memo_keys"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Exists),
        expect: seen(&[1, 20], &[], &[Body::Definition(7)]),
    },
    Case {
        name: CaseName("exists_over_an_inline_body_reads_nothing_of_the_bodys_steps"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::ExistsInline),
        expect: seen(&[], &[], &[Body::Inline(1)]),
    },
    Case {
        name: CaseName("an_or_has_each_body_and_writes_its_outputs"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Or),
        expect: seen(&[1, 20], &[10, 11], &[Body::Inline(1), Body::Definition(7)]),
    },
    Case {
        name: CaseName("a_fork_has_each_slot"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Fork),
        expect: seen(&[], &[], &[Body::Inline(1), Body::Inline(1)]),
    },
    Case {
        name: CaseName("a_yield_reads_its_row"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Yield),
        expect: seen(&[1, 2], &[], &[]),
    },
    Case {
        name: CaseName("a_latest_reads_its_key_and_bound"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Latest),
        expect: seen(&[1, 2], &[10], &[]),
    },
    Case {
        name: CaseName("an_emit_reads_its_register_columns"),
        word: Word::Succeeds,
        input: || Scenario::Step(Step::Emit),
        expect: seen(&[1, 2], &[], &[]),
    },
    Case {
        name: CaseName("a_scan_names_the_definition_whose_table_it_reads"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Scan),
        expect: seen(&[], &[10], &[Body::Definition(7)]),
    },
    Case {
        name: CaseName("a_probe_reads_its_values_and_both_ends_of_its_range"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Probe),
        expect: seen(&[1, 2, 3], &[10], &[]),
    },
    Case {
        name: CaseName("a_probe_that_asks_present_has_its_out_as_written"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::ProbePresent),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("a_product_names_the_definition_whose_table_it_reads"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Product),
        expect: seen(&[], &[10], &[Body::Definition(7)]),
    },
    Case {
        name: CaseName("a_use_reads_what_it_binds_and_writes_what_it_collects"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Use),
        expect: seen(&[1, 2], &[10, 11], &[Body::Definition(7)]),
    },
    Case {
        name: CaseName("a_latest_stage_reads_its_key_and_bound"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Latest),
        expect: seen(&[1, 2], &[10], &[]),
    },
    Case {
        name: CaseName("a_read_stage_reads_the_handle"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Read),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("a_keep_reads_both_sides"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Keep),
        expect: seen(&[1, 2], &[], &[]),
    },
    Case {
        name: CaseName("a_call_stage_reads_its_arguments_and_writes_its_outputs"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Call),
        expect: seen(&[1, 2], &[10], &[]),
    },
    Case {
        name: CaseName("a_render_stage_reads_its_node"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Render),
        expect: seen(&[1], &[10], &[]),
    },
    Case {
        name: CaseName("a_walk_reads_its_start"),
        word: Word::Succeeds,
        input: || Scenario::Stage(Stage::Walk),
        expect: seen(&[1], &[10], &[]),
    },
];

/// What was touched, by number.
fn numbered(touched: &Touched<'_>) -> Seen {
    let bodies = touched.bodies.iter().map(|body| match body {
        TouchedBody::Inline(program) => Body::Inline(program.0.len()),
        TouchedBody::Definition(definition) => Body::Definition(definition.0),
    });
    Seen {
        reads: touched.reads.iter().map(|register| register.0).collect(),
        writes: touched.writes.iter().map(|register| register.0).collect(),
        bodies: bodies.collect(),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::Step(named) => numbered(&step_touched(&step(named))),
        Scenario::Stage(named) => numbered(&stage_touched(&stage(named))),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
