//! Demand and CPU of ordered base-view row lists. View projections do not change row IDs.
use super::{E, Exec, File, Native, Op, Run, ViewSpec};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::{Value, json};
use std::rc::Rc;

#[derive(Default)]
pub(super) struct Plan {
    ids: Vec<usize>,
    probes: std::cell::RefCell<Vec<[u64; 3]>>,
    forms: Vec<&'static str>,
    keys: Vec<[Metric; 2]>,
    handoffs: Vec<Option<Handoff>>,
    by_handoff: std::collections::BTreeMap<&'static str, (u64, u64)>,
    metrics: FxHashMap<(&'static str, bool, bool), Metric>,
    fan: std::collections::BTreeMap<(usize, usize), u64>,
    slots: [u64; 6],
    phases: std::collections::BTreeMap<&'static str, (u64, f64, f64)>,
}

#[derive(Default)]
pub(super) struct PerFile {
    seen: FxHashMap<usize, Seen>,
    consumer: Option<usize>,
    counts: [Metric; 2],
}

#[derive(Default)]
struct Seen {
    consumers: FxHashSet<usize>,
    builders: usize,
    first: Option<Rc<Vec<u32>>>,
}

#[derive(Default, Clone, Copy)]
pub(super) struct Metric {
    pub(super) builds: u64,
    pub(super) entered: u64,
    pub(super) tests: u64,
    pub(super) kept: u64,
    pub(super) seconds: f64,
}
impl Metric {
    pub(super) fn add(&mut self, x: Self) {
        self.builds += x.builds;
        self.entered += x.entered;
        self.tests += x.tests;
        self.kept += x.kept;
        self.seconds += x.seconds;
    }
    pub(super) fn tsv(self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{:.9}",
            self.builds, self.entered, self.tests, self.kept, self.seconds
        )
    }
}

#[inline]
pub(super) fn auditing() -> bool {
    static ACTIVE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ACTIVE.get_or_init(|| {
        [
            "VEC_COLD_COUNT",
            "VEC_COLD_CLOCK",
            "VEC_COLD_PHASE",
            "VEC_COLD_ENTRIES",
        ]
        .iter()
        .any(|s| std::env::var_os(s).is_some())
    })
}

pub(super) fn clock() -> f64 {
    if sw!("VEC_COLD_CLOCK") || sw!("VEC_COLD_PHASE") {
        thread_local! {static START: cpu_time::ThreadTime = cpu_time::ThreadTime::now();}
        START.with(|t| t.elapsed().as_secs_f64())
    } else {
        0.
    }
}

// Stable typed atoms, remapped to base columns by Exec::view. Flatten/sort associative
// conjunctions and alternatives, intern literal membership as a set, retain seed and regex identity.
/// The entire expression is a positive literal membership on this column. The entry
/// planner may extract only one side of an AND, which is insufficient for identity.
pub(super) fn members(e: &E, col: usize) -> Option<Vec<u32>> {
    match e {
        E::Or(a, b) => {
            let mut values = members(a, col)?;
            values.extend(members(b, col)?);
            Some(values)
        }
        E::Cmp(Op::Eq, ..) | E::In(_, _, false) => super::kinds_required(e, 0, col),
        _ => None,
    }
}

fn normalized(e: &E) -> Value {
    if let E::Cmp(Op::Eq, a, b) = e {
        if let (E::Col(0, c, super::Ty::Sym), E::Text(t))
        | (E::Text(t), E::Col(0, c, super::Ty::Sym)) = (&**a, &**b)
        {
            return json!(["member", c, [t]]);
        }
    }
    if matches!(e, E::Or(..) | E::In(..)) {
        fn column(e: &E) -> Option<usize> {
            match e {
                E::Col(0, c, super::Ty::Sym) => Some(*c),
                E::Cmp(_, a, b) | E::Or(a, b) | E::In(a, b, _) => column(a).or_else(|| column(b)),
                _ => None,
            }
        }
        if let Some(c) = column(e)
            && let Some(mut vs) = members(e, c)
        {
            vs.sort_unstable();
            vs.dedup();
            return json!(["member", c, vs]);
        }
    }
    fn terms(e: &E, and: bool, out: &mut Vec<String>) {
        match e {
            E::And(a, b) if and => {
                terms(a, and, out);
                terms(b, and, out);
            }
            E::Or(a, b) if !and => {
                terms(a, and, out);
                terms(b, and, out);
            }
            _ => out.push(normalized(e).to_string()),
        }
    }
    match e {
        E::And(..) | E::Or(..) => {
            let and = matches!(e, E::And(..));
            let mut xs = Vec::new();
            terms(e, and, &mut xs);
            xs.sort_unstable();
            xs.dedup();
            json!([if and { "and" } else { "or" }, xs])
        }
        E::Cmp(op, a, b) => {
            let (mut a, mut b) = (normalized(a).to_string(), normalized(b).to_string());
            if matches!(op, Op::Eq | Op::Ne) && a > b {
                std::mem::swap(&mut a, &mut b);
            }
            json!(["cmp", op.as_str(), a, b])
        }
        E::Not(a) => json!(["not", normalized(a)]),
        E::Null(a, neg) => json!(["null", neg, normalized(a)]),
        E::In(a, b, neg) => {
            let mut items = if let E::List(xs) = &**b {
                xs.iter()
                    .map(|x| normalized(x).to_string())
                    .collect::<Vec<_>>()
            } else {
                return json!(["in-expr", neg, normalized(a), normalized(b)]);
            };
            items.sort_unstable();
            items.dedup();
            json!(["in", neg, normalized(a), items])
        }
        E::Call(name, xs) => json!(["call", name, xs.iter().map(normalized).collect::<Vec<_>>()]),
        E::TextTest(a, re) => json!(["text-test", normalized(a), format!("{re:?}")]),
        E::Seed(a, k) => json!(["seed", normalized(a), k]),
        E::List(xs) => json!(["list", xs.iter().map(normalized).collect::<Vec<_>>()]),
        _ => json!(format!("{e:?}")),
    }
}

pub(super) fn conjuncts(es: &[E]) -> Vec<String> {
    fn append(e: &E, out: &mut Vec<String>) {
        if let E::And(a, b) = e {
            append(a, out);
            append(b, out);
        } else {
            out.push(normalized(e).to_string());
        }
    }
    let mut xs = Vec::new();
    for e in es {
        append(e, &mut xs);
    }
    xs.sort_unstable();
    xs.dedup();
    xs
}
fn uses(e: &E, col: usize) -> bool {
    match e {
        E::Col(0, c, _) => *c == col,
        E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
            uses(a, col) || uses(b, col)
        }
        E::Not(a) | E::Null(a, _) | E::Seed(a, _) | E::TextTest(a, _) => uses(a, col),
        E::Call(_, xs) | E::List(xs) => xs.iter().any(|e| uses(e, col)),
        _ => false,
    }
}
pub(super) fn form(v: &ViewSpec) -> &'static str {
    if v.filter.is_empty() {
        return "unfiltered";
    }
    match v.base {
        Native::Child if v.filter.iter().all(|e| members(e, 3).is_some()) => "child-field",
        Native::Child if v.filter.iter().any(|e| uses(e, 3)) => "mixed-field",
        Native::Tree if v.filter.iter().any(|e| uses(e, 2)) => "text",
        Native::Tree if v.filter.iter().any(|e| uses(e, 1)) => "kind",
        Native::Fact(_) => "fact-column",
        _ => "other",
    }
}

impl Exec<'_> {
    pub(crate) fn cold_load(&mut self, run: &mut Run) {
        if !auditing() {
            return;
        }
        let mut plan = Plan::default();
        if sw!("VEC_COLD_COUNT") {
            plan.probes
                .get_mut()
                .resize(self.plan.nodes.len() + 1, [0; 3]);
        }
        let mut keys = FxHashMap::default();
        for node in 0..self.plan.nodes.len() {
            let Ok(Some(v)) = self.view(run, node) else {
                plan.ids.push(usize::MAX);
                continue;
            };
            let key = super::cold_entries::view_key(run, &v);
            let id = *keys.entry(key).or_insert_with(|| {
                let id = plan.forms.len();
                plan.forms.push(form(&v));
                if sw!("VEC_COLD_HANDOFF") {
                    plan.handoffs.push(Handoff::compile(run, &v));
                }
                if sw!("VEC_COLD_KEYS") {
                    plan.keys.push([Metric::default(); 2]);
                    println!(
                        "COLD_DEF\t{id}\t{node}\t{}\t{:?}\t{}",
                        form(&v),
                        v.base,
                        self.plan.nodes[node].j
                    );
                }
                id
            });
            plan.ids.push(id);
        }
        run.cold = plan;
    }

    pub(super) fn cold_make(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn super::Provider,
        node: usize,
    ) -> super::R<super::Res> {
        let prev = f.cold.consumer.replace(node);
        f.vrows.owner(Some(node));
        let result = self.make_(f, run, p, node);
        f.cold.consumer = prev;
        f.vrows.owner(prev);
        result
    }
}
impl Plan {
    pub(super) fn probe(&self, f: &File<'_>, bulk_keys: Option<usize>) {
        let owner = f.cold.consumer.unwrap_or(self.ids.len());
        let mut all = self.probes.borrow_mut();
        let n = &mut all[owner];
        if let Some(keys) = bulk_keys {
            n[1] += 1;
            n[2] += keys as u64;
        } else {
            n[0] += 1;
        }
    }

    pub(super) fn ask(&self, f: &mut File<'_>, node: usize) {
        let seen = f.cold.seen.entry(self.ids[node]).or_default();
        let _ = seen.consumers.insert(f.cold.consumer.unwrap_or(node));
    }
    pub(super) fn built(
        &mut self,
        f: &mut File<'_>,
        node: usize,
        leaf: bool,
        entered: usize,
        tests: u64,
        rows: &super::cold_bucket::Rows<'_>,
        seconds: f64,
    ) {
        f.vrows.primary(node, entered, tests);
        let key = self.ids[node];
        let seen = f.cold.seen.get_mut(&key).expect("list requested");
        let dup = seen.builders != 0;
        if sw!("VEC_COLD_COUNT") {
            if let Some(first) = &seen.first {
                assert_eq!(
                    first.as_slice(),
                    rows.shadow(),
                    "file {}, list {key}, node {node}",
                    f.version.file
                );
            } else {
                seen.first = Some(Rc::new(rows.shadow()));
            }
        }
        seen.builders += 1;
        if sw!("VEC_COLD_HANDOFF")
            && let Some(h) = &self.handoffs[key]
        {
            let how = h.check(f, &rows.shadow());
            let n = self.by_handoff.entry(how).or_default();
            n.0 += 1;
            n.1 += rows.len() as u64;
        }
        let m = Metric {
            builds: 1,
            entered: entered as u64,
            tests,
            kept: rows.len() as u64,
            seconds,
        };
        self.metrics
            .entry((self.forms[key], leaf, dup))
            .or_default()
            .add(m);
        f.cold.counts[usize::from(dup)].add(m);
        if sw!("VEC_COLD_KEYS") {
            self.keys[key][usize::from(dup)].add(m);
        }
    }
    pub(crate) fn end_file(&mut self, f: &mut File<'_>) {
        if !auditing() {
            return;
        }
        let mut consumers = 0;
        for seen in f.cold.seen.values() {
            *self
                .fan
                .entry((seen.consumers.len(), seen.builders))
                .or_default() += 1;
            consumers += seen.consumers.len();
        }
        if sw!("VEC_COLD_FILES") {
            println!(
                "COLD_FILE\t{}\t{}\t{}\t{}\t{}",
                f.file,
                f.cold.seen.len(),
                consumers,
                f.cold.counts[0].tsv(),
                f.cold.counts[1].tsv()
            );
        }
        for key in f.vrows.keys() {
            let namespace = (*key >> 56).min(5);
            self.slots[namespace] += 1;
        }
        f.cold.seen.clear();
    }
    pub(super) fn phase(&mut self, node: usize, entry: f64, filter: f64) {
        let t = self.phases.entry(self.forms[self.ids[node]]).or_default();
        t.0 += 1;
        t.1 += entry;
        t.2 += filter;
    }
    pub(crate) fn report(&self) {
        if !auditing() {
            return;
        }
        for (node, counts) in self.probes.borrow().iter().enumerate() {
            if *counts != [0; 3] {
                println!(
                    "COLD_PROBE\t{node}\t{}\t{}\t{}",
                    counts[0], counts[1], counts[2]
                );
            }
        }
        for (how, (calls, rows)) in &self.by_handoff {
            println!("COLD_HANDOFF\t{how}\t{calls}\t{rows}");
        }
        for (id, buckets) in self.keys.iter().enumerate() {
            if buckets[0].builds + buckets[1].builds > 0 {
                println!("COLD_KEY\t{id}\t{}\t{}", buckets[0].tsv(), buckets[1].tsv());
            }
        }
        for (form, (calls, entry, filter)) in &self.phases {
            println!("COLD_PHASE\t{form}\t{calls}\t{entry:.9}\t{filter:.9}");
        }
        let mut out = self.metrics.iter().collect::<Vec<_>>();
        out.sort_by_key(|x| *x.0);
        for ((form, leaf, dup), m) in out {
            println!("COLD_LIST\t{form}\t{leaf}\t{dup}\t{}", m.tsv());
        }
        for ((n, b), count) in &self.fan {
            println!("COLD_FAN\t{n}\t{b}\t{count}");
        }
        println!(
            "COLD_PLAN\t{}\t{}\t{:?}",
            self.ids.iter().filter(|i| **i != usize::MAX).count(),
            self.forms.len(),
            self.slots
        );
        if sw!("VEC_COLD_CLOCK") {
            let mut cost = [0.; 1001];
            for x in &mut cost {
                let t = clock();
                *x = clock() - t;
            }
            cost.sort_unstable_by(f64::total_cmp);
            println!("COLD_CLOCK_NS\t{:.3}", 1e9 * cost[cost.len() / 2]);
        }
    }
}

impl Exec<'_> {
    pub(crate) fn cold_end(&mut self, f: &mut File<'_>, run: &mut Run) {
        run.cold.end_file(f);
        if sw!("VEC_COLD_ENTRIES") {
            let mut audit = std::mem::take(&mut run.cold_entries);
            audit.finish(self, run, f);
            run.cold_entries = audit;
        }
    }
}

impl Exec<'_> {
    pub(crate) fn cold_report(&self, run: &Run) {
        super::cold_test::report();
        super::cold_bucket::report();
        super::cold_text::report();
        super::cold_pred::report();
        run.cold.report();
        run.cold_entries.report();
    }
}

/// Whether the file's existing, ordered provider buckets already contain the result.
pub(super) struct Handoff {
    fields: bool,
    codes: Vec<u16>,
}
impl Handoff {
    pub(super) fn compile(run: &Run, v: &ViewSpec) -> Option<Self> {
        let (col, fields, vocab) = match v.base {
            Native::Tree => (1, false, &run.kind_sym),
            Native::Child if !run.file_field_codes => (3, true, &run.field_sym),
            _ => return None,
        };
        if v.filter.is_empty() {
            return None;
        }
        let requirements = v
            .filter
            .iter()
            .map(|e| members(e, col))
            .collect::<Option<Vec<_>>>()?;
        if fields && requirements.iter().all(|xs| xs.contains(&run.empty)) {
            return None;
        }
        let codes = vocab
            .iter()
            .enumerate()
            .filter(|(_, s)| requirements.iter().all(|xs| xs.contains(s)))
            .filter_map(|(i, _)| u16::try_from(i).ok())
            .collect();
        Some(Self { fields, codes })
    }
    pub(super) fn codes(&self) -> &[u16] {
        &self.codes
    }
    pub(super) fn select(&self, f: &File<'_>) -> super::cold_bucket::Slot {
        super::cold_bucket::select(f, self.fields, &self.codes)
    }

    fn check(&self, f: &File<'_>, rows: &[u32]) -> &'static str {
        use ryo::languages::{FieldCode, KindBits, node_table};
        let mut live = self
            .codes
            .iter()
            .map(|c| {
                if self.fields {
                    node_table::ids_of_field(f.table, FieldCode(*c))
                } else {
                    node_table::ids_of_kind(f.table, KindBits(*c))
                }
            })
            .filter(|s| !s.is_empty());
        let Some(first) = live.next() else {
            assert!(rows.is_empty());
            return "empty";
        };
        let Some(second) = live.next() else {
            if first.iter().map(|p| p.0).eq(rows.iter().copied()) {
                return "borrow one slice";
            }
            assert!(
                self.fields
                    && first
                        .iter()
                        .map(|p| p.0)
                        .filter(|p| f.has_parent(*p))
                        .eq(rows.iter().copied())
            );
            return "one slice with parent check";
        };
        let mut ids = first.iter().chain(second).map(|p| p.0).collect::<Vec<_>>();
        for other in live {
            ids.extend(other.iter().map(|p| p.0));
        }
        ids.sort_unstable();
        if self.fields {
            ids.retain(|p| f.has_parent(*p));
        }
        assert_eq!(ids, rows, "merged provider buckets");
        "merge ordered slices"
    }
}
