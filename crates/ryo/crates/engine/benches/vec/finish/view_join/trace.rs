//! Measurement hooks for a detached diagnostic build. Nested operator bodies have their
//! own clocks. Each interval subtracts the measured spacing of adjacent thread-clock reads.
use super::{E, Exec, Op, Run};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::Location;

const N: usize = 32;
#[derive(Clone, Default)]
struct Counts {
    calls: u64,
    rows: u64,
    time: [i64; N],
}
struct Call {
    node: usize,
    mode: u8, // 0 before body, 1 whole, 2 keyed, 3 materialisation, 4 gate
    phase: usize,
    time: [i64; N],
}
#[derive(Default)]
struct Readers {
    sites: BTreeSet<String>,
    rows: usize,
}
#[derive(Default)]
struct Trace {
    last: i64,
    overhead: i64,
    stack: Vec<Call>,
    totals: BTreeMap<(usize, u8, bool), Counts>,
    file: Option<u32>,
    readers: BTreeMap<usize, Readers>,
    uses: BTreeMap<(String, usize, usize), u64>,
    materialized: BTreeMap<Vec<String>, (u64, u64)>,
    shapes: Vec<String>,
    probes: rustc_hash::FxHashMap<(usize, super::Native, usize, bool), (u64, u64)>,
    events: BTreeMap<(usize, &'static str), (u64, u64)>,
}
thread_local! { static TRACE: RefCell<Trace> = RefCell::new(Trace::default()); }
fn now() -> i64 {
    (super::super::thread_cpu() * 1e9).round() as i64
}
impl Trace {
    fn stop(&mut self) {
        let t = now();
        if let Some(frame) = self.stack.last_mut() {
            frame.time[frame.phase] += t - self.last - self.overhead;
        }
    }
    fn start(&mut self) {
        self.last = now();
    }
    fn flush(&mut self) {
        for (_, r) in std::mem::take(&mut self.readers) {
            let v = self
                .materialized
                .entry(r.sites.into_iter().collect())
                .or_default();
            v.0 += 1;
            v.1 += r.rows as u64;
        }
    }
}
pub(crate) fn load(exec: &mut Exec<'_>, run: &mut Run) {
    let mut shapes = Vec::new();
    for i in 0..exec.plan.nodes.len() {
        let pn = &exec.plan.nodes[i];
        let shape =
            if pn.op != "select" {
                String::new()
            } else if exec.view(run, i).ok().flatten().is_some() {
                "view materialisation".to_owned()
            } else if let Ok(sp) = exec.sel(run, i) {
                let k = sp.srcs.len();
                let n = sp
                    .srcs
                    .iter()
                    .filter(|j| exec.view(run, **j).ok().flatten().is_some())
                    .count();
                if n == 0 {
                    String::new()
                } else {
                    let rep = if n == k { "views" } else { "mixed" };
                    let keys = sp.conj.iter().filter(|term| {
                    let E::Cmp(Op::Eq, left, right) = term else { return false };
                    matches!((&**left, &**right), (E::Col(x, _, _), E::Col(y, _, _)) if x != y)
                }).count();
                    let tests = if k == 1 {
                        if sp.conj.is_empty() {
                            "project"
                        } else {
                            "filter + project"
                        }
                    } else if keys == 0 {
                        "no column equikey"
                    } else if keys == sp.conj.len() {
                        "equi only"
                    } else {
                        "equi + tests"
                    };
                    format!("{rep} / {k}-way {tests}")
                }
            } else {
                String::new()
            };
        shapes.push(shape);
    }
    let mut costs = [0; 2001];
    for c in &mut costs {
        let a = now();
        *c = now() - a;
    }
    costs.sort_unstable();
    TRACE.with_borrow_mut(|t| {
        t.shapes = shapes;
        t.overhead = costs[costs.len() / 2];
    });
}
pub(super) fn enter(node: usize, mode: u8, phase: usize) {
    if sw!("VEC_J4_ORDER") {
        return;
    }
    TRACE.with_borrow_mut(|t| {
        t.stop();
        t.stack.push(Call {
            node,
            mode,
            phase,
            time: [0; N],
        });
        t.start();
    });
}
pub(super) fn mode(mode: u8) {
    if sw!("VEC_J4_ORDER") {
        return;
    }
    TRACE.with_borrow_mut(|t| {
        if let Some(c) = t.stack.last_mut() {
            c.mode = mode;
        }
    });
}
pub(super) fn current() -> usize {
    TRACE.with_borrow(|t| t.stack.last().map_or(0, |frame| frame.phase))
}
pub(super) fn phase(i: usize) -> usize {
    if sw!("VEC_J4_ORDER") {
        return 0;
    }
    TRACE.with_borrow_mut(|t| {
        t.stop();
        let old = t
            .stack
            .last_mut()
            .map_or(0, |frame| std::mem::replace(&mut frame.phase, i));
        t.start();
        old
    })
}
pub(super) fn leave(rows: usize) {
    if sw!("VEC_J4_ORDER") {
        return;
    }
    TRACE.with_borrow_mut(|t| {
        t.stop();
        let c = t.stack.pop().expect("balanced view join diagnostic");
        if !t.shapes[c.node].is_empty() {
            let v = t.totals.entry((c.node, c.mode, rows != 0)).or_default();
            v.calls += 1;
            v.rows += rows as u64;
            for (dest, x) in v.time.iter_mut().zip(c.time) {
                *dest += x;
            }
        }
        if let Some(parent) = t.stack.last_mut() {
            // Match the two-table profiler: keyed work is charged to the requesting
            // step. A whole child charges only its executor frame to its requester.
            match c.mode {
                2 | 5 | 7 => {
                    // Keyed work and the resolution of an eval handle belong to this
                    // caller. Preserve nested view work, provider fills and child frames.
                    let mut rest = c.time.iter().sum::<i64>();
                    for phase in [16, 28, 29] {
                        parent.time[phase] += c.time[phase];
                        rest -= c.time[phase];
                    }
                    let inside_view: i64 = [3, 19, 20, 21, 22, 23, 24, 25, 26, 27, 30, 31]
                        .iter()
                        .map(|i| c.time[*i])
                        .sum();
                    parent.time[29] += inside_view;
                    let target = match c.mode {
                        2 => 11,
                        5 if parent.phase != 20 => 19,
                        _ => parent.phase,
                    };
                    parent.time[target] += rest - inside_view;
                }
                3 => {}
                1 => {
                    parent.time[16] += [0, 1, 14, 15, 17, 18]
                        .iter()
                        .map(|p| c.time[*p])
                        .sum::<i64>()
                }
                _ => parent.time[16] += c.time.iter().sum::<i64>(),
            }
        }
        t.start();
    });
}
pub(super) fn probes(node: usize, base: super::Native, col: usize, direct: bool, keys: usize) {
    if sw!("VEC_J4_ORDER") {
        return;
    }
    TRACE.with_borrow_mut(|t| {
        t.stop();
        let v = t.probes.entry((node, base, col, direct)).or_default();
        v.0 += 1;
        v.1 += keys as u64;
        t.start()
    });
}
thread_local! {static WRITER: RefCell<Option<std::io::BufWriter<std::fs::File>>> = const { RefCell::new(None) };}
/// The separate gate streams the complete result, including empty rows, into SHA-256.
pub(super) fn ordered(file: u32, node: usize, mode: u8, table: &super::Table) {
    use std::io::Write;
    if !sw!("VEC_J4_ORDER") {
        return;
    }

    WRITER.with_borrow_mut(|slot| {
        let out = slot.get_or_insert_with(|| {
            std::io::BufWriter::new(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(std::env::var_os("VEC_J4_ORDER").expect("join stream"))
                    .expect("join stream writer"),
            )
        });
        for x in [
            file as u64,
            node as u64,
            mode as u64,
            table.n as u64,
            table.cols.len() as u64,
        ] {
            out.write_all(&x.to_le_bytes()).unwrap();
        }
        for col in &table.cols {
            for word in col {
                out.write_all(&word.to_le_bytes()).unwrap();
            }
        }
    });
}
pub(super) fn event(name: &'static str, n: usize) {
    if sw!("VEC_J4_ORDER") {
        return;
    }
    TRACE.with_borrow_mut(|t| {
        t.stop();
        if let Some(frame) = t.stack.last() {
            let v = t.events.entry((frame.node, name)).or_default();
            v.0 += 1;
            v.1 += n as u64;
        }
        t.start();
    });
}
pub(super) fn reader(file: u32, node: usize, rows: usize, site: &Location<'_>) {
    if sw!("VEC_J4_ORDER") {
        return;
    }
    TRACE.with_borrow_mut(|t| {
        t.stop();
        if t.file != Some(file) {
            t.flush();
            t.file = Some(file);
        }
        let name = format!(
            "{}:{}",
            site.file().rsplit("/vec/").next().unwrap_or(site.file()),
            site.line()
        );
        let r = t.readers.entry(node).or_default();
        r.rows = rows;
        r.sites.insert(name.clone());
        let owner = t.stack.last().map_or(usize::MAX, |c| c.node);
        *t.uses.entry((name, owner, node)).or_default() += 1;
        t.start();
    });
}
pub(crate) fn report() {
    if sw!("VEC_J4_ORDER") {
        use std::io::Write;
        WRITER.with_borrow_mut(|writer| {
            if let Some(mut w) = writer.take() {
                w.flush().unwrap();
            }
        });
        return;
    }

    TRACE.with_borrow_mut(|t| {
        t.flush();
        println!("J4CLOCK\t{}", t.overhead);
        for (i, s) in t.shapes.iter().enumerate().filter(|(_, s)| !s.is_empty()) {
            println!("J4SHAPE\t{i}\t{s}");
        }
        for ((i, m, p), v) in &t.totals {
            println!(
                "J4PH\t{i}\t{m}\t{}\t{}\t{}\t{}",
                u8::from(*p),
                v.calls,
                v.rows,
                v.time.map(|x| x.to_string()).join(",")
            );
        }
        for ((i, base, col, direct), (n, w)) in &t.probes {
            println!("J4PROBE\t{i}\t{base:?}\t{col}\t{direct}\t{n}\t{w}");
        }
        for ((i, s), (n, w)) in &t.events {
            println!("J4EV\t{i}\t{s}\t{n}\t{w}");
        }
        for ((s, owner, node), n) in &t.uses {
            println!("J4USE\t{s}\t{owner}\t{node}\t{n}");
        }
        for (sites, (n, w)) in &t.materialized {
            println!("J4MAT\t{}\t{n}\t{w}", sites.join(","));
        }
    });
}
