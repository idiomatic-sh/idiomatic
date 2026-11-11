//! Generation-local sharing, exact identity and bounded admission through the real owner.
use super::fixtures::{
    BesideAnIndex, Observed, Repeated, Retired, Unbudgeted, Unusable, beside_an_index, repeated,
    retired, unbudgeted, unusable,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

use crate::application::file_dag::derived::owner::graphs::fixtures::{ENDPOINTS, SOURCE, rows};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_relations::domain::relations::types::{FactColumn, Integer, Relation, Value as CellValue};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use std::borrow::Cow;
use std::sync::Arc;

/// The source a graph is asked over, and what the owner does around the asks.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The edges 1 -> 2, 2 -> 3 (twice) and 90 -> 91, acquired twice and once reversed, and
    /// reached from 1 each time.
    Repeated,
    /// The edges 1 -> 2 -> 3 under no budget, then under a graph-less owner.
    WithoutBudget,
    /// A partial population, a ragged one, then two generations of different facts.
    Unusable,
    /// The edges 1 -> 2 -> 3 -> 4 -> 5 while an index on its first column is live, then dormant.
    BesideAnIndex,
    /// The edges 1 -> 2 -> 3 and the identifiers {3}, then the owner retires its caches.
    Retired,
}

type Input = Asked;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared_adjacency_builds_once_and_sparse_root_memo_avoids_retraversal"),
        word: Word::Succeeds,
        input: || Asked::Repeated,
        expect: Expect::Value(Observed::Repeated(Repeated {
            reached: Cow::Borrowed(&[CellValue::Number(Integer(2)), CellValue::Number(Integer(3))]),
            first: (1, 1, 3),
            one_graph: true,
            one_answer: true,
            shared: (1, 1, 1, 1),
            reverse: Orientation::Reverse,
            reverse_one_graph: true,
            within_limit: true,
        })),
    },
    Case {
        name: CaseName("budget_refusal_keeps_complete_local_graph_without_retaining_handles"),
        word: Word::Succeeds,
        input: || Asked::WithoutBudget,
        expect: Expect::Value(Observed::Unbudgeted(Unbudgeted {
            sharing: GraphSharing::Local,
            graphs: 0,
            optional_bytes_unchanged: true,
            backings: 1,
            answered: false,
            walked: 2,
            refusals: 1,
            backings_after_drop: 0,
            graphless_acquired: false,
            graphless_graphs: 0,
        })),
    },
    Case {
        name: CaseName("physical_partiality_ragged_views_and_next_generation_never_hit_old_graphs"),
        word: Word::Succeeds,
        input: || Asked::Unusable,
        expect: Expect::Value(Observed::Unusable(Unusable {
            partial_acquired: false,
            ragged_acquired: false,
            one_graph_across_generations: false,
            builds: 1,
            reuses: 0,
            same_vertices: false,
        })),
    },
    Case {
        name: CaseName(
            "adjacency_admission_reclaims_dormant_payload_but_falls_back_while_the_index_is_live",
        ),
        word: Word::Succeeds,
        input: || Asked::BesideAnIndex,
        expect: Expect::Value(Observed::BesideAnIndex(BesideAnIndex {
            graph_costs: true,
            indexes: 1,
            index_retains_view_bytes: true,
            while_live: GraphSharing::Local,
            indexes_while_live: 1,
            pinned_refusals: 1,
            backings_while_live: 1,
            once_dormant: GraphSharing::Shared,
            indexes_once_dormant: 0,
            capacity_kept: true,
            reclaimed_indexes: 1,
            retained_view_bytes: 0,
            reclaimed_the_view: true,
            optional_within_limit: true,
            one_graph: true,
            same_vertices: true,
            same_forward: true,
            same_backward: true,
            builds: 1,
        })),
    },
    Case {
        name: CaseName(
            "retired_immutable_handles_keep_their_lease_but_cannot_authorize_new_demands",
        ),
        word: Word::Succeeds,
        input: || Asked::Retired,
        expect: Expect::Value(Observed::Retired(Retired {
            graphs: 1,
            domains: 1,
            required_leased: true,
            graphs_retired: 0,
            domains_retired: 0,
            retained_view_bytes: 0,
            optional_bytes: 0,
            held_still_charged: true,
            metadata_released: true,
            only_required_left: true,
            graph_still_read: true,
            domain_still_read: true,
            reach: Some(OperatingError::NotRead),
            nearest: Some(OperatingError::NotRead),
            required_after_drop: 0,
            unique_after_drop: 0,
        })),
    },
];

/// Asks the owner for the graph as the input says.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Asked::Repeated => Observed::Repeated(repeated()?),
        Asked::WithoutBudget => Observed::Unbudgeted(unbudgeted()?),
        Asked::Unusable => Observed::Unusable(unusable()?),
        Asked::BesideAnIndex => Observed::BesideAnIndex(beside_an_index()?),
        Asked::Retired => Observed::Retired(retired()?),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
