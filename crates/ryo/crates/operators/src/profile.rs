//! Optional operator ledger, adapted from the contribution observer.
//! Nesting removes child CPU from the enclosing operator's clock.
use cpu_time::ThreadTime;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::{Mutex, OnceLock},
};

static OUTPUT: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();

/// Open the diagnostic ledger. Call this before submitting profiled work.
///
/// # Errors
/// The file cannot be created or a ledger is already configured.
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
    metrics: HashMap<(usize, &'static str, String), [u64; 12]>,
}

struct Frame {
    node: usize,
    input: u64,
    children: u64,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

/// Record the complete text that owns this program ID.
pub fn query(id: usize, text: &str) {
    if OUTPUT.get().is_some() {
        STATE.with_borrow_mut(|state| {
            state.queries.entry(id).or_insert_with(|| text.to_owned());
        });
    }
}

/// Restores the previous program attribution at the end of a scope.
pub struct Scope(Option<(usize, &'static str)>);

/// Attribute work on the current worker to a program and phase.
#[must_use]
pub fn scope(id: usize, phase: &'static str) -> Scope {
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

fn elapsed(start: ThreadTime) -> u64 {
    start
        .try_elapsed()
        .ok()
        .and_then(|time| u64::try_from(time.as_nanos()).ok())
        .unwrap_or_default()
}

/// Open a timed operator for a configured program, ignoring a nested use of the same node.
#[inline]
#[must_use]
pub fn enter(node: usize, input: usize) -> Option<ThreadTime> {
    if OUTPUT.get().is_none() {
        return None;
    }
    STATE.with_borrow_mut(|state| {
        if state.context.is_none() || state.stack.last().is_some_and(|frame| frame.node == node) {
            return None;
        }
        let start = ThreadTime::try_now().ok()?;
        state.stack.push(Frame {
            node,
            input: input as u64,
            children: 0,
        });
        Some(start)
    })
}

/// Add input rows to the current operator.
pub fn rows_in(rows: usize) {
    if OUTPUT.get().is_some() {
        STATE.with_borrow_mut(|state| {
            if let Some(frame) = state.stack.last_mut() {
                frame.input += rows as u64;
            }
        });
    }
}

/// Close the current timing and add its self CPU and row counts to the ledger.
pub fn leave(start: ThreadTime, kind: &str, rows: usize, ran: bool) {
    let took = elapsed(start);
    STATE.with_borrow_mut(|state| {
        let (Some(frame), Some((query, phase))) = (state.stack.pop(), state.context) else {
            return;
        };
        // Cached reads still contribute CPU but did no new runs or output.
        let key = (query, phase, kind.to_owned());
        let metrics = state.metrics.entry(key).or_default();
        metrics[0] += u64::from(ran);
        metrics[1] += frame.input;
        metrics[2] += rows as u64 * u64::from(ran);
        let own = took.saturating_sub(frame.children);
        metrics[3] += own;
        // Runs by rows in: none, 1 to 8, 9 to 64, more (a run and its CPU in each).
        let bucket = match frame.input { 0 => 0, 1..=8 => 1, 9..=64 => 2, _ => 3 };
        metrics[4 + bucket] += u64::from(ran);
        metrics[8 + bucket] += own;
        if let Some(parent) = state.stack.last_mut() {
            parent.input += rows as u64;
            parent.children += elapsed(start);
        }
    });
}

/// Write this worker's accumulated diagnostic rows to the configured ledger.
///
/// # Errors
/// The ledger cannot be locked, written or flushed.
pub fn flush() -> std::io::Result<()> {
    let Some(output) = OUTPUT.get() else {
        return Ok(());
    };
    STATE.with_borrow_mut(|state| {
        let mut out = output
            .lock()
            .map_err(|_| std::io::Error::other("operator ledger lock"))?;
        for (id, text) in std::mem::take(&mut state.queries) {
            write!(out, "query\t{id}\t")?;
            for byte in text.as_bytes() {
                write!(out, "{byte:02x}")?;
            }
            writeln!(out)?;
        }
        for ((query, phase, kind), [runs, input, output, ns, r0, r1, r2, r3, n0, n1, n2, n3]) in state.metrics.drain() {
            writeln!(
                out,
                "operator\t{query}\t{phase}\t{kind}\t{runs}\t{input}\t{output}\t{ns}\t{r0}\t{r1}\t{r2}\t{r3}\t{n0}\t{n1}\t{n2}\t{n3}"
            )?;
        }
        out.flush()
    })
}
