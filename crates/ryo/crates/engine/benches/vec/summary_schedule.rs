//! Lower associative file guards to short-circuit instruction lists. The
//! provider still supplies every fact and the original program owns each test.
use super::pair_summary::{Domain, Id, Need, Program};
use rustc_hash::FxHashSet;

// Enabled physical operators can be ablated independently in the harness.
pub(super) fn enabled(name: &str) -> bool {
    std::env::var_os(name).is_none_or(|value| value != "0")
}

pub(super) struct Schedule {
    pub(super) buckets: bool,
    pub(super) direct_masks: bool,
    pub(super) postings: Option<super::pair_checks::postings::Layout>,
    pub(super) ready_guards: bool,
    pub(super) literals: Option<super::summary_literals::Layout>,
    pub(super) proof_order: bool,
    pub(super) price: Vec<u8>,
    pub(super) rules: Option<super::summary_rules::Rules>,
    pub(super) byte_regex: bool,
    pub(super) groups: Option<super::summary_groups::Groups>,
    pub(super) scan: Option<super::pair_checks::scan::Layout>,
    pub(super) branches: Option<super::summary_branches::Branches>,
    pub(super) windows: bool,
    pub(super) classes: bool,
    pub(super) alias: Vec<Option<Id>>,
    pub(super) select: bool,
    pub(super) keys: Vec<Option<ryo::languages::Column>>,
    pub(super) existential: bool,
    pub(super) symbols: Option<Vec<Option<Id>>>,
    pub(super) cache_regex: bool,
    pub(super) paths: Vec<Option<super::summary_local::Path>>,
    pub(super) needs: Vec<Vec<Id>>,
    pub(super) cost: Vec<u8>,
}

impl Schedule {
    pub(super) fn new(p: &Program) -> Self {
        let mut cost: Vec<u8> = Vec::with_capacity(p.domains.len());
        for d in &p.domains {
            let c = match d {
                Domain::Empty
                | Domain::Any
                | Domain::Kind(_)
                | Domain::Field(_)
                | Domain::HasParent => 0,
                Domain::Text(_) => 1,
                Domain::TextMatching(..) | Domain::SpanMatching(..) => 3,
                Domain::Both(a, b) | Domain::Either(a, b) | Domain::Except(a, b) => {
                    cost[*a].max(cost[*b])
                }
                Domain::Children(x)
                | Domain::Parents(x)
                | Domain::Ancestors(x)
                | Domain::Descendants(x) => cost[*x].max(2),
                Domain::CountUnder(x, ..) => cost[*x].max(3),
                Domain::When(_, x) => cost[*x],
                Domain::ProvenExcept(a, b, ..) => cost[*a].max(cost[*b]).max(4),
                _ => 4,
            };
            cost.push(c);
        }
        let mut price = Vec::<u8>::with_capacity(p.needs.len());
        for n in &p.needs {
            price.push(match n {
                Need::Any | Need::Empty | Need::PathMatches(_) | Need::SourceContains(_) => 0,
                Need::Some(d) => cost[*d],
                Need::Both(a, b) | Need::Either(a, b) => price[*a].max(price[*b]),
            });
        }
        let mut todo: Vec<_> = p.roots.iter().map(|r| r.1).collect();
        todo.extend(p.exact.iter().map(|r| r.1.guard));
        for d in &p.domains {
            match d {
                Domain::When(need, _) | Domain::ProvenExcept(_, _, need, _) => todo.push(*need),
                _ => {}
            }
        }
        let mut done = vec![false; p.needs.len()];
        let mut needs = vec![Vec::new(); p.needs.len()];
        let proofs = enabled("VEC_IMPLIED_TESTS").then(|| super::summary_proofs::compile(p));
        while let Some(id) = todo.pop() {
            if std::mem::replace(&mut done[id], true) {
                continue;
            }
            let both = matches!(p.needs[id], Need::Both(..));
            if !both && !matches!(p.needs[id], Need::Either(..)) {
                continue;
            }
            let mut seen = FxHashSet::default();
            let mut stack = vec![id];
            let mut terms = Vec::new();
            while let Some(x) = stack.pop() {
                if !seen.insert(x) {
                    continue;
                }
                match p.needs[x] {
                    Need::Both(a, b) if both => stack.extend([a, b]),
                    Need::Either(a, b) if !both => stack.extend([a, b]),
                    _ => terms.push(x),
                }
            }
            if both && let Some(proofs) = &proofs {
                let mut redundant = FxHashSet::default();
                for n in &terms {
                    if let Need::Some(d) = p.needs[*n] {
                        redundant.extend(proofs.weaker[d].iter().copied());
                    }
                }
                terms.retain(|&n| !matches!(p.needs[n],Need::Some(d) if redundant.contains(&d)));
            }
            terms.sort_unstable_by_key(|&x| (price[x], x));
            todo.extend(terms.iter().copied());
            needs[id] = terms;
        }
        let select = std::env::var_os("VEC_SELECTED_NODES").is_some();
        let paths = if select || std::env::var_os("VEC_LOCAL_PROGRAM").is_some() {
            (0..p.domains.len())
                .map(|id| super::summary_local::Path::new(&p.domains, id, select))
                .collect()
        } else {
            Vec::new()
        };
        let branches = std::env::var_os("VEC_FILE_BRANCHES")
            .is_some()
            .then(|| super::summary_branches::Branches::new(p, &needs));
        let rules = enabled("VEC_SHARED_RULES")
            .then(|| super::summary_rules::Rules::new(p, &needs, &price));
        let groups = std::env::var_os("VEC_MASK_GROUPS")
            .is_some()
            .then(|| super::summary_groups::Groups::new(p, &cost));
        let scan = std::env::var_os("VEC_NODE_SCAN")
            .or_else(|| std::env::var_os("VEC_BORROW_BUCKETS"))
            .is_some()
            .then(|| super::pair_checks::scan::Layout::new(p));
        Self {
            buckets: std::env::var_os("VEC_BORROW_BUCKETS").is_some(),
            rules,
            byte_regex: enabled("VEC_BYTE_REGEX"),
            groups,
            scan,
            branches,
            windows: enabled("VEC_WORD_WINDOWS"),
            classes: std::env::var_os("VEC_NODE_CLASSES").is_some(),
            alias: proofs.map_or_else(Vec::new, |p| p.alias),
            select,
            keys: Vec::new(),
            existential: enabled("VEC_WITNESS_PROGRAM"),
            direct_masks: enabled("VEC_STREAM_MASKS"),
            postings: None,
            ready_guards: enabled("VEC_READY_GUARDS"),
            literals: None,
            cost,
            price,
            proof_order: enabled("VEC_PROOF_ORDER"),
            needs,
            paths,
            symbols: None,
            cache_regex: enabled("VEC_WORD_REGEX"),
        }
    }
}
