//! Select a positive guard from the cost of its typed emit. Both experiment
//! arms use the ordinary formula evaluator and the ordinary exact certificates.
use super::{frontier, learning, prices, Id, Need, Program, ANY};
use rustc_hash::FxHashMap as Map;
use ryo_values::domain::finish_plans::types::FinishPlan;
use std::{cell::RefCell, rc::Rc};

pub(super) fn prepare(p:&mut Program, plan:&FinishPlan) {
    let policy=std::env::var("P8_POLICY").unwrap_or_default();
    if policy!="static" && policy!="adaptive" { return; }
    let mut protected=vec![false;p.roots.len()];
    for (r,_) in &p.exact { protected[*r]=true; }
    let mut seen=vec![0;p.needs.len()];
    let mut clauses=Vec::with_capacity(p.roots.len());
    for (r,(_,root)) in p.roots.iter().enumerate() {
        let mut tests=Vec::new();
        if !protected[r] {
            frontier::conjuncts(p,*root,&mut seen,r+1,&mut tests);
            tests.retain(|&t| t!=ANY);
        }
        clauses.push(tests);
    }
    if policy=="adaptive" {
        p.file_fees=(0..p.needs.len()).map(|i|super::feedback::operation(super::cost::file_family(p,i))).collect();
        p.node_fees=p.domains.iter().map(|d|super::feedback::operation(super::cost::family(d,&p.facts))).collect();
        p.learning=Some(Rc::new(RefCell::new(learning::Learning {
            names:p.roots.iter().enumerate().map(|(r,(name,_))|(name.clone(),r)).collect(),
            active:true,
            rules:clauses.into_iter().zip(protected).map(|(v,proof)| learning::Rule::new(v,proof)).collect(),
        })));
        return;
    }
    let budgets=frontier::exits(plan);
    let mut prices_by_group=frontier::Quote::new(p);
    let mut replacements=Vec::new();
    for (r,mut tests) in clauses.into_iter().enumerate() {
        if protected[r] { continue; }
        let limit=prices::FIXED.saturating_add(prices::STEP.saturating_mul(budgets[r]));
        let count=tests.len();
        tests.retain(|&test|prices_by_group.file(test)<=limit);
        if tests.len()!=count { replacements.push((r,tests)); }
    }
    let mut shared=Map::<Vec<Id>,Id>::default();
    for (r,tests) in replacements {
        p.roots[r].1=match &tests[..] {
            []=>ANY,
            [only]=>*only,
            _=>*shared.entry(tests).or_insert_with_key(|set| {
                let k=p.needs.len();
                p.needs.push(Need::Terms(true,set.clone().into_boxed_slice())); k
            }),
        };
    }
}

