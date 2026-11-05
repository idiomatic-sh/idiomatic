//! Opt-in diagnostic accounting; never participates in execution or admission decisions.
use std::{cell::RefCell, collections::HashMap, fmt::Write};

/// Explicit caller route survives delegation; only the physical build site increments it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BuildRoute {
    OwnerAcquire,
    SelectorCandidate,
    Column,
    SelectorPredicate,
    SelectorPostings,
    SelectorDistinct,
    ClosurePartition,
    GraphAdjacency,
    GraphDomain,
}
impl BuildRoute {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OwnerAcquire => "owner_build",
            Self::SelectorCandidate => "selector_candidate_build",
            Self::Column => "column_build",
            Self::SelectorPredicate => "selector_predicate_build",
            Self::SelectorPostings => "selector_postings_build",
            Self::SelectorDistinct => "selector_distinct_build",
            Self::ClosurePartition => "closure_partition_build",
            Self::GraphAdjacency => "graph_adjacency_build",
            Self::GraphDomain => "graph_domain_build",
        }
    }
}

const MAX_KEYS: usize = 32_768;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct Origin {
    #[cfg(feature = "diagnostic-row-progress")]
    namespace: &'static str,
    rule: Option<usize>,
    node: Option<usize>,
    owner: bool,
    owner_id: Option<usize>,
    consumer: Option<&'static str>,
    plan: Option<usize>,
    populations: [Option<usize>; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    origin: Origin,
    route: &'static str,
    family: &'static str,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Count {
    calls: u64,
    successes: u64,
    failures: u64,
}
struct State {
    file: usize,
    origin: Origin,
    counts: HashMap<Key, Count>,
    overflow: u64,
    rules: HashMap<usize, String>,
    #[cfg(feature = "diagnostic-row-progress")]
    progress: ProgressState,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

pub(crate) struct FileScope {
    active: bool,
}
impl FileScope {
    pub(crate) fn begin(file: usize) -> Self {
        let enabled =
            tracing::enabled!(target: "ryo::diagnostic::resource_routes", tracing::Level::TRACE);
        #[cfg(feature = "diagnostic-row-progress")]
        let enabled = enabled
            || tracing::enabled!(target: "ryo::diagnostic::row_progress", tracing::Level::TRACE);
        let active = enabled
            && STATE.with_borrow_mut(|state| {
                if let Some(parent) = state {
                    // A nested file cannot be attributed by this bounded synchronous profile.
                    // Mark the aggregate incomplete rather than changing execution behavior.
                    parent.overflow += 1;
                    return false;
                }
                *state = Some(State {
                    file,
                    origin: Origin::default(),
                    counts: HashMap::new(),
                    overflow: 0,
                    rules: HashMap::new(),
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress: ProgressState::new(),
                });
                true
            });
        Self { active }
    }
}
impl Drop for FileScope {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        #[cfg(feature = "diagnostic-row-progress")]
        progress_flush("file_end");
        if let Some(state) = STATE.with_borrow_mut(Option::take) {
            if !tracing::enabled!(target: "ryo::diagnostic::resource_routes", tracing::Level::TRACE)
            {
                return;
            }
            let mut rows = state.counts.into_iter().collect::<Vec<_>>();
            rows.sort_by_key(|(key, _)| {
                (
                    key.origin.rule,
                    key.origin.node,
                    key.origin.owner,
                    key.origin.owner_id,
                    key.origin.consumer,
                    key.origin.plan,
                    key.origin.populations,
                    key.route,
                    key.family,
                )
            });
            let mut counts = String::from("[");
            for (index, (key, count)) in rows.into_iter().enumerate() {
                if index > 0 {
                    counts.push(',');
                }
                let rule = key
                    .origin
                    .rule
                    .map_or_else(|| "null".to_owned(), |n| n.to_string());
                let node = key
                    .origin
                    .node
                    .map_or_else(|| "null".to_owned(), |n| n.to_string());
                let owner_id = key
                    .origin
                    .owner_id
                    .map_or_else(|| "null".to_owned(), |n| n.to_string());
                let consumer = key.origin.consumer.unwrap_or("unclassified");
                let plan = key
                    .origin
                    .plan
                    .map_or_else(|| "null".to_owned(), |n| n.to_string());
                let populations = key
                    .origin
                    .populations
                    .map(|n| n.map_or_else(|| "null".to_owned(), |n| n.to_string()));
                write!(&mut counts, "{{\"rule\":{rule},\"node\":{node},\"owner\":{},\"owner_id\":{owner_id},\"consumer\":\"{consumer}\",\"plan\":{plan},\"populations\":[{},{}],\"route\":\"{}\",\"family\":\"{}\",\"calls\":{},\"successes\":{},\"failures\":{}}}", key.origin.owner, populations[0], populations[1], key.route, key.family, count.calls, count.successes, count.failures).expect("writing a String cannot fail");
            }
            counts.push(']');
            let mut names = state.rules.into_iter().collect::<Vec<_>>();
            names.sort_by_key(|(index, _)| *index);
            let mut rules = String::from("[");
            for (position, (index, name)) in names.into_iter().enumerate() {
                if position > 0 {
                    rules.push(',');
                }
                write!(&mut rules, "{{\"index\":{index},\"name\":").expect("String write");
                json_string(&mut rules, &name);
                rules.push('}');
            }
            rules.push(']');
            tracing::trace!(target: "ryo::diagnostic::resource_routes", event = "resource_routes", file = state.file, overflow = state.overflow, counts = %counts, rules = %rules);
        }
    }
}

fn json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                write!(out, "\\u{:04x}", c as u32).expect("String write");
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy)]
pub(crate) struct CapturedOrigin(Option<Origin>);
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn capture() -> CapturedOrigin {
    CapturedOrigin(STATE.with_borrow(|state| state.as_ref().map(|state| state.origin)))
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn resume(origin: CapturedOrigin) -> OriginScope {
    scope(|current| {
        if let Some(origin) = origin.0 {
            *current = origin;
        }
    })
}
pub(crate) struct OriginScope {
    previous: Option<Origin>,
}
fn scope(change: impl FnOnce(&mut Origin)) -> OriginScope {
    let previous = STATE.with_borrow_mut(|state| {
        state.as_mut().map(|state| {
            let old = state.origin;
            change(&mut state.origin);
            old
        })
    });
    OriginScope { previous }
}
pub(crate) fn rule(rule: usize, name: &str) -> OriginScope {
    STATE.with_borrow_mut(|state| {
        if let Some(state) = state {
            if !state.rules.contains_key(&rule) {
                if state.rules.len() < MAX_KEYS {
                    state.rules.insert(rule, name.to_owned());
                } else {
                    state.overflow += 1;
                }
            }
        }
    });
    scope(|origin| {
        origin.rule = Some(rule);
        origin.node = None;
    })
}
pub(crate) fn node(node: usize) -> OriginScope {
    scope(|origin| {
        origin.node = Some(node);
        #[cfg(feature = "diagnostic-row-progress")]
        {
            origin.namespace = "dag";
        }
    })
}
pub(crate) fn consumer(consumer: &'static str) -> OriginScope {
    scope(|origin| origin.consumer = Some(consumer))
}
pub(crate) fn identity(plan: usize, populations: [Option<usize>; 2]) -> OriginScope {
    scope(|origin| {
        origin.plan = Some(plan);
        origin.populations = populations;
    })
}
pub(crate) fn owner(owner_id: usize) -> OriginScope {
    scope(|origin| {
        origin.owner = true;
        origin.owner_id = Some(owner_id);
    })
}
impl Drop for OriginScope {
    fn drop(&mut self) {
        if let Some(old) = self.previous {
            STATE.with_borrow_mut(|state| {
                if let Some(state) = state {
                    state.origin = old;
                }
            });
        }
    }
}
fn update(route: &'static str, family: &'static str, stage: u8) {
    STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return;
        };
        let key = Key {
            origin: state.origin,
            route,
            family,
        };
        if !state.counts.contains_key(&key) && state.counts.len() == MAX_KEYS {
            state.overflow += 1;
            return;
        }
        let count = state.counts.entry(key).or_default();
        match stage {
            0 => count.calls += 1,
            1 => count.successes += 1,
            _ => count.failures += 1,
        }
    });
}
pub(crate) fn event(route: &'static str, family: &'static str) {
    update(route, family, 0);
}
pub(crate) fn observe<T, E>(
    route: &'static str,
    family: &'static str,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    update(route, family, 0);
    let result = run();
    update(route, family, if result.is_ok() { 1 } else { 2 });
    result
}
pub(crate) fn derived(kind: ryo_derived::domain::contract::types::DerivedKind) -> &'static str {
    use ryo_derived::domain::contract::types::DerivedKind::*;
    match kind {
        Equality => "Equality",
        PredicateMatches => "PredicateMatches",
        CandidatePostings => "CandidatePostings",
        DistinctRepresentatives => "DistinctRepresentatives",
        Adjacency => "Adjacency",
        Membership => "Membership",
        PartitionOrder => "PartitionOrder",
        ClosureAdjacency => "ClosureAdjacency",
        CycleAdjacency => "CycleAdjacency",
        OrderedKeys => "OrderedKeys",
        OrderedDfs => "OrderedDfs",
        NativeLookup => "NativeLookup",
        RawKeyOrder => "RawKeyOrder",
    }
}
pub(crate) fn representation(
    kind: crate::domain::table_model::collections::representations::types::RepresentationKind,
) -> &'static str {
    use crate::domain::table_model::collections::representations::types::RepresentationKind::*;
    match kind {
        Stored => "Stored",
        Selected => "Selected",
        Tree => "Tree",
        Computed => "Computed",
        Compact => "Compact",
        Positioned => "Positioned",
    }
}
#[cfg(test)]
#[path = "route_counts_test.rs"]
mod tests;

#[cfg(feature = "diagnostic-row-progress")]
struct ProgressState {
    started: std::time::Instant,
    last: std::time::Instant,
    ticks: u64,
    name: String,
    nodes: HashMap<(Option<usize>, &'static str, usize), String>,
    row_file: Option<(u32, String)>,
    inputs: HashMap<(Option<usize>, &'static str, Option<usize>, usize), usize>,
}
#[cfg(feature = "diagnostic-row-progress")]
impl ProgressState {
    fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            started: now,
            last: now,
            ticks: 0,
            name: String::new(),
            nodes: HashMap::new(),
            row_file: None,
            inputs: HashMap::new(),
        }
    }
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn file_name(name: &str) {
    STATE.with_borrow_mut(|state| {
        if let Some(state) = state {
            state.progress.name = name.to_owned();
        }
    });
    progress_flush("scope_start");
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn row_file(
    row: &[ryo_relations::domain::relations::types::Value],
    paths: &[ryo_language::domain::files::types::FilePath],
) {
    let file = row.iter().find_map(|value| match value {
        ryo_relations::domain::relations::types::Value::File(file) => Some(file.0),
        _ => None,
    });
    STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return;
        };
        match file {
            Some(file) if state.progress.row_file.as_ref().map(|(id, _)| *id) != Some(file) => {
                state.progress.row_file = paths
                    .get(file as usize)
                    .map(|path| (file, path.0.to_string()));
            }
            None => state.progress.row_file = None,
            _ => {}
        }
    });
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn input_cardinality(which: usize, rows: usize) {
    STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return;
        };
        let key = (
            state.origin.rule,
            state.origin.namespace,
            state.origin.node,
            which,
        );
        if state.progress.inputs.contains_key(&key) || state.progress.inputs.len() < MAX_KEYS {
            state.progress.inputs.insert(key, rows);
        } else {
            state.overflow += 1;
        }
    });
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn node_description(operation: &ryo_values::domain::plans::types::Operation) {
    STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return;
        };
        let Some(node) = state.origin.node else {
            return;
        };
        if !state
            .progress
            .nodes
            .contains_key(&(state.origin.rule, state.origin.namespace, node))
            && state.progress.nodes.len() < MAX_KEYS
        {
            let text = format!("{operation:?}");
            state.progress.nodes.insert(
                (state.origin.rule, state.origin.namespace, node),
                text.chars().take(8192).collect(),
            );
        }
    });
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn progress(
    counter: crate::domain::operator::contract::views::ProgressCounter,
    amount: u64,
) {
    use crate::domain::operator::contract::views::ProgressCounter;
    let label = match counter {
        ProgressCounter::InputRows => "input_rows",
        ProgressCounter::EmittedRows => "emitted_rows",
        ProgressCounter::PredicateEvaluations => "predicate_evaluations",
        ProgressCounter::IndexProbes => "index_probes",
        ProgressCounter::IndexRowsExamined => "index_chain_rows_examined",
        ProgressCounter::EqualityMatches => "equality_matches",
        ProgressCounter::JoinCandidatePairs => "join_candidate_pairs",
        ProgressCounter::KernelOutputRows => "kernel_output_rows",
        ProgressCounter::PositionRowsYielded => "position_rows_yielded",
    };
    let flush = STATE.with_borrow_mut(|state| {
        let Some(state) = state else {
            return false;
        };
        let key = Key {
            origin: state.origin,
            route: "row_progress",
            family: label,
        };
        if !state.counts.contains_key(&key) && state.counts.len() == MAX_KEYS {
            state.overflow += amount;
        } else {
            state.counts.entry(key).or_default().calls += amount;
        }
        state.progress.ticks += amount;
        if state.progress.ticks == amount
            || (state.progress.ticks & 4095 == 0 && state.progress.last.elapsed().as_secs() >= 1)
        {
            state.progress.last = std::time::Instant::now();
            true
        } else {
            false
        }
    });
    if flush {
        progress_flush("progress");
    }
}
#[cfg(feature = "diagnostic-row-progress")]
fn progress_flush(stage: &'static str) {
    let snapshot = STATE.with_borrow(|state| {
        let state = state.as_ref()?;
        let mut rows = state
            .counts
            .iter()
            .filter(|(key, _)| key.route == "row_progress")
            .collect::<Vec<_>>();
        rows.sort_by_key(|(key, count)| {
            (
                std::cmp::Reverse(count.calls),
                key.origin.rule,
                key.origin.node,
                key.family,
            )
        });
        let mut current = [0_u64; 9];
        let labels = [
            "input_rows",
            "emitted_rows",
            "predicate_evaluations",
            "index_probes",
            "index_chain_rows_examined",
            "equality_matches",
            "join_candidate_pairs",
            "kernel_output_rows",
            "position_rows_yielded",
        ];
        for (key, count) in &state.counts {
            if key.route == "row_progress"
                && key.origin.rule == state.origin.rule
                && key.origin.node == state.origin.node
                && key.origin.namespace == state.origin.namespace
            {
                if let Some(i) = labels.iter().position(|label| *label == key.family) {
                    current[i] += count.calls;
                }
            }
        }
        let mut current_json = String::from("{");
        for (i, label) in labels.iter().enumerate() {
            if i != 0 {
                current_json.push(',');
            }
            write!(&mut current_json, "\"{label}\":{}", current[i]).expect("String write");
        }
        current_json.push('}');
        let input_cardinalities: [Option<usize>; 3] = std::array::from_fn(|which| {
            state
                .progress
                .inputs
                .get(&(
                    state.origin.rule,
                    state.origin.namespace,
                    state.origin.node,
                    which,
                ))
                .copied()
        });
        let operation = state
            .origin
            .node
            .and_then(|n| {
                state
                    .progress
                    .nodes
                    .get(&(state.origin.rule, state.origin.namespace, n))
            })
            .cloned()
            .unwrap_or_default();
        let mut hottest = String::from("[");
        for (i, (key, count)) in rows.into_iter().take(16).enumerate() {
            if i != 0 {
                hottest.push(',');
            }
            write!(
                &mut hottest,
                "{{\"rule\":{},\"node\":{},\"counter\":\"{}\",\"count\":{},\"rule_name\":",
                key.origin
                    .rule
                    .map_or_else(|| "null".into(), |n| n.to_string()),
                key.origin
                    .node
                    .map_or_else(|| "null".into(), |n| n.to_string()),
                key.family,
                count.calls
            )
            .expect("String write");
            json_string(
                &mut hottest,
                key.origin
                    .rule
                    .and_then(|r| state.rules.get(&r))
                    .map_or("<none>", String::as_str),
            );
            hottest.push_str(",\"input_cardinalities\":[");
            for which in 0..3 {
                if which != 0 {
                    hottest.push(',');
                }
                if let Some(size) = state.progress.inputs.get(&(
                    key.origin.rule,
                    key.origin.namespace,
                    key.origin.node,
                    which,
                )) {
                    write!(&mut hottest, "{size}").expect("String write");
                } else {
                    hottest.push_str("null");
                }
            }
            hottest.push(']');
            hottest.push_str(",\"node_namespace\":");
            json_string(&mut hottest, key.origin.namespace);
            hottest.push('}');
        }
        hottest.push(']');
        Some((
            state.file,
            state.progress.name.clone(),
            state.origin,
            state
                .origin
                .rule
                .and_then(|r| state.rules.get(&r))
                .cloned()
                .unwrap_or_default(),
            state.progress.started.elapsed().as_millis() as u64,
            state.progress.ticks,
            state.overflow,
            hottest,
            current_json,
            operation,
            input_cardinalities,
            state.progress.row_file.clone(),
        ))
    });
    if let Some((
        file,
        path,
        origin,
        rule,
        elapsed_ms,
        ticks,
        overflow,
        hottest,
        current,
        operation,
        input_cardinalities,
        row_file,
    )) = snapshot
    {
        let phase = if file == usize::MAX {
            "project"
        } else {
            "file"
        };
        let file = (file != usize::MAX).then_some(file);
        tracing::trace!(target: "ryo::diagnostic::row_progress", event = "row_progress", stage, phase, file = ?file, path, row_file = ?row_file, rule, node_namespace = origin.namespace, rule_index = ?origin.rule, node = ?origin.node, consumer = ?origin.consumer, elapsed_ms, ticks, overflow, hottest = %hottest, counters = %current, operation, input_cardinalities = ?input_cardinalities);
    }
}
