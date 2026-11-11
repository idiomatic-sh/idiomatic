//! Per-row and request-owned routes beside a shared head: barrier, classification, isolation.
use super::super::super::fixtures::Root;
use super::super::fixtures::routes::RowRoute;
use super::fixtures::{Observed, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{Fusibility, Ineligible};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{
    FileId, Integer, NodeId, Offset, Span, Value as Cell,
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

type Input = RowRoute;
type Value = Observed;
type Failure = OperatingError;

const HEAD: &[Cell] = &[
    Cell::Node(NodeId(0)),
    Cell::Node(NodeId(1)),
    Cell::Node(NodeId(2)),
];

/// The route of `kind` is its own region, excluded from fusion for `reason`, answers `routed`,
/// and leaves the head's three rows and the walk untouched.
const fn barrier(
    kind: OperationKind,
    reason: Ineligible,
    routed: Result<Root, OperatingError>,
) -> Observed {
    Observed {
        kind,
        class_member: false,
        fusibility: Some(Fusibility::Excluded(reason)),
        instructions: 1,
        head: Root::complete(HEAD, 3),
        routed,
        walk_counts_unchanged: true,
        retry_same: true,
        head_unpoisoned: true,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("per_row_and_request_owned_routes_are_barriers_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::Capture,
        expect: Expect::Value(barrier(
            OperationKind::Capture,
            Ineligible::RowFault,
            Ok(Root::complete(&[], 0)),
        )),
    },
    Case {
        name: CaseName("a_rank_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::Rank,
        expect: Expect::Value(barrier(
            OperationKind::Rank,
            Ineligible::RowFault,
            Ok(Root::complete(&[], 0)),
        )),
    },
    Case {
        name: CaseName("a_replace_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::Replace,
        expect: Expect::Value(barrier(
            OperationKind::Replace,
            Ineligible::RowFault,
            Ok(Root::complete(
                &[
                    Cell::Node(NodeId(0)),
                    Cell::Symbol(Symbol(0)),
                    Cell::File(FileId(0)),
                    Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(1),
                    }),
                    Cell::Symbol(Symbol(0)),
                    Cell::Node(NodeId(1)),
                    Cell::Symbol(Symbol(0)),
                    Cell::File(FileId(0)),
                    Cell::Span(Span {
                        start: Offset(1),
                        end: Offset(2),
                    }),
                    Cell::Symbol(Symbol(0)),
                    Cell::Node(NodeId(2)),
                    Cell::Symbol(Symbol(0)),
                    Cell::File(FileId(0)),
                    Cell::Span(Span {
                        start: Offset(2),
                        end: Offset(3),
                    }),
                    Cell::Symbol(Symbol(0)),
                ],
                3,
            )),
        )),
    },
    Case {
        name: CaseName("a_span_between_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::SpanBetween,
        expect: Expect::Value(barrier(
            OperationKind::SpanBetween,
            Ineligible::RowFault,
            Ok(Root::Failed(OperatingError::InvalidSpan)),
        )),
    },
    Case {
        name: CaseName("a_source_matches_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::SourceMatches,
        expect: Expect::Value(barrier(
            OperationKind::SourceMatches,
            Ineligible::RowFault,
            Ok(Root::Failed(OperatingError::InvalidSourceMatchSpan)),
        )),
    },
    Case {
        name: CaseName("a_render_syntax_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::RenderSyntax,
        expect: Expect::Value(barrier(
            OperationKind::RenderSyntax,
            Ineligible::RowFault,
            Ok(Root::SemanticIncomplete(Cow::Borrowed(
                "no file semantic session was admitted",
            ))),
        )),
    },
    Case {
        name: CaseName("a_fits_rendered_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::FitsRendered,
        expect: Expect::Value(barrier(
            OperationKind::FitsRendered,
            Ineligible::RowFault,
            Ok(Root::Failed(OperatingError::InvalidSpan)),
        )),
    },
    Case {
        name: CaseName("a_span_bounds_route_is_a_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::SpanBounds,
        expect: Expect::Value(barrier(
            OperationKind::SpanBounds,
            Ineligible::RowFault,
            Ok(Root::complete(
                &[
                    Cell::Node(NodeId(0)),
                    Cell::Symbol(Symbol(0)),
                    Cell::File(FileId(0)),
                    Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(1),
                    }),
                    Cell::Number(Integer(0)),
                    Cell::Number(Integer(1)),
                    Cell::Node(NodeId(1)),
                    Cell::Symbol(Symbol(0)),
                    Cell::File(FileId(0)),
                    Cell::Span(Span {
                        start: Offset(1),
                        end: Offset(2),
                    }),
                    Cell::Number(Integer(1)),
                    Cell::Number(Integer(2)),
                    Cell::Node(NodeId(2)),
                    Cell::Symbol(Symbol(0)),
                    Cell::File(FileId(0)),
                    Cell::Span(Span {
                        start: Offset(2),
                        end: Offset(3),
                    }),
                    Cell::Number(Integer(2)),
                    Cell::Number(Integer(3)),
                ],
                3,
            )),
        )),
    },
    Case {
        name: CaseName("a_rows_route_is_a_source_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::Rows,
        expect: Expect::Value(barrier(
            OperationKind::Rows,
            Ineligible::Source,
            Ok(Root::complete(&[Cell::Symbol(Symbol(0))], 1)),
        )),
    },
    Case {
        name: CaseName("a_hosted_route_is_a_source_barrier_beside_a_shared_head"),
        word: Word::Succeeds,
        input: || RowRoute::Hosted,
        expect: Expect::Value(barrier(
            OperationKind::Hosted,
            Ineligible::Source,
            Ok(Root::complete(
                &[Cell::Number(Integer(5)), Cell::Number(Integer(6))],
                2,
            )),
        )),
    },
];

/// Runs the route beside the shared head.
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
