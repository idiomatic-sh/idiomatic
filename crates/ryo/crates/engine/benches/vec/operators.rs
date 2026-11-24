//! Optional operator ledger, adapted from the contribution observer.
//! Nesting removes child CPU from the enclosing operator's clock.
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::{Mutex, OnceLock},
};

static OUTPUT: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();

pub fn configure(path: &Path) -> std::io::Result<()> {
    let output = BufWriter::new(File::create(path)?);
    OUTPUT
        .set(Mutex::new(output))
        .map_err(|_| std::io::Error::other("operator ledger already configured"))
}

#[derive(Default)]
struct State {
    context: Option<(usize, &'static str)>,
    queries: BTreeMap<usize, String>,
    stack: Vec<Frame>,
    metrics: HashMap<(usize, &'static str, String), [u64; 4]>,
}

struct Frame {
    node: usize,
    input: u64,
    children: u64,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

pub(crate) fn query(id: usize, text: &str) {
    if OUTPUT.get().is_some() {
        STATE.with_borrow_mut(|state| {
            state.queries.entry(id).or_insert_with(|| text.to_owned());
        });
    }
}

pub(crate) struct Scope(Option<(usize, &'static str)>);

pub(crate) fn scope(id: usize, phase: &'static str) -> Scope {
    if OUTPUT.get().is_some() {
        Scope(STATE.with_borrow_mut(|state| state.context.replace((id, phase))))
    } else {
        Scope(None)
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if OUTPUT.get().is_some() {
            STATE.with_borrow_mut(|state| state.context = self.0);
        }
    }
}

fn clock() -> u64 {
    (crate::thread_cpu() * 1e9) as u64
}

#[inline]
pub(crate) fn enter(node: usize, input: usize) -> Option<u64> {
    if OUTPUT.get().is_none() {
        return None;
    }
    STATE.with_borrow_mut(|state| {
        if state.context.is_none() || state.stack.last().is_some_and(|frame| frame.node == node) {
            return None;
        }
        state.stack.push(Frame {
            node,
            input: input as u64,
            children: 0,
        });
        Some(clock())
    })
}

pub(crate) fn rows_in(rows: usize) {
    if OUTPUT.get().is_some() {
        STATE.with_borrow_mut(|state| {
            if let Some(frame) = state.stack.last_mut() {
                frame.input += rows as u64;
            }
        });
    }
}

pub(crate) fn leave(start: u64, kind: &str, rows: usize, ran: bool) {
    let elapsed = clock() - start;
    STATE.with_borrow_mut(|state| {
        let frame = state.stack.pop().expect("operator frame");
        let (query, phase) = state.context.expect("operator query");
        // Reuse the key's owned kind between calls. Cached reads stay in the CPU
        // column but contribute no new runs or output.
        let key = (query, phase, kind.to_owned());
        let metrics = state.metrics.entry(key).or_default();
        metrics[0] += u64::from(ran);
        metrics[1] += frame.input;
        metrics[2] += rows as u64 * u64::from(ran);
        metrics[3] += elapsed.saturating_sub(frame.children);
        if let Some(parent) = state.stack.last_mut() {
            parent.input += rows as u64;
            parent.children += clock() - start;
        }
    });
}

pub(crate) fn flush() {
    if let Some(output) = OUTPUT.get() {
        STATE.with_borrow_mut(|state| {
            let mut out = output.lock().expect("operator output");
            for (id, text) in std::mem::take(&mut state.queries) {
                write!(out, "query\t{id}\t").expect("write query text");
                for byte in text.as_bytes() {
                    write!(out, "{byte:02x}").expect("write query bytes");
                }
                writeln!(out).expect("finish query");
            }
            for ((query, phase, kind), [runs, input, output, ns]) in state.metrics.drain() {
                writeln!(
                    out,
                    "operator\t{query}\t{phase}\t{kind}\t{runs}\t{input}\t{output}\t{ns}"
                )
                .expect("write operator");
            }
            out.flush().expect("flush operator ledger");
        });
    }
}
