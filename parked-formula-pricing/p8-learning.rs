//! A run owns its rule counters. One ordered window learns a price per group.
use super::{prices, Id};
use std::{cell::RefCell, rc::Rc};

pub(crate) const WINDOW: u64 = 256;
pub(crate) type Shared = Rc<RefCell<Learning>>;

#[derive(Default)]
pub(crate) struct EmptyAsks {
    observations: u64,
    paid: u64,
    observed_cpu: f64,
}

#[derive(Default)]
enum Decision {
    #[default]
    Learning,
    Keep,
    Admit,
}

struct Group {
    test: Id,
    decision: Decision,
    empty_at_decision: Option<(u64,u64,f64)>,
    seen: u64,
    rejected: u64,
    paid: u64,
    observed_cpu: f64,
}

pub(crate) struct Rule {
    groups: Vec<Group>,
    empty: EmptyAsks,
    frozen: bool,
    unchanged: bool,
}

impl Rule {
    pub(super) fn new(tests: Vec<Id>, protected: bool) -> Self {
        let frozen = protected || tests.is_empty();
        Self {
            groups: tests.into_iter().map(|test| Group {
                test, decision: Decision::Learning, empty_at_decision:None, seen: 0, rejected: 0, paid: 0, observed_cpu: 0.,
            }).collect(),
            empty: EmptyAsks::default(),
            frozen, unchanged: true,
        }
    }

    pub(crate) fn admits(&mut self, root: Id, mut pass: impl FnMut(Id, bool) -> (bool,u64,f64)) -> bool {
        if self.frozen && self.unchanged {
            return pass(root,false).0;
        }
        let mut settled = false;
        let mut alive = true;
        for g in &mut self.groups {
            match g.decision {
                Decision::Keep => alive = pass(g.test, false).0,
                Decision::Admit => continue,
                Decision::Learning => {
                    // First apply any decision earned on PREVIOUS files. This file
                    // cannot train the decision that decides its own finding.
                    if g.seen >= WINDOW && self.empty.observations > 0 {
                        let cost = u128::from(g.paid) * u128::from(self.empty.observations);
                        let saving = u128::from(g.rejected) * u128::from(self.empty.paid);
                        settled = true;
                        g.empty_at_decision=Some((self.empty.observations,self.empty.paid,self.empty.observed_cpu));
                        if cost >= saving {
                            g.decision = Decision::Admit;
                            self.unchanged = false;
                            continue;
                        }
                        g.decision = Decision::Keep;
                        alive = pass(g.test,false).0;
                    } else {
                        let (yes,units,cpu) = pass(g.test,true);
                        g.seen += 1;
                        g.rejected += u64::from(!yes);
                        g.paid = g.paid.saturating_add(units);
                        g.observed_cpu += cpu;
                        alive = yes;
                    }
                }
            }
            if !alive { break; }
        }
        if settled && self.groups.iter().all(|g| !matches!(g.decision, Decision::Learning)) {
            self.frozen = true;
        }
        alive
    }

    pub(crate) fn observed_empty(&mut self, steps: u64, rows: u64, diagnostic_cpu: f64) {
        if self.frozen { return; }
        self.empty.observations += 1;
        self.empty.paid = self.empty.paid.saturating_add(prices::FIXED)
            .saturating_add(steps.saturating_mul(prices::PAID_STEP))
            .saturating_add(rows.saturating_mul(prices::OUTPUT_ROW));
        self.empty.observed_cpu += diagnostic_cpu;
    }
}

pub(crate) struct Learning {
    pub(crate) names: rustc_hash::FxHashMap<String,usize>,
    pub(crate) active: bool,
    pub(crate) rules: Vec<Rule>,
}
impl Learning {
    pub(crate) fn diagnostics(&self) -> String {
        let mut names=vec!["";self.rules.len()];
        for (name,&index) in &self.names { names[index]=name; }
        use std::fmt::Write;
        let mut out = String::new();
        for (name,r) in names.into_iter().zip(&self.rules) {
            for g in &r.groups {
                if g.seen == 0 { continue; }
                let decision = match g.decision { Decision::Learning=>"open", Decision::Keep=>"keep", Decision::Admit=>"admit" };
                let (samples,paid,cpu)=g.empty_at_decision.unwrap_or((r.empty.observations,r.empty.paid,r.empty.observed_cpu));
                writeln!(out,"D\t{name}\t{}\t{}\t{}\t{}\t{:.9}\t{}\t{}\t{:.9}\t{decision}",
                    g.test,g.seen,g.rejected,g.paid,g.observed_cpu,samples,paid,cpu).unwrap();
            }
        }
        out
    }
}

