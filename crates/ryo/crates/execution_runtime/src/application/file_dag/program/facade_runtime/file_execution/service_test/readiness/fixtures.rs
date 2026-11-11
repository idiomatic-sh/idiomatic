//! Runtime admission fixtures: the omitting adapter and the obligations it is compared against.
pub(super) mod rows;
use super::super::super::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
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
use crate::application::file_dag::program::kernel_session::KernelSession;
use crate::cases::{Case, CaseName, Expect, Word};
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
    pub(crate) expected: BTreeSet<Obligation>,
    pub(crate) served: BTreeSet<Obligation>,
    pub(crate) dispatches: usize,
    pub(crate) skipped: usize,
    pub(crate) heads: usize,
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
    let events = events.0.lock().unwrap();
    for event in events.iter().filter(|event| {
        event.get("event").map(String::as_str)
            == Some(crate::application::file_dag::program::constants::WALK_CLASS)
    }) {
        for head in numbers(&event["heads"]) {
            assert!(
                heads
                    .insert(head, event["class"].parse::<u64>().unwrap())
                    .is_none()
            );
        }
    }
    for event in events.iter() {
        match event.get("event").map(String::as_str) {
            Some(crate::application::file_dag::program::constants::WALK_READY) => {
                assert_eq!(event["phase"], "PerFile");
                assert_eq!(event["file"], "0");
                let head = event["logical"].parse::<u64>().unwrap();
                assert!(
                    actual_heads.insert(head),
                    "one authority-issued readiness obligation per physical head in this valid demand-all corpus"
                );
                expected.insert(Obligation {
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
                    assert!(
                        served.insert(Obligation {
                            generation: event["generation"].clone(),
                            population: event["population"].clone(),
                            class
                        }),
                        "same class/population/generation dispatched twice"
                    );
                }
            }
            Some(crate::application::file_dag::program::constants::WALK_REFUSED) => {
                panic!("this valid typed corpus has no representation refusal: {event:?}")
            }
            _ => {}
        }
    }
    assert_eq!(
        actual_heads, required_heads,
        "all independently declared and compiled roots of this demand-all corpus must reach runtime admission"
    );
    Coverage {
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

#[derive(Clone, Copy)]
pub(super) enum Scenario {
    Actual,
    Omitted,
}
