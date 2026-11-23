//! Compare the file-static emptiness proof to each decision at its actual time of use.
//! Keep size estimates separate from decisions that skip a whole or a keyed execution.
use super::{Exec, File, Res, known};
use std::cell::RefCell;

#[derive(Clone, Copy, Default)]
struct Seen {
    asks: u8,
    yes: bool,
    no: bool,
    keyed_ran: bool,
    unmatched_inputs: Option<(usize, usize)>,
}

#[derive(Default)]
struct Audit {
    file: Option<u32>,
    seen: Vec<Seen>,
    // For each use: false, static yes, late yes, any earlier false then yes,
    // earlier actual run then yes, earlier yes then false.
    counts: [[u64; 6]; 3],
    example: [[bool; 3]; 3],
    joins: std::collections::BTreeMap<usize, JoinWitness>,
}

#[derive(Default)]
struct JoinWitness {
    // Same pure equijoin, two nonempty stored inputs, output empty or productive.
    cases: [Option<(u32, usize, usize, usize)>; 2],
    counts: [u64; 2],
    used_as_empty: u64,
    consumer: Option<usize>,
}

thread_local! { static AUDIT: RefCell<Audit> = RefCell::default(); }

impl Audit {
    fn enter_file(&mut self, f: &File<'_>) {
        if self.file != Some(f.file) {
            self.file = Some(f.file);
            self.seen.resize(f.memo.len(), Seen::default());
            self.seen.fill(Seen::default());
        }
    }
}

pub(super) fn observe(
    ex: &Exec<'_>,
    f: &File<'_>,
    node: usize,
    usage: known::AskUse,
    answer: bool,
) {
    AUDIT.with_borrow_mut(|a| {
        a.enter_file(f);
        let previous = a.seen[node];
        let role = usage as usize;
        let static_yes = known::proved(f.known[node]);
        let category = if !answer {
            0
        } else if static_yes {
            1
        } else {
            2
        };
        a.counts[role][category] += 1;
        a.counts[role][3] += u64::from(answer && previous.no);
        a.counts[role][4] += u64::from(answer && previous.keyed_ran);
        a.counts[role][5] += u64::from(!answer && previous.yes);
        if answer && !static_yes {
            let inputs = &ex.plan.nodes[node].inputs;
            let mut counted = |i: usize| {
                if let Some(pair) = a.seen[i].unmatched_inputs
                    && let Some(j) = a.joins.get_mut(&i)
                {
                    if j.consumer.is_none() {
                        j.consumer = Some(node);
                        j.cases[0] = Some((f.file, pair.0, pair.1, 0));
                    }
                    j.used_as_empty += 1;
                }
            };
            match ex.frames[node].op {
                super::frame::Op::Select => {
                    if let Some(&i) = inputs
                        .iter()
                        .find(|&&i| matches!(&f.memo[i],Some(Res::Tab(t)) if t.n==0))
                    {
                        counted(i);
                    }
                }
                super::frame::Op::Union => inputs.iter().copied().for_each(counted),
                _ => counted(ex.frames[node].first as usize),
            }
            let kind = if previous.keyed_ran {
                2
            } else if previous.no {
                1
            } else {
                0
            };
            if !a.example[role][kind] {
                a.example[role][kind] = true;
                let inputs: Vec<_> = ex.plan.nodes[node]
                    .inputs
                    .iter()
                    .map(|&i| {
                        let rows = match &f.memo[i] {
                            Some(Res::Tab(t)) => Some(t.n),
                            _ => None,
                        };
                        (i, ex.plan.nodes[i].op.as_str(), f.known[i], rows)
                    })
                    .collect();
                eprintln!(
                    "ASK_WITNESS\t{}\t{node}\t{}\t{role}\t{kind}\t{}\t{inputs:?}",
                    f.file, ex.plan.nodes[node].op, previous.asks
                );
            }
        }
        let seen = &mut a.seen[node];
        seen.asks |= 1 << role;
        seen.no |= !answer;
        seen.yes |= answer;
    });
}

pub(super) fn whole_ran(ex: &Exec<'_>, f: &File<'_>, node: usize, rows: usize) {
    if !matches!(ex.col.operator(node), super::col::Operator::Join(_)) {
        return;
    }
    let &[left, right] = ex.plan.nodes[node].inputs.as_slice() else {
        return;
    };
    let (Some(Res::Tab(l)), Some(Res::Tab(r))) = (&f.memo[left], &f.memo[right]) else {
        return;
    };
    if l.n == 0 || r.n == 0 {
        return;
    }
    let i = usize::from(rows != 0);
    AUDIT.with_borrow_mut(|a| {
        a.enter_file(f);
        if rows == 0 {
            a.seen[node].unmatched_inputs = Some((l.n, r.n));
        }
        let j = a.joins.entry(node).or_default();
        j.counts[i] += 1;
        j.cases[i].get_or_insert((f.file, l.n, r.n, rows));
    });
}

pub(super) fn keyed_ran(f: &File<'_>, node: usize) {
    AUDIT.with_borrow_mut(|a| {
        a.enter_file(f);
        a.seen[node].keyed_ran = true;
    });
}

pub(crate) fn begin() {
    AUDIT.with_borrow_mut(|a| *a = Audit::default());
}

pub(crate) fn report() {
    AUDIT.with_borrow(|a| {
        for (node, j) in &a.joins {
            if let Some(consumer) = j.consumer {
                eprintln!("ASK_JOIN_CONSUMER\t{node}\t{consumer}");
            }
            if let [Some((ef, el, er, _)), Some((pf, pl, pr, out))] = j.cases {
                eprintln!(
                    "ASK_JOIN_PAIR\t{node}\t{ef}\t{el}\t{er}\t{pf}\t{pl}\t{pr}\t{out}\t{}\t{}\t{}",
                    j.counts[0], j.counts[1], j.used_as_empty
                );
            }
        }
        for (role, counts) in ["whole", "keyed", "size"].iter().zip(a.counts) {
            eprintln!(
                "ASK_PROOFS\t{role}\t{}\t{}\t{}\t{}\t{}\t{}",
                counts[0], counts[1], counts[2], counts[3], counts[4], counts[5]
            );
        }
    });
}
