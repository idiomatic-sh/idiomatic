//! Row-progress accounting of the opt-in diagnostic profile.
use std::time::Instant;

use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Value;

use crate::domain::operator::contract::views::ProgressCounter;

use super::super::constants::{self, MAX_KEYS};
use super::super::profile::service::STATE;
use super::super::schemas::document;
use super::super::types::{
    RouteCardinality, RouteCounters, RouteFileIndex, RouteFileName, RouteHotRow, RouteInputKey,
    RouteInputSlot, RouteKey, RouteLabel, RouteRowFile, RouteRuleName, RouteSnapshot, RouteTally,
};

pub(crate) fn file_name(name: &FilePath) {
    let name = RouteFileName(name.0.to_string());
    STATE.with_borrow_mut(|state| {
        if let Some(state) = state {
            state.progress.name = name;
        }
    });
    flush(RouteLabel(constants::STAGE_SCOPE_START));
}
pub(crate) fn row_file(row: &[Value], paths: &[FilePath]) {
    let file = row.iter().find_map(|value| match value {
        Value::File(file) => Some(file),
        _ => None,
    });
    STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return;
        };
        match file {
            Some(file)
                if state.progress.row_file.as_ref().map(|known| known.file.0) != Some(file.0) =>
            {
                state.progress.row_file = paths.get(file.0 as usize).map(|path| RouteRowFile {
                    file: ryo_relations::domain::relations::types::FileId(file.0),
                    path: RouteFileName(path.0.to_string()),
                });
            }
            None => state.progress.row_file = None,
            _ => {}
        }
    });
}
fn counter_label(counter: ProgressCounter) -> RouteLabel {
    RouteLabel(match counter {
        ProgressCounter::InputRows => constants::COUNTER_INPUT_ROWS,
        ProgressCounter::PredicateEvaluations => constants::COUNTER_PREDICATE_EVALUATIONS,
        ProgressCounter::IndexProbes => constants::COUNTER_INDEX_PROBES,
        ProgressCounter::IndexRowsExamined => constants::COUNTER_INDEX_ROWS_EXAMINED,
        ProgressCounter::EqualityMatches => constants::COUNTER_EQUALITY_MATCHES,
        ProgressCounter::JoinCandidatePairs => constants::COUNTER_JOIN_CANDIDATE_PAIRS,
        ProgressCounter::KernelOutputRows => constants::COUNTER_KERNEL_OUTPUT_ROWS,
    })
}
pub(crate) fn progress(counter: ProgressCounter, amount: u64) {
    let label = counter_label(counter);
    let flush_now = STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return false;
        };
        let key = RouteKey {
            origin: state.origin,
            route: RouteLabel(constants::ROW_PROGRESS),
            family: label,
        };
        if !state.counts.contains_key(&key) && state.counts.len() == MAX_KEYS {
            state.overflow.0 += amount;
        } else {
            state.counts.entry(key).or_default().calls.0 += amount;
        }
        state.progress.ticks.0 += amount;
        if state.progress.ticks.0 == amount
            || (state.progress.ticks.0 & constants::FLUSH_TICK_MASK == 0
                && state.progress.last.elapsed().as_secs() >= constants::FLUSH_SECONDS)
        {
            state.progress.last = Instant::now();
            true
        } else {
            false
        }
    });
    if flush_now {
        flush(RouteLabel(constants::STAGE_PROGRESS));
    }
}
fn add_counter(counters: &mut RouteCounters, family: RouteLabel, calls: RouteTally) {
    let slot = match family.0 {
        constants::COUNTER_INPUT_ROWS => &mut counters.input_rows,
        constants::COUNTER_EMITTED_ROWS => &mut counters.emitted_rows,
        constants::COUNTER_PREDICATE_EVALUATIONS => &mut counters.predicate_evaluations,
        constants::COUNTER_INDEX_PROBES => &mut counters.index_probes,
        constants::COUNTER_INDEX_ROWS_EXAMINED => &mut counters.index_chain_rows_examined,
        constants::COUNTER_EQUALITY_MATCHES => &mut counters.equality_matches,
        constants::COUNTER_JOIN_CANDIDATE_PAIRS => &mut counters.join_candidate_pairs,
        constants::COUNTER_KERNEL_OUTPUT_ROWS => &mut counters.kernel_output_rows,
        constants::COUNTER_POSITION_ROWS_YIELDED => &mut counters.position_rows_yielded,
        _ => return,
    };
    slot.0 += calls.0;
}
fn snapshot() -> Option<RouteSnapshot> {
    STATE.with_borrow(|state| {
        let state = state.as_ref()?;
        let mut rows = state
            .counts
            .iter()
            .filter(|(key, _)| key.route == RouteLabel(constants::ROW_PROGRESS))
            .collect::<Vec<_>>();
        rows.sort_by_key(|(key, count)| {
            (
                std::cmp::Reverse(count.calls.0),
                key.origin.rule,
                key.origin.node,
                key.family,
            )
        });
        let mut counters = RouteCounters::default();
        for (key, count) in &state.counts {
            if key.route == RouteLabel(constants::ROW_PROGRESS)
                && key.origin.rule == state.origin.rule
                && key.origin.node == state.origin.node
                && key.origin.namespace == state.origin.namespace
            {
                add_counter(&mut counters, key.family, count.calls);
            }
        }
        let input_cardinalities: [Option<RouteCardinality>; constants::INPUT_SLOTS] =
            std::array::from_fn(|which| {
                state
                    .progress
                    .inputs
                    .get(&RouteInputKey {
                        rule: state.origin.rule,
                        namespace: state.origin.namespace,
                        node: state.origin.node,
                        slot: RouteInputSlot(which),
                    })
                    .copied()
            });
        let operation = state
            .origin
            .node
            .and_then(|node| {
                state
                    .progress
                    .nodes
                    .get(&super::super::types::RouteNodeKey {
                        rule: state.origin.rule,
                        namespace: state.origin.namespace,
                        node,
                    })
            })
            .cloned()
            .unwrap_or_default();
        let hottest = rows
            .into_iter()
            .take(constants::HOTTEST_LIMIT)
            .map(|(key, count)| RouteHotRow {
                rule: key.origin.rule,
                node: key.origin.node,
                counter: key.family,
                count: count.calls,
                rule_name: key
                    .origin
                    .rule
                    .and_then(|rule| state.rules.get(&rule))
                    .cloned()
                    .unwrap_or_else(|| RouteRuleName(constants::NO_RULE.to_owned())),
                input_cardinalities: std::array::from_fn(|which| {
                    state
                        .progress
                        .inputs
                        .get(&RouteInputKey {
                            rule: key.origin.rule,
                            namespace: key.origin.namespace,
                            node: key.origin.node,
                            slot: RouteInputSlot(which),
                        })
                        .copied()
                }),
                node_namespace: key.origin.namespace,
            })
            .collect::<Vec<_>>();
        Some(RouteSnapshot {
            file: state.file,
            path: state.progress.name.clone(),
            origin: state.origin,
            rule: state
                .origin
                .rule
                .and_then(|rule| state.rules.get(&rule))
                .cloned()
                .unwrap_or_default(),
            elapsed_ms: RouteTally(state.progress.started.elapsed().as_millis() as u64),
            ticks: state.progress.ticks,
            overflow: state.overflow,
            hottest: document(&hottest),
            counters: document(&counters),
            operation,
            input_cardinalities,
            row_file: state.progress.row_file.clone(),
        })
    })
}
pub(crate) fn flush(stage: RouteLabel) {
    let Some(snapshot) = snapshot() else {
        return;
    };
    let project = snapshot.file == RouteFileIndex(usize::MAX);
    let phase = RouteLabel(if project {
        constants::PHASE_PROJECT
    } else {
        constants::PHASE_FILE
    });
    let file = (!project).then_some(snapshot.file.0);
    let row_file = snapshot.row_file.map(|known| (known.file.0, known.path.0));
    let origin = snapshot.origin;
    let (stage, phase) = (stage.0, phase.0);
    let (path, rule, operation) = (snapshot.path.0, snapshot.rule.0, snapshot.operation.0);
    let (elapsed_ms, ticks, overflow) =
        (snapshot.elapsed_ms.0, snapshot.ticks.0, snapshot.overflow.0);
    let (hottest, counters) = (snapshot.hottest, snapshot.counters);
    let input_cardinalities = snapshot
        .input_cardinalities
        .map(|size| size.map(|size| size.0));
    tracing::trace!(target: constants::ROW_PROGRESS_TARGET, event = constants::ROW_PROGRESS, stage, phase, file = ?file, path, row_file = ?row_file, rule, node_namespace = origin.namespace.0, rule_index = ?origin.rule.map(|rule| rule.0), node = ?origin.node.map(|node| node.0), consumer = ?origin.consumer.map(|consumer| consumer.0), elapsed_ms, ticks, overflow, hottest = %hottest, counters = %counters, operation = operation, input_cardinalities = ?input_cardinalities);
}
