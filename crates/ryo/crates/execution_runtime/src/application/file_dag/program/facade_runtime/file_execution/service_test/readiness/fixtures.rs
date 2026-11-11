//! Runtime admission fixtures: the omitting adapter and the obligations it is compared against.
pub(super) mod rows;
use super::super::super::service::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use ryo_execution_domain::domain::table_cells::fixtures::{SMALL_SOURCE, SmallTree, small_tree};
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_cells::types::{TreeForm, TreeRows},
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{FileId, Integer, Relation};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope},
    plans::types::Operation,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::{rc::Rc, sync::Arc};
pub(super) mod adapter;
pub(super) mod events;
use self::adapter::ReadyAdapter;
use self::events::Events;
use super::super::fixtures::*;
use crate::application::file_dag::program::kernel_session::service::KernelSession;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use std::cell::Cell as Counter;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Obligation {
    pub(crate) generation: String,
    pub(crate) population: String,
    pub(crate) class: u64,
}
pub(crate) struct Coverage {
    pub(crate) events: EventChecks,
    pub(crate) expected: BTreeSet<Obligation>,
    pub(crate) served: BTreeSet<Obligation>,
    pub(crate) dispatches: usize,
    pub(crate) skipped: usize,
    pub(crate) heads: usize,
}
/// What the decoded events show beyond their obligations: class events naming a head already classed, ready events
/// off the per-file phase or off file 0, ready heads issued twice, dispatches of an obligation already served, and
/// whether the ready heads are exactly the heads the authored roots require.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EventChecks {
    pub(crate) reclassed_heads: usize,
    pub(crate) misplaced_ready: usize,
    pub(crate) repeated_ready_heads: usize,
    pub(crate) repeated_dispatches: usize,
    pub(crate) heads_as_required: bool,
}
pub(crate) fn numbers(value: &str) -> Vec<u64> {
    let value = value.strip_prefix('[').unwrap().strip_suffix(']').unwrap();
    if value.trim().is_empty() {
        return vec![];
    }
    value
        .split(',')
        .map(|number| number.trim().parse().unwrap())
        .collect()
}
pub(crate) fn population(value: &str) -> String {
    value
        .strip_prefix("Ok(")
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .into()
}
pub(crate) fn decode(events: &Events, skipped: usize, required_heads: BTreeSet<u64>) -> Coverage {
    let mut heads = BTreeMap::new();
    let mut expected = BTreeSet::new();
    let mut served = BTreeSet::new();
    let mut dispatches = 0;
    let mut ready_heads = 0;
    let mut actual_heads = BTreeSet::new();
    let mut reclassed_heads = 0;
    let mut misplaced_ready = 0;
    let mut repeated_ready_heads = 0;
    let mut repeated_dispatches = 0;
    let events = events.0.lock().unwrap();
    for event in events.iter().filter(|event| {
        event.get("event").map(String::as_str)
            == Some(crate::application::file_dag::program::constants::WALK_CLASS)
    }) {
        for head in numbers(&event["heads"]) {
            if heads
                .insert(head, event["class"].parse::<u64>().unwrap())
                .is_some()
            {
                reclassed_heads += 1;
            }
        }
    }
    for event in events.iter() {
        match event.get("event").map(String::as_str) {
            Some(crate::application::file_dag::program::constants::WALK_READY) => {
                if event["phase"] != "PerFile" || event["file"] != "0" {
                    misplaced_ready += 1;
                }
                let head = event["logical"].parse::<u64>().unwrap();
                // One authority-issued readiness obligation per physical head in this demand-all corpus.
                if !actual_heads.insert(head) {
                    repeated_ready_heads += 1;
                }
                let _new = expected.insert(Obligation {
                    generation: event["generation"].clone(),
                    population: population(&event["population"]),
                    class: *heads
                        .get(&head)
                        .expect("missing actual compiled physical-head mapping"),
                });
                ready_heads += 1;
            }
            Some(crate::application::file_dag::program::constants::WALK_DISPATCH) => {
                dispatches += 1;
                for class in numbers(&event["classes"]) {
                    if !served.insert(Obligation {
                        generation: event["generation"].clone(),
                        population: event["population"].clone(),
                        class,
                    }) {
                        repeated_dispatches += 1;
                    }
                }
            }
            Some(crate::application::file_dag::program::constants::WALK_REFUSED) => {
                panic!("this valid typed corpus has no representation refusal: {event:?}")
            }
            _ => {}
        }
    }
    Coverage {
        // Every independently declared and compiled root of this demand-all corpus reaches runtime admission.
        events: EventChecks {
            reclassed_heads,
            misplaced_ready,
            repeated_ready_heads,
            repeated_dispatches,
            heads_as_required: actual_heads == required_heads,
        },
        expected,
        served,
        dispatches,
        skipped,
        heads: ready_heads,
    }
}
// Distinct callsites make both CASES exercise first registration independently.
pub(crate) fn registration_probe(omission: bool) {
    if omission {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = "capture_registration_omission");
    } else {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = "capture_registration_actual");
    }
}

/// Whether the ready adapter submits every actual ready head, or omits one.
#[derive(Clone, Copy)]
pub(super) enum Submission {
    Actual,
    Omitted,
}
