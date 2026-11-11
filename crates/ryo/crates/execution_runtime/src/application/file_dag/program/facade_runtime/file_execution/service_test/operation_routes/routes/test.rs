//! Every production route publishes the independent expected rows.
use super::super::fixtures::chain::execute_fixture;
use crate::application::file_dag::fixtures::operator_contract::expectations::Conformance;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use std::sync::Arc;
// Each case uses the real normal scheduler for both an emitted root and an authored downstream
// row consumer. Rows/Hosted remain request-owned sources. The route's rows are checked against
// the operator contract's independent expected rows of the kind; the value is how many it published.

type Input = (OperationKind, bool);
/// The rows the route published, and how they compare with the fixture's expected rows.
#[derive(Debug, PartialEq)]
struct Routed {
    rows: usize,
    conformance: Conformance,
}

type Value = Routed;
type Failure = ryo_execution_domain::domain::operating::errors::OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all26_actual_production_routes_publish_the_independent_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Replace, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_replace_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Replace, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_ordered_match_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::OrderedMatch, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName(
            "the_ordered_match_route_publishes_its_expected_rows_to_a_downstream_reader",
        ),
        word: Word::Succeeds,
        input: || (OperationKind::OrderedMatch, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_select_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Select, false),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_select_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Select, true),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_project_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Project, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_project_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Project, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_extend_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Extend, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_extend_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Extend, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_join_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Join, false),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_join_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Join, true),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_union_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Union, false),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_union_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Union, true),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_intersection_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Intersection, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_intersection_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Intersection, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_difference_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Difference, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_difference_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Difference, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_antijoin_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Antijoin, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_antijoin_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Antijoin, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_group_count_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::GroupCount, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_group_count_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::GroupCount, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_group_digest_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::GroupDigest, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_group_digest_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::GroupDigest, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_capture_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Capture, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_capture_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Capture, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_rank_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Rank, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_rank_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Rank, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_words_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Words, false),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_words_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Words, true),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_axis_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Axis, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_axis_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Axis, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_closure_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Closure, false),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_closure_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Closure, true),
        expect: Expect::Value(Routed {
            rows: 3,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_cycles_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Cycles, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_cycles_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Cycles, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_rows_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Rows, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_rows_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Rows, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_hosted_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::Hosted, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_hosted_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::Hosted, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_span_between_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::SpanBetween, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_span_between_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::SpanBetween, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_source_matches_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::SourceMatches, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName(
            "the_source_matches_route_publishes_its_expected_rows_to_a_downstream_reader",
        ),
        word: Word::Succeeds,
        input: || (OperationKind::SourceMatches, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_render_syntax_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::RenderSyntax, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName(
            "the_render_syntax_route_publishes_its_expected_rows_to_a_downstream_reader",
        ),
        word: Word::Succeeds,
        input: || (OperationKind::RenderSyntax, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_fits_rendered_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::FitsRendered, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName(
            "the_fits_rendered_route_publishes_its_expected_rows_to_a_downstream_reader",
        ),
        word: Word::Succeeds,
        input: || (OperationKind::FitsRendered, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_span_bounds_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::SpanBounds, false),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_span_bounds_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::SpanBounds, true),
        expect: Expect::Value(Routed {
            rows: 1,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_ordered_dfs_route_publishes_its_expected_rows"),
        word: Word::Succeeds,
        input: || (OperationKind::OrderedDfs, false),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
    Case {
        name: CaseName("the_ordered_dfs_route_publishes_its_expected_rows_to_a_downstream_reader"),
        word: Word::Succeeds,
        input: || (OperationKind::OrderedDfs, true),
        expect: Expect::Value(Routed {
            rows: 2,
            conformance: Conformance::Conforms,
        }),
    },
];

/// Runs the kind's fixture through the production route, emitted alone or read downstream.
fn subject((kind, downstream): Input) -> Result<Value, Failure> {
    execute_fixture(
        kind,
        downstream,
        Arc::new(crate::application::file_dag::fixtures::operators()),
    )
    .map(|(table, conformance)| Routed {
        rows: table.count.0,
        conformance,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
