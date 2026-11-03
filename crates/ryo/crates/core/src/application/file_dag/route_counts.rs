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
    #[cfg(test)]
    Fixture,
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
            #[cfg(test)]
            Self::Fixture => "fixture_build",
        }
    }
}

const MAX_KEYS: usize = 32_768;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct Origin {
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
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

pub(crate) struct FileScope {
    active: bool,
}
impl FileScope {
    pub(crate) fn begin(file: usize) -> Self {
        let enabled =
            tracing::enabled!(target: "ryo::diagnostic::resource_routes", tracing::Level::TRACE);
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
        if let Some(state) = STATE.with_borrow_mut(Option::take) {
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
    scope(|origin| origin.node = Some(node))
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
    }
}
#[cfg(test)]
#[path = "route_counts_test.rs"]
mod tests;
