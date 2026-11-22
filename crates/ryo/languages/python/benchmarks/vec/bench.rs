//! THROWAWAY measurement (bead ryo-fmx3.1): what finding every rule's first-condition candidates costs over flat per-file node
//! tables bucketed by kind. Not wired into the engine; it names node kinds only through the anchors file it is given.
//!
//! `vec census <files> <literals> <out>`: per kind and field element counts, and the kinds that carry each literal text.
//! `vec run <mode> <files> <anchors> [--batch N] [--repeat K]`: modes `floor`, `bucket`, `skip`, `first`, `shared`, `kernels`.

#![expect(clippy::expect_used, clippy::print_stdout, clippy::print_stderr, clippy::indexing_slicing, clippy::cast_possible_truncation, clippy::too_many_lines, clippy::unwrap_used, reason = "a throwaway measurement")]

use ryo::languages::{
    self, ByteSlice, FileFacts, FileId, FileVisit, Fillers as _, KindBits, NodeTable, Position, RelationNeeds, Walk, WalkRequest,
    node_table,
};
use std::collections::HashMap;
use std::hint::black_box;

struct NoVisit;
impl FileVisit for NoVisit {
    fn visit(&mut self, _file: FileFacts<'_>) {}
}

fn thread_cpu() -> f64 {
    thread_local! {
        static ORIGIN: cpu_time::ThreadTime = cpu_time::ThreadTime::now();
    }
    ORIGIN.with(|origin| origin.elapsed().as_secs_f64())
}

#[derive(Default)]
struct Phases(Vec<(&'static str, f64)>);
impl Phases {
    fn add(&mut self, name: &'static str, seconds: f64) {
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| *n == name) {
            slot.1 += seconds;
        } else {
            self.0.push((name, seconds));
        }
    }
}

#[derive(Clone)]
enum Lit {
    None,
    Eq(Vec<Vec<u8>>),
    Re(regex::bytes::Regex),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Key {
    Kind(u16),
    Field(u16),
}

#[derive(Clone)]
struct Anchor {
    keys: Vec<Key>,
    lit: Lit,
    lit_text: String,
}

struct Rule {
    name: String,
    anchors: Vec<Anchor>,
    /// Each clause: kind words, field words; a file survives when every clause meets a present bit.
    clauses: Vec<(Vec<u64>, Vec<u64>)>,
    never: bool,
    whole: Option<String>,
}

struct Vocab {
    kinds: HashMap<String, u16>,
    kind_names: Vec<String>,
    fields: HashMap<String, u16>,
    field_names: Vec<String>,
}

fn vocabulary<W: Walk>(_: &W) -> Vocab {
    let v = W::vocabulary();
    let kind_names: Vec<String> = v.kinds.0.iter().map(|n| n.0.to_owned()).collect();
    let field_names: Vec<String> = v.fields.0.iter().map(|n| n.0.to_owned()).collect();
    Vocab {
        kinds: kind_names.iter().enumerate().map(|(i, n)| (n.clone(), i as u16)).collect(),
        fields: field_names.iter().enumerate().map(|(i, n)| (n.clone(), i as u16)).collect(),
        kind_names,
        field_names,
    }
}

fn words(bits: usize) -> usize {
    bits.div_ceil(64).max(1)
}

fn load_rules(path: &str, vocab: &Vocab) -> Vec<Rule> {
    let text = std::fs::read_to_string(path).expect("anchors file");
    let mut missing = std::collections::BTreeSet::new();
    let (kw, fw) = (words(vocab.kind_names.len()), words(vocab.field_names.len()));
    let mut rules = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (name, class, anchors, clauses) = (cols[0], cols[1], cols[2], cols.get(4).copied().unwrap_or(""));
        let mut rule = Rule { name: name.to_owned(), anchors: Vec::new(), clauses: Vec::new(), never: false, whole: None };
        if class == "rel" {
            rule.whole = Some(anchors.to_owned());
        } else {
            for part in anchors.split('\u{1e}') {
                let (keys, lit) = part.split_once('=').expect("anchor =");
                let (tag, names) = keys.split_at(2);
                let mut list = Vec::new();
                for n in names.split(',').filter(|n| !n.is_empty()) {
                    let code = if tag == "k:" { vocab.kinds.get(n).map(|c| Key::Kind(*c)) } else { vocab.fields.get(n).map(|c| Key::Field(*c)) };
                    match code {
                        Some(c) => list.push(c),
                        None => {
                            missing.insert(format!("{tag}{n}"));
                        }
                    }
                }
                let parsed = if let Some(eq) = lit.strip_prefix("eq:") {
                    let v: Vec<String> = serde_json::from_str(eq).expect("eq json");
                    Lit::Eq(v.into_iter().map(String::into_bytes).collect())
                } else if let Some(re) = lit.strip_prefix("re:") {
                    let v: Vec<String> = serde_json::from_str(re).expect("re json");
                    Lit::Re(regex::bytes::Regex::new(&v.join("|")).expect("regex"))
                } else {
                    Lit::None
                };
                rule.anchors.push(Anchor { keys: list, lit: parsed, lit_text: lit.to_owned() });
            }
        }
        for clause in clauses.split(';').filter(|c| !c.is_empty()) {
            if clause == "-" {
                rule.never = true;
                continue;
            }
            let (mut k, mut f) = (vec![0u64; kw], vec![0u64; fw]);
            for n in clause.split(',') {
                let (tag, name) = n.split_at(2);
                if tag == "k:" {
                    if let Some(c) = vocab.kinds.get(name) {
                        k[usize::from(*c) / 64] |= 1 << (c % 64);
                    } else {
                        missing.insert(n.to_owned());
                    }
                } else if let Some(c) = vocab.fields.get(name) {
                    f[usize::from(*c) / 64] |= 1 << (c % 64);
                } else {
                    missing.insert(n.to_owned());
                }
            }
            rule.clauses.push((k, f));
        }
        rules.push(rule);
    }
    if !missing.is_empty() {
        eprintln!("vec: names not in the vocabulary: {missing:?}");
    }
    rules
}

/// One file's columns the experiment reads beyond the table's own buckets.
#[derive(Default)]
struct Columns {
    kind: Vec<u16>,
    field: Vec<u16>,
    start: Vec<u32>,
    end: Vec<u32>,
    field_offsets: Vec<u32>,
    by_field: Vec<u32>,
    field_present: Vec<u64>,
    kind_offsets: Vec<u32>,
    by_kind: Vec<u32>,
}

fn extract(table: &NodeTable, cols: &mut Columns, with_spans: bool, with_fields: bool) {
    let count = node_table::element_count(table).0;
    cols.kind.clear();
    cols.field.clear();
    cols.start.clear();
    cols.end.clear();
    for id in 0..count as u32 {
        let p = Position(id);
        cols.kind.push(node_table::kind_of(table, p).0);
        if with_fields {
            cols.field.push(node_table::field_of(table, p).0);
        }
        if with_spans {
            let s = node_table::span_of(table, p);
            cols.start.push(s.start.0);
            cols.end.push(s.end.0);
        }
    }
}

/// Counting sort of positions by a small code: offsets has width + 1 entries.
fn counting_sort(codes: &[u16], width: usize, offsets: &mut Vec<u32>, sorted: &mut Vec<u32>) {
    offsets.clear();
    offsets.resize(width + 1, 0);
    for c in codes {
        offsets[usize::from(*c) + 1] += 1;
    }
    for i in 0..width {
        offsets[i + 1] += offsets[i];
    }
    sorted.clear();
    sorted.resize(codes.len(), 0);
    let mut fill: Vec<u32> = offsets[..width].to_vec();
    for (i, c) in codes.iter().enumerate() {
        let slot = &mut fill[usize::from(*c)];
        sorted[*slot as usize] = i as u32;
        *slot += 1;
    }
}

fn field_index(cols: &mut Columns, fields: usize) {
    let Columns { field, field_offsets, by_field, field_present, .. } = cols;
    counting_sort(field, fields, field_offsets, by_field);
    field_present.clear();
    field_present.resize(words(fields), 0);
    for f in 0..fields {
        if field_offsets[f + 1] > field_offsets[f] {
            field_present[f / 64] |= 1 << (f % 64);
        }
    }
}

fn test_lit(lit: &Lit, text: &[u8]) -> bool {
    match lit {
        Lit::None => true,
        Lit::Eq(set) => set.iter().any(|s| s.as_slice() == text),
        Lit::Re(re) => re.is_match(text),
    }
}

fn bucket<'t>(table: &'t NodeTable, cols: &'t Columns, key: Key) -> Bucket<'t> {
    match key {
        Key::Kind(k) => Bucket::Table(node_table::ids_of_kind(table, KindBits(k))),
        Key::Field(f) => {
            let (a, b) = (cols.field_offsets[usize::from(f)] as usize, cols.field_offsets[usize::from(f) + 1] as usize);
            Bucket::Own(&cols.by_field[a..b])
        }
    }
}

enum Bucket<'t> {
    Table(&'t [Position]),
    Own(&'t [u32]),
}

/// Visits one anchor's buckets in a file and pushes its survivors; returns entries visited.
fn first_condition(table: &NodeTable, cols: &Columns, source: &[u8], anchor: &Anchor, out: &mut Vec<u32>) -> u64 {
    let mut visited = 0u64;
    for key in &anchor.keys {
        match (bucket(table, cols, *key), &anchor.lit) {
            (Bucket::Table(ids), Lit::None) => {
                visited += ids.len() as u64;
                out.extend(ids.iter().map(|p| p.0));
            }
            (Bucket::Own(ids), Lit::None) => {
                visited += ids.len() as u64;
                out.extend_from_slice(ids);
            }
            (Bucket::Table(ids), lit) => {
                visited += ids.len() as u64;
                for p in ids {
                    let i = p.0 as usize;
                    let text = &source[cols.start[i] as usize..cols.end[i] as usize];
                    if test_lit(lit, text) {
                        out.push(p.0);
                    }
                }
            }
            (Bucket::Own(ids), lit) => {
                visited += ids.len() as u64;
                for &i in ids {
                    let text = &source[cols.start[i as usize] as usize..cols.end[i as usize] as usize];
                    if test_lit(lit, text) {
                        out.push(i);
                    }
                }
            }
        }
    }
    visited
}

fn survives(rule: &Rule, kinds: &[u64], fields: &[u64]) -> bool {
    if rule.never {
        return false;
    }
    rule.clauses.iter().all(|(k, f)| {
        k.iter().zip(kinds).any(|(a, b)| a & b != 0) || f.iter().zip(fields).any(|(a, b)| a & b != 0)
    })
}

fn files(list: &str) -> Vec<String> {
    std::fs::read_to_string(list).expect("file list").lines().map(str::to_owned).collect()
}

#[derive(Default)]
struct Totals {
    files: u64,
    walked: u64,
    elements: u64,
    rule_files: u64,
    rule_files_alive: u64,
    entries: u64,
    survivors: u64,
    shared_entries: u64,
    shared_survivors: u64,
    shared_fanout: u64,
    distinct_conditions: u64,
}

fn census(walk: &impl Walk, vocab: &Vocab, list: &str, literals: &str, out: &str) {
    let lits: Vec<String> = std::fs::read_to_string(literals)
        .expect("literals")
        .lines()
        .map(|l| serde_json::from_str::<String>(l).expect("literal json"))
        .collect();
    let mut kinds = vec![0u64; vocab.kind_names.len().max(256)];
    let mut fields = vec![0u64; vocab.field_names.len().max(256)];
    let mut texts: HashMap<(usize, u16), u64> = HashMap::new();
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let mut cols = Columns::default();
    let (mut walked, mut elements) = (0u64, 0u64);
    for path in files(list) {
        let bytes = std::fs::read(&path).expect("read");
        let mut builder = node_table::builder(&mut table, walk_kinds(walk));
        let ok = walk
            .walk(
                WalkRequest { path: languages::SourcePath(&path), file: FileId(0), source: ByteSlice(&bytes) },
                RelationNeeds(&[]),
                &mut builder,
                &mut scratch,
                &mut NoVisit,
            )
            .is_ok();
        let t = node_table::finished(&mut builder);
        if !ok {
            continue;
        }
        walked += 1;
        extract(t, &mut cols, true, true);
        elements += cols.kind.len() as u64;
        for i in 0..cols.kind.len() {
            kinds[usize::from(cols.kind[i])] += 1;
            fields[usize::from(cols.field[i])] += 1;
            let text = &bytes[cols.start[i] as usize..cols.end[i] as usize];
            for (l, lit) in lits.iter().enumerate() {
                if lit.as_bytes() == text {
                    *texts.entry((l, cols.kind[i])).or_default() += 1;
                }
            }
        }
    }
    let mut w = String::new();
    for (i, n) in vocab.kind_names.iter().enumerate() {
        w.push_str(&format!("kind\t{n}\t{}\n", kinds[i]));
    }
    for (i, n) in vocab.field_names.iter().enumerate() {
        w.push_str(&format!("field\t{n}\t{}\n", fields[i]));
    }
    let mut t: Vec<_> = texts.into_iter().collect();
    t.sort();
    for ((l, k), c) in t {
        w.push_str(&format!("text\t{}\t{}\t{c}\n", serde_json::to_string(&lits[l]).expect("json"), vocab.kind_names[usize::from(k)]));
    }
    std::fs::write(out, w).expect("write census");
    println!("census: files walked {walked}, elements {elements}");
}

fn walk_kinds<W: Walk>(_: &W) -> languages::Count {
    W::KIND_COUNT
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `cargo bench` passes `--bench`; drop it.
    let args: Vec<&str> = args.iter().map(String::as_str).filter(|a| *a != "--bench").collect();
    let walk = ryo_python::walk();
    let vocab = vocabulary(&walk);
    match args.get(1).copied() {
        Some("census") => census(&walk, &vocab, args[2], args[3], args[4]),
        Some("run") => run(&walk, &vocab, &args[2..]),
        _ => eprintln!("usage: vec census <files> <literals> <out> | vec run <mode> <files> <anchors> [--batch N] [--repeat K] [--rules-out F]"),
    }
}

fn flag(args: &[&str], name: &str) -> Option<String> {
    args.iter().position(|a| *a == name).map(|i| args[i + 1].to_owned())
}

fn run(walk: &impl Walk, vocab: &Vocab, args: &[&str]) {
    let mode = args[0];
    let list = files(args[1]);
    let rules = load_rules(args[2], vocab);
    let batch: usize = flag(args, "--batch").map_or(1, |b| b.parse().expect("batch"));
    let repeat: usize = flag(args, "--repeat").map_or(1, |b| b.parse().expect("repeat"));
    let rules_out = flag(args, "--rules-out");
    let form = flag(args, "--form").unwrap_or_else(|| "bucket".to_owned());
    let level = match mode {
        "floor" => 0,
        "bucket" => 1,
        "skip" => 2,
        "first" => 3,
        "shared" => 4,
        "kernels" => 5,
        other => panic!("mode {other}"),
    };
    let fields_needed = rules.iter().any(|r| r.anchors.iter().any(|a| a.keys.iter().any(|k| matches!(k, Key::Field(_)))) || r.clauses.iter().any(|(_, f)| f.iter().any(|w| *w != 0)));
    let kinds_width = vocab.kind_names.len();
    let fields_width = vocab.field_names.len();
    let mut phases = Phases::default();
    let mut totals = Totals::default();
    let mut per_rule_entries = vec![0u64; rules.len()];
    let mut per_rule_survivors = vec![0u64; rules.len()];
    let mut per_rule_files = vec![0u64; rules.len()];
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let mut cols = Columns::default();
    let mut out: Vec<u32> = Vec::new();
    // shared: distinct (key list, literal) conditions.
    let mut distinct: HashMap<(Vec<Key>, String), usize> = HashMap::new();
    let mut rule_conditions: Vec<Vec<usize>> = Vec::new();
    let mut conditions: Vec<Anchor> = Vec::new();
    for rule in &rules {
        let mut ids = Vec::new();
        for a in &rule.anchors {
            let key = (a.keys.clone(), a.lit_text.clone());
            let n = distinct.len();
            let id = *distinct.entry(key).or_insert(n);
            if id == conditions.len() {
                conditions.push(a.clone());
            }
            ids.push(id);
        }
        rule_conditions.push(ids);
    }
    let mut condition_stamp = vec![u64::MAX; conditions.len()];
    let mut condition_range = vec![(0usize, 0usize); conditions.len()];
    let mut fanout: Vec<(usize, usize, usize)> = Vec::new();
    // batched bucket state
    let mut batch_kinds: Vec<u16> = Vec::new();
    let mut batch_offsets: Vec<u32> = Vec::new();
    let mut batch_sorted: Vec<u32> = Vec::new();
    let mut in_batch = 0usize;
    let mut kernels = kernels::State::default();
    let started = thread_cpu();
    for (file_no, path) in list.iter().enumerate() {
        totals.files += 1;
        let c0 = thread_cpu();
        let bytes = std::fs::read(path).expect("read");
        let mut builder = node_table::builder(&mut table, walk_kinds(walk));
        let ok = walk
            .walk(
                WalkRequest { path: languages::SourcePath(path), file: FileId(0), source: ByteSlice(&bytes) },
                RelationNeeds(&[]),
                &mut builder,
                &mut scratch,
                &mut NoVisit,
            )
            .is_ok();
        let t = node_table::finished(&mut builder);
        let n = node_table::element_count(t).0;
        let c1 = thread_cpu();
        phases.add("walk+table", c1 - c0);
        if !ok {
            continue;
        }
        totals.walked += 1;
        totals.elements += n as u64;
        if level == 0 {
            continue;
        }
        // Columns the experiment reads (the table keeps them private): kinds always; spans for literal tests; fields when
        // some rule anchors on a field.
        extract(t, &mut cols, level >= 3, fields_needed || level == 1);
        let c2 = thread_cpu();
        phases.add("extract columns", c2 - c1);
        if level == 1 {
            // own counting sort of the kind column, per batch of `batch` files.
            batch_kinds.extend_from_slice(&cols.kind);
            in_batch += 1;
            if in_batch == batch || file_no + 1 == list.len() {
                for _ in 0..repeat {
                    counting_sort(&batch_kinds, kinds_width.max(256), &mut batch_offsets, &mut batch_sorted);
                    black_box(&batch_sorted);
                }
                batch_kinds.clear();
                in_batch = 0;
            }
            phases.add("bucket (own counting sort, kinds)", (thread_cpu() - c2) / repeat as f64);
            if fields_needed {
                let c3 = thread_cpu();
                for _ in 0..repeat {
                    let Columns { field, kind_offsets, by_kind, .. } = &mut cols;
                    counting_sort(field, fields_width.max(1), kind_offsets, by_kind);
                    black_box(&by_kind);
                }
                phases.add("bucket (own counting sort, fields)", (thread_cpu() - c3) / repeat as f64);
            }
            continue;
        }
        if fields_needed {
            field_index(&mut cols, fields_width.max(1));
        }
        let c3 = thread_cpu();
        phases.add("field buckets + field presence", c3 - c2);
        let kinds: Vec<u64> = node_table::kinds_present(t).0.iter().map(|w| w.0).collect();
        let mut alive = vec![false; rules.len()];
        for r in 0..repeat {
            for (i, rule) in rules.iter().enumerate() {
                let s = survives(rule, &kinds, &cols.field_present);
                if r == 0 {
                    alive[i] = s;
                }
                black_box(s);
            }
        }
        let c4 = thread_cpu();
        phases.add("skip tests", (c4 - c3) / repeat as f64);
        totals.rule_files += rules.len() as u64;
        let alive_count = alive.iter().filter(|a| **a).count() as u64;
        totals.rule_files_alive += alive_count;
        for (i, a) in alive.iter().enumerate() {
            if *a {
                per_rule_files[i] += 1;
            }
        }
        if level == 2 {
            continue;
        }
        if level == 3 {
            for r in 0..repeat {
                for (i, rule) in rules.iter().enumerate() {
                    if !alive[i] {
                        continue;
                    }
                    for anchor in &rule.anchors {
                        out.clear();
                        let v = first_condition(t, &cols, &bytes, anchor, &mut out);
                        black_box(&out);
                        if r == 0 {
                            per_rule_entries[i] += v;
                            per_rule_survivors[i] += out.len() as u64;
                        }
                    }
                }
            }
            phases.add("first conditions (hand, per rule)", (thread_cpu() - c4) / repeat as f64);
            continue;
        }
        if level == 4 {
            for r in 0..repeat {
                out.clear();
                fanout.clear();
                let stamp = (file_no * repeat + r) as u64;
                for (i, rule) in rules.iter().enumerate() {
                    if !alive[i] {
                        continue;
                    }
                    for &c in &rule_conditions[i] {
                        if condition_stamp[c] != stamp {
                            condition_stamp[c] = stamp;
                            let from = out.len();
                            let v = first_condition(t, &cols, &bytes, &conditions[c], &mut out);
                            condition_range[c] = (from, out.len());
                            if r == 0 {
                                totals.shared_entries += v;
                                totals.shared_survivors += (out.len() - from) as u64;
                                totals.distinct_conditions += 1;
                            }
                        }
                        // fan out: the rule takes a reference to the shared survivors.
                        let (a, b) = condition_range[c];
                        fanout.push((i, a, b));
                        if r == 0 {
                            totals.shared_fanout += (b - a) as u64;
                        }
                    }
                }
                black_box(&fanout);
            }
            phases.add("first conditions (shared, fan out)", (thread_cpu() - c4) / repeat as f64);
            continue;
        }
        // kernels: every distinct condition over the batch (no per-file skip inside a batch).
        let c5 = thread_cpu();
        kernels.add_file(&cols, &bytes);
        phases.add("kernels: batch columns", thread_cpu() - c5);
        in_batch += 1;
        if in_batch == batch || file_no + 1 == list.len() {
            let c6 = thread_cpu();
            let arrays = kernels.arrays();
            let c7 = thread_cpu();
            phases.add("kernels: arrow arrays", c7 - c6);
            let scan = form == "scan";
            for r in 0..repeat {
                let k = if scan { kernels.scan(&conditions, &arrays) } else { kernels.bucketed(&conditions, &arrays, kinds_width.max(256), fields_width.max(1)) };
                if r == 0 {
                    totals.shared_entries += k.visited;
                    totals.shared_survivors += k.survivors;
                    totals.distinct_conditions += k.calls;
                }
            }
            phases.add(if scan { "first conditions (arrow kernels, scan form)" } else { "first conditions (arrow kernels, bucket form)" }, (thread_cpu() - c7) / repeat as f64);
            kernels.clear();
            in_batch = 0;
        }
    }
    for (i, a) in rules.iter().enumerate() {
        totals.entries += per_rule_entries[i];
        totals.survivors += per_rule_survivors[i];
        let _ = a;
    }
    let total = thread_cpu() - started;
    println!("mode {mode} form {form} batch {batch} repeat {repeat}: files {} walked {} elements {} rules {}", totals.files, totals.walked, totals.elements, rules.len());
    println!("thread CPU total {total:.4} s");
    for (n, s) in &phases.0 {
        println!("phase {n}: {s:.4} s");
    }
    println!(
        "rule-files {} alive {} entries {} survivors {} shared: distinct-condition-files {} entries {} survivors {} fanout {}",
        totals.rule_files, totals.rule_files_alive, totals.entries, totals.survivors, totals.distinct_conditions, totals.shared_entries, totals.shared_survivors, totals.shared_fanout
    );
    if let Some(path) = rules_out {
        let mut w = String::from("rule\tfiles_alive\tentries\tsurvivors\n");
        for (i, r) in rules.iter().enumerate() {
            w.push_str(&format!("{}\t{}\t{}\t{}\n", r.name, per_rule_files[i], per_rule_entries[i], per_rule_survivors[i]));
        }
        std::fs::write(path, w).expect("rules out");
    }
}

#[cfg(not(feature = "vec-arrow"))]
mod kernels {
    //! Stub when the Arrow crates are not linked.
    use super::{Anchor, Columns};
    #[derive(Default)]
    pub(super) struct State;
    #[derive(Default, Clone, Copy)]
    pub(super) struct Counts {
        pub(super) visited: u64,
        pub(super) survivors: u64,
        pub(super) calls: u64,
    }
    impl State {
        pub(super) fn add_file(&mut self, _: &Columns, _: &[u8]) {}
        pub(super) fn clear(&mut self) {}
        pub(super) fn arrays(&self) {}
        pub(super) fn scan(&self, _: &[Anchor], (): &()) -> Counts {
            panic!("built without the Arrow crates")
        }
        pub(super) fn bucketed(&mut self, _: &[Anchor], (): &(), _: usize, _: usize) -> Counts {
            panic!("built without the Arrow crates")
        }
    }
}

#[cfg(feature = "vec-arrow")]
#[path = "kernels.rs"]
mod kernels;
