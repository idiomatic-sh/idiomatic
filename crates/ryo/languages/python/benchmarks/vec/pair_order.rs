//! Physical short-circuit order of a fixed summary. A witness of an intersection
//! also witnesses its operands. A related node witnesses the input of the traversal.
use super::pair_summary::{Domain, Id, Need, Program};
use rustc_hash::{FxHashMap as Map, FxHashSet as Set};

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub(super) enum Bucket {
    Kind(u16),
    Field(u16),
}
pub(super) struct Order {
    pub measure: bool,
    pub masks: Vec<super::pair_checks::mask::Conjuncts>,
    pub buckets: Vec<Vec<Bucket>>,
    pub keys: Vec<Option<ryo::languages::Column>>,
    pub groups: Vec<Box<[Id]>>,
    pub costs: Vec<u8>,
    pub texts: Vec<bool>,
}

impl Order {
    pub fn new(p: &Program, keys: Vec<Option<ryo::languages::Column>>) -> Self {
        let mut costs: Vec<u8> = Vec::with_capacity(p.domains.len());
        let mut implies: Vec<Vec<Id>> = Vec::with_capacity(p.domains.len());
        let mut whole = Map::default();
        let mut kind = Map::default();
        for (id, d) in p.domains.iter().enumerate() {
            match d {
                Domain::Fact(slot, None) => {
                    whole.insert(*slot, id);
                }
                Domain::Kind(k) => {
                    kind.insert(*k, id);
                }
                _ => {}
            }
        }
        for d in &p.domains {
            // A witnessed domain proves these other domains have a witness.
            let mut evidence = Vec::new();
            let cost = match *d {
                Domain::Any | Domain::Empty | Domain::Kind(_) | Domain::Field(_) => 0,
                Domain::Text(_) => 1,
                Domain::TextMatching(k, _) => {
                    evidence.extend(kind.get(&k));
                    3
                }
                Domain::Parents(a) | Domain::Ancestors(a) | Domain::Children(a) => {
                    evidence.push(a);
                    evidence.extend_from_slice(&implies[a]);
                    costs[a].max(2)
                }
                Domain::Both(a, b) => {
                    evidence.extend([a, b]);
                    evidence.extend_from_slice(&implies[a]);
                    evidence.extend_from_slice(&implies[b]);
                    costs[a].max(costs[b]).max(1)
                }
                Domain::Either(a, b) => {
                    evidence.extend(
                        implies[a]
                            .iter()
                            .copied()
                            .filter(|x| implies[b].binary_search(x).is_ok()),
                    );
                    costs[a].max(costs[b]).max(1)
                }
                Domain::Fact(slot, Some(_)) | Domain::Matching(slot, _, _) => {
                    evidence.extend(whole.get(&slot));
                    5
                }
                Domain::Fact(_, None) => 4,
            };
            evidence.sort_unstable();
            evidence.dedup();
            costs.push(cost);
            implies.push(evidence);
        }
        let mut groups: Vec<Box<[Id]>> = vec![Box::default(); p.needs.len()];
        let mut done = vec![false; p.needs.len()];
        let mut price = vec![0; p.needs.len()];
        let mut open: Vec<_> = p.roots.iter().map(|r| r.1).collect();
        while let Some(root) = open.pop() {
            if std::mem::replace(&mut done[root], true) {
                continue;
            }
            let all = matches!(p.needs[root], Need::Both(..));
            if !all && !matches!(p.needs[root], Need::Either(..)) {
                continue;
            }
            let mut terms = Vec::new();
            let mut visited = Set::default();
            let mut stack = vec![root];
            while let Some(i) = stack.pop() {
                if !visited.insert(i) {
                    continue;
                }
                match p.needs[i] {
                    Need::Both(a, b) if all => stack.extend([a, b]),
                    Need::Either(a, b) if !all => stack.extend([a, b]),
                    _ => terms.push(i),
                }
            }
            if all {
                let mut redundant = Set::default();
                for &n in &terms {
                    if let Need::Some(d) = p.needs[n] {
                        redundant.extend(implies[d].iter().copied());
                    }
                }
                terms.retain(|&n| !matches!(p.needs[n],Need::Some(d) if redundant.contains(&d)));
            }
            open.extend(terms.iter().copied());
            groups[root] = terms.into();
        }
        for (i, n) in p.needs.iter().enumerate() {
            price[i] = match *n {
                Need::Any | Need::Empty | Need::PathMatches(_) => 0,
                Need::Some(d) => costs[d],
                Need::Both(..) | Need::Either(..) => {
                    groups[i].sort_unstable_by_key(|&x| (price[x], x));
                    groups[i].iter().map(|&x| price[x]).max().unwrap_or(0)
                }
            }
        }
        let mut texts = vec![false; 1 + p.text_kinds.iter().copied().max().unwrap_or(0) as usize];
        for &k in &p.text_kinds {
            texts[k as usize] = true;
        }
        let mut buckets: Vec<Vec<Bucket>> = Vec::with_capacity(p.domains.len());
        for d in &p.domains {
            let list = match *d {
                Domain::Kind(k) | Domain::TextMatching(k, _) => vec![Bucket::Kind(k)],
                Domain::Field(f) => vec![Bucket::Field(f)],
                Domain::Both(a, b) => {
                    let mut v = buckets[a].clone();
                    v.extend_from_slice(&buckets[b]);
                    v.sort_unstable();
                    v.dedup();
                    v
                }
                _ => Vec::new(),
            };
            buckets.push(list);
        }
        Self {
            masks: super::pair_checks::mask::compile(p),
            measure: std::env::var_os("VEC_SUMMARY_METRICS").is_some(),
            groups,
            costs,
            texts,
            buckets,
            keys,
        }
    }
}
