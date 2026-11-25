//! Diagnostic (`VEC_MAKE_COUNT=1`): every make of a step, by the step's structural identity (operator, arguments and its
//! inputs' identities, so the same question asked by two programs is the same step) and the file; own thread CPU (nested
//! makes removed); repeats within one program and across programs. Printed at the end of a session.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Default)]
struct Entry {
    makes: u32,
    programs: Vec<usize>,
}

#[derive(Default)]
struct Step {
    op: String,
    first: f64,
    within: f64,
    across: f64,
    makes: u64,
    programs: HashSet<usize>,
}

#[derive(Default)]
struct State {
    program: usize,
    ids: HashMap<(usize, usize), u64>,
    stack: Vec<f64>,
    seen: HashMap<(u64, u32), Entry>,
    steps: HashMap<u64, Step>,
    total: u64,
    own: [f64; 3],
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[inline]
pub(crate) fn on() -> bool {
    sw!("VEC_MAKE_COUNT")
}

pub(crate) fn program(id: usize) {
    STATE.with_borrow_mut(|s| s.program = id);
}

/// The step's identity: operator, arguments, the identities of its inputs.
pub(super) fn identity(plan: &super::Plan, node: usize) -> u64 {
    let key = (std::ptr::from_ref(plan) as usize, node);
    if let Some(id) = STATE.with_borrow(|s| s.ids.get(&key).copied()) {
        return id;
    }
    let step = &plan.nodes[node];
    let mut inputs = Vec::new();
    for &i in step.inputs.iter() {
        inputs.push(identity(plan, i));
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    step.op().hash(&mut h);
    step.base().hash(&mut h);
    format!("{:?}", step.args()).hash(&mut h);
    inputs.hash(&mut h);
    let id = h.finish();
    STATE.with_borrow_mut(|s| s.ids.insert(key, id));
    id
}

pub(super) fn enter() {
    STATE.with_borrow_mut(|s| s.stack.push(0.0));
}

pub(super) fn leave(id: u64, op: &str, file: u32, total: f64) {
    STATE.with_borrow_mut(|s| {
        let children = s.stack.pop().unwrap_or(0.0);
        let own = (total - children).max(0.0);
        if let Some(parent) = s.stack.last_mut() {
            *parent += total;
        }
        s.total += 1;
        let program = s.program;
        let entry = s.seen.entry((id, file)).or_default();
        entry.makes += 1;
        let kind = if entry.makes == 1 {
            0
        } else if entry.programs.contains(&program) {
            1
        } else {
            2
        };
        if !entry.programs.contains(&program) {
            entry.programs.push(program);
        }
        s.own[kind] += own;
        let step = s.steps.entry(id).or_default();
        if step.op.is_empty() {
            step.op = op.to_owned();
        }
        step.makes += 1;
        step.programs.insert(program);
        match kind {
            0 => step.first += own,
            1 => step.within += own,
            _ => step.across += own,
        }
    });
}

pub(crate) fn print() {
    if !on() {
        return;
    }
    STATE.with_borrow(|s| {
        let distinct = s.seen.len() as f64;
        println!("MAKES\ttotal {}\tdistinct_step_file {}\tratio {:.3}", s.total, s.seen.len(), s.total as f64 / distinct.max(1.0));
        println!("MAKECPU\tfirst_ms {:.1}\trepeat_within_program_ms {:.1}\trepeat_across_programs_ms {:.1}", s.own[0] * 1e3, s.own[1] * 1e3, s.own[2] * 1e3);
        let mut rows: Vec<_> = s.steps.iter().collect();
        rows.sort_by(|a, b| (b.1.within + b.1.across).total_cmp(&(a.1.within + a.1.across)));
        for (id, st) in rows.into_iter().take(20) {
            println!("MAKESTEP\t{id:x}\t{}\tmakes {}\tprograms {}\tfirst_ms {:.1}\twithin_ms {:.1}\tacross_ms {:.1}", st.op, st.makes, st.programs.len(), st.first * 1e3, st.within * 1e3, st.across * 1e3);
        }
        let mut byop: HashMap<&str, [f64; 3]> = HashMap::new();
        for st in s.steps.values() {
            let e = byop.entry(st.op.as_str()).or_default();
            e[0] += st.first; e[1] += st.within; e[2] += st.across;
        }
        for (op, e) in byop {
            println!("MAKEOP\t{op}\tfirst_ms {:.1}\twithin_ms {:.1}\tacross_ms {:.1}", e[0] * 1e3, e[1] * 1e3, e[2] * 1e3);
        }
    });
}
