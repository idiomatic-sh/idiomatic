//! THROWAWAY (ryo-fmx3.1, redo-union): how the union and antijoin kernels read an input by key, and the buffers they
//! reuse. An input that only the kernel's step reads is asked exactly the keys the step's own per-key memo found missing,
//! so a second memo on the input never answers anything: it is probed straight (its step body, timed as a probe), not
//! filed. Any other input goes through `Exec::probe` and its memo.

use super::{Exec, File, Provider, R, Run, Table};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Key lists, taken and given back around a probe (a probe may recurse into another kernel).
    static KEYS: RefCell<Vec<Vec<u64>>> = const { RefCell::new(Vec::new()) };
    /// Lists of input answers, likewise.
    static GOT: RefCell<Vec<Vec<(Rc<Table>, u32)>>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn take_keys() -> Vec<u64> {
    KEYS.with(|k| k.borrow_mut().pop()).unwrap_or_default()
}

pub(super) fn give_keys(mut v: Vec<u64>) {
    v.clear();
    KEYS.with(|k| k.borrow_mut().push(v));
}

pub(super) fn take_got() -> Vec<(Rc<Table>, u32)> {
    GOT.with(|k| k.borrow_mut().pop()).unwrap_or_default()
}

pub(super) fn give_got(mut v: Vec<(Rc<Table>, u32)>) {
    v.clear();
    GOT.with(|k| k.borrow_mut().push(v));
}

/// Whether `input` is read by no step but those of `tree` (and is not emitted): then the keys the tree's root asks are
/// the only keys it is ever asked.
pub(super) fn sole(ex: &Exec<'_>, input: usize, tree: &[usize]) -> bool {
    !ex.emitted.contains(&input) && !ex.consumers[input].is_empty() && ex.consumers[input].iter().all(|c| tree.contains(c))
}

/// The rows of `node` whose column `col` holds one of `keys` (distinct, none asked of this node before in this file
/// where `sole`).
#[expect(clippy::too_many_arguments, reason = "a throwaway")]
pub(super) fn input_probe(ex: &mut Exec<'_>, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], sole: bool) -> R<Rc<Table>> {
    // 7 redo-union: every probe a kernel makes of an input is timed as the input's own: the per-key probe leaves a view
    // read in place and a memo answer untimed, so that work fell to the asking step. `VEC_KERNEL_INPUT_UNTIMED` leaves
    // it to the asker as before.
    let direct = sole && !keys.is_empty() && !sw!("VEC_KERNEL_MEMO") && !sw!("VEC_NO_PMEMO") && ex.overridden(node).is_none() && f.memo[node].is_none() && ex.view(run, node)?.is_none();
    if !direct {
        if keys.is_empty() || sw!("VEC_KERNEL_INPUT_UNTIMED") {
            return ex.probe(f, run, p, node, col, keys);
        }
        let t0 = super::step_clock();
        let saved = ex.inner;
        ex.inner = 0.0;
        let out = ex.probe(f, run, p, node, col, keys);
        let dt = super::step_clock() - t0;
        ex.stats.time[node] += dt - ex.inner;
        ex.inner = saved + dt;
        return out;
    }
    let sc = ex.schema(run, node)?;
    let t0 = super::step_clock();
    let saved = ex.inner;
    ex.inner = 0.0;
    ex.stack.push(node);
    let out = ex.probe_body(f, run, p, node, col, keys, &sc);
    ex.stack.pop();
    let dt = super::step_clock() - t0;
    ex.stats.time[node] += dt - ex.inner;
    ex.inner = saved + dt;
    let out = out?;
    ex.stats.probes[node] += 1;
    ex.stats.rows[node] += out.n as u64;
    if sw!("VEC_KERNEL_CHECK") {
        let want: rustc_hash::FxHashSet<u64> = keys.iter().copied().collect();
        let stray = out.cols[col].iter().filter(|k| !want.contains(k)).count();
        assert!(stray == 0, "node {node} col {col}: {stray} rows of keys not asked");
    }
    Ok(out)
}
