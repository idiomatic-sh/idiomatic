//! Route counts attribute each counted call to its rule, node, owner and plan, bound the keys, and
//! count each physical build once.
use super::constants::{
    GRAPH_ADJACENCY_BUILD, KIND_EQUALITY, KIND_MEMBERSHIP, MAX_KEYS, OWNER_BUILD,
    REPRESENTATION_STORED,
};
use super::types::{RouteCount, RouteKey, RouteOrigin, RouteOwnership, RouteTally};
use crate::application::file_dag::derived::constants::{OWNER_HIT, OWNER_REQUEST};
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::route_counts::ids::service::RouteRuleIndex;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::{
    Answer, EXAMPLE_RULE, ORIGINAL_FAILURE, QUERY_NAME, QUERY_TWO, SCOPED_BUILD, Workload, capture,
    run_workload,
};
use std::borrow::Cow;

mod fixtures;

/// A recorded owner or plan id: one the workload named, or one taken from an allocation's
/// address, which differs from run to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Id {
    Named(usize),
    Allocated,
}

/// Where a route was counted: the rule and node, the owner and its id, the plan and populations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    rule: Option<usize>,
    node: Option<usize>,
    ownership: RouteOwnership,
    owner: Option<Id>,
    plan: Option<Id>,
    populations: [Option<usize>; 2],
}

/// One counted key: the route, its family, where it was counted, and its count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Route {
    route: &'static str,
    family: &'static str,
    place: Place,
    count: RouteCount,
}

/// What a recording holds after its workload: the counted keys (the first eight in order
/// listed), the overflow, the named rules, whether the origin is back to the default, and what
/// the calls answered.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Recorded {
    keys: usize,
    first: Cow<'static, [Route]>,
    overflow: RouteTally,
    rules: Cow<'static, [(RouteRuleIndex, Cow<'static, str>)]>,
    origin_restored: bool,
    answers: Cow<'static, [Answer]>,
}

type Input = Workload;
type Value = Recorded;
type Failure = String;

/// A count of `calls` calls, `successes` of them succeeding and `failures` failing.
const fn count(calls: u64, successes: u64, failures: u64) -> RouteCount {
    RouteCount {
        calls: RouteTally(calls),
        successes: RouteTally(successes),
        failures: RouteTally(failures),
    }
}

/// A place under `rule` at `node`, with no owner, plan or population.
const fn at(rule: Option<usize>, node: usize) -> Place {
    Place {
        rule,
        node: Some(node),
        ownership: RouteOwnership::Unowned,
        owner: None,
        plan: None,
        populations: [None, None],
    }
}

/// The same place, owned by `owner`.
const fn owned(place: Place, owner: Id) -> Place {
    Place {
        ownership: RouteOwnership::Owned,
        owner: Some(owner),
        ..place
    }
}

/// The same place, reading `plan` over `population`.
const fn planned(place: Place, plan: Id, population: usize) -> Place {
    Place {
        plan: Some(plan),
        populations: [Some(population), None],
        ..place
    }
}

/// `route` of `family`, counted at `place`.
const fn counted(
    route: &'static str,
    family: &'static str,
    place: Place,
    count: RouteCount,
) -> Route {
    Route {
        route,
        family,
        place,
        count,
    }
}

/// The id as a row states it: ids beyond 32 bits are allocation addresses.
fn id(id: usize) -> Id {
    if u32::try_from(id).is_ok() {
        Id::Named(id)
    } else {
        Id::Allocated
    }
}

/// A counted key as a row states it.
fn route((key, count): (RouteKey, RouteCount)) -> Route {
    let origin = key.origin;
    Route {
        route: key.route.0,
        family: key.family.0,
        place: Place {
            rule: origin.rule.map(|rule| rule.0),
            node: origin.node.map(|node| node.0),
            ownership: origin.owner,
            owner: origin.owner_id.map(|owner| id(owner.0)),
            plan: origin.plan.map(|plan| id(plan.0)),
            populations: origin
                .populations
                .map(|population| population.map(|id| id.0)),
        },
        count,
    }
}

/// Rule 3 at node 19, owned by owner 7.
const NESTED: Place = owned(at(Some(3), 19), Id::Named(7));
/// Rule 2 at node 41, owned by owner 17.
const OWNER_17: Place = owned(at(Some(2), 41), Id::Named(17));

/// Rule 2 at node 41, owned by an allocated owner reading an allocated plan over population 0.
const SELECTED: Place = planned(owned(at(Some(2), 41), Id::Allocated), Id::Allocated, 0);
/// The route that inspects a stored representation, as the inspection counts it.
const REPRESENTATION_INSPECT: &str = "representation_inspect";

/// An unowned owner request at `node` outside any rule.
const fn request(node: usize) -> Route {
    counted(OWNER_REQUEST, KIND_EQUALITY, at(None, node), count(1, 0, 0))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("physical_build_is_once_and_original_error_survives_nested_scopes"),
        word: Word::Succeeds,
        input: || Workload::FailedBuildInNestedScopes,
        expect: Expect::Value(Recorded {
            keys: 3,
            first: Cow::Borrowed(&[
                counted(OWNER_HIT, KIND_EQUALITY, NESTED, count(1, 0, 0)),
                counted(OWNER_REQUEST, KIND_EQUALITY, NESTED, count(1, 0, 0)),
                counted(SCOPED_BUILD, KIND_EQUALITY, NESTED, count(1, 0, 1)),
            ]),
            overflow: RouteTally(0),
            rules: Cow::Borrowed(&[(RouteRuleIndex(3), Cow::Borrowed(EXAMPLE_RULE))]),
            origin_restored: true,
            answers: Cow::Borrowed(&[Answer::Failed(ORIGINAL_FAILURE)]),
        }),
    },
    Case {
        name: CaseName("cross_rule_origin_is_attributed_and_disabled_recording_allocates_no_state"),
        word: Word::Succeeds,
        input: || Workload::BuildUnderTwoRules,
        expect: Expect::Value(Recorded {
            keys: 2,
            first: Cow::Borrowed(&[
                counted(
                    SCOPED_BUILD,
                    KIND_MEMBERSHIP,
                    at(Some(1), 41),
                    count(1, 1, 0),
                ),
                counted(
                    SCOPED_BUILD,
                    KIND_MEMBERSHIP,
                    at(Some(2), 41),
                    count(1, 1, 0),
                ),
            ]),
            overflow: RouteTally(0),
            rules: Cow::Borrowed(&[
                (RouteRuleIndex(1), Cow::Borrowed(EXAMPLE_RULE)),
                (RouteRuleIndex(2), Cow::Borrowed(EXAMPLE_RULE)),
            ]),
            origin_restored: true,
            answers: Cow::Borrowed(&[Answer::Recording(false)]),
        }),
    },
    Case {
        name: CaseName("bounded_capture_reports_overflow_instead_of_claiming_complete_counts"),
        word: Word::Succeeds,
        input: || Workload::RequestsPastTheKeyBound,
        expect: Expect::Value(Recorded {
            keys: MAX_KEYS,
            first: Cow::Borrowed(&[
                request(0),
                request(1),
                request(2),
                request(3),
                request(4),
                request(5),
                request(6),
                request(7),
            ]),
            overflow: RouteTally(3),
            rules: Cow::Borrowed(&[]),
            origin_restored: true,
            answers: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName(
            "owner_identity_distinguishes_live_population_keys_and_rules_are_named_once",
        ),
        word: Word::Succeeds,
        input: || Workload::TwoLivePopulations,
        expect: Expect::Value(Recorded {
            keys: 2,
            first: Cow::Borrowed(&[
                counted(
                    OWNER_REQUEST,
                    KIND_MEMBERSHIP,
                    planned(OWNER_17, Id::Named(29), 3),
                    count(1, 0, 0),
                ),
                counted(
                    OWNER_REQUEST,
                    KIND_MEMBERSHIP,
                    planned(OWNER_17, Id::Named(29), 4),
                    count(1, 0, 0),
                ),
            ]),
            overflow: RouteTally(0),
            rules: Cow::Borrowed(&[(RouteRuleIndex(2), Cow::Borrowed(QUERY_NAME))]),
            origin_restored: true,
            answers: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("owner_routes_count_each_physical_equality_build_once"),
        word: Word::Succeeds,
        input: || Workload::EqualityBuiltByTwoOwners,
        expect: Expect::Value(Recorded {
            keys: 6,
            first: Cow::Borrowed(&[
                counted(
                    REPRESENTATION_INSPECT,
                    REPRESENTATION_STORED,
                    at(Some(2), 41),
                    count(2, 2, 0),
                ),
                counted(
                    GRAPH_ADJACENCY_BUILD,
                    KIND_EQUALITY,
                    SELECTED,
                    count(1, 1, 0),
                ),
                counted(OWNER_BUILD, KIND_EQUALITY, SELECTED, count(1, 1, 0)),
                counted(OWNER_HIT, KIND_EQUALITY, SELECTED, count(1, 0, 0)),
                counted(OWNER_REQUEST, KIND_EQUALITY, SELECTED, count(1, 0, 0)),
                counted(OWNER_REQUEST, KIND_EQUALITY, SELECTED, count(2, 0, 0)),
            ]),
            overflow: RouteTally(0),
            rules: Cow::Borrowed(&[(RouteRuleIndex(2), Cow::Borrowed(QUERY_TWO))]),
            origin_restored: true,
            answers: Cow::Borrowed(&[
                Answer::Equality(true),
                Answer::Constructed(Construction::Built),
                Answer::Constructed(Construction::Reused),
            ]),
        }),
    },
];

/// Runs the workload under a recording profile and reads what it recorded.
fn subject(workload: Input) -> Result<Value, Failure> {
    let (state, answers) = capture(|profile| run_workload(profile, workload));
    let mut keys = state.counts.into_iter().map(route).collect::<Vec<_>>();
    keys.sort_by_key(|key| {
        let count = key.count;
        let tally = (count.calls.0, count.successes.0, count.failures.0);
        (key.place, key.route, key.family, tally)
    });
    let mut rules = state
        .rules
        .into_iter()
        .map(|(index, name)| (index, Cow::Owned(name.0)))
        .collect::<Vec<_>>();
    rules.sort();
    Ok(Recorded {
        keys: keys.len(),
        first: Cow::Owned(keys.into_iter().take(8).collect()),
        overflow: state.overflow,
        rules: Cow::Owned(rules),
        origin_restored: state.origin == RouteOrigin::default(),
        answers: Cow::Owned(answers?),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
