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
    /// The name tokens spelling one required word (step 3).
    Word(u16),
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

fn words_of(bits: usize) -> usize {
    words(bits)
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
        Key::Word(_) => Bucket::Own(&[]),
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
        Some("funnel") => run_funnel(&walk, &vocab, &args[2..]),
        Some("catalog") => catalog_out(args[2]),
        _ => eprintln!("usage: vec census <files> <literals> <out> | vec run <mode> <files> <anchors> [--batch N] [--repeat K] [--rules-out F]"),
    }
}

/// spec step: every relation of the catalog with its columns and their cell kinds (`relation\tcolumn\tkind`), what the
/// planner reads to know a relation's file column.
fn catalog_out(out: &str) {
    use ryo_relations::domain::relations::service as catalog;
    let mut w = String::new();
    for r in catalog::all() {
        for c in catalog::columns(*r) {
            w.push_str(&format!("{}\t{}\t{:?}\n", catalog::name(*r).0, catalog::column_name(c).0, catalog::kind(c)));
        }
    }
    std::fs::write(out, w).expect("catalog out");
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

#[path = "funnel.rs"]
mod funnel;
#[path = "facts.rs"]
mod facts;
#[path = "spec.rs"]
mod spec;

/// `vec funnel <way> <files> <anchors> <plans> [--repeat K] [--rules-out F] [--hits-out F]`: ways `cands`, `structure`,
/// `interp`, `spec`, `knock`.
fn run_funnel(walk: &impl Walk, vocab: &Vocab, args: &[&str]) {
    let way = args[0];
    let list = files(args[1]);
    let rules = load_rules(args[2], vocab);
    let mut fact_catalog = facts::Catalog::default();
    let mut interner = facts::Interner { ids: facts::Table::named(&flag(args, "--hasher").unwrap_or_else(|| "std".to_owned())), ..Default::default() };
    let (frules, mut words) = funnel::load(args[3], vocab, &mut fact_catalog, &mut interner);
    let use_facts = args.contains(&"--facts");
    // spec step: the derived requirements and entries
    let use_req = args.contains(&"--req");
    let entry_mode = flag(args, "--entry").unwrap_or_else(|| "census".to_owned());
    let presence_all = flag(args, "--presence").as_deref() == Some("all");
    let root = flag(args, "--root").unwrap_or_default();
    let (srules, atoms) = if use_req || way == "entry" {
        let mut sl = spec::Load { vocab, words: &mut words, catalog: &mut fact_catalog, interner: &mut interner, atoms: spec::Atoms::default() };
        let r = spec::load(args[3], entry_mode == "census", &mut sl);
        (r, sl.atoms)
    } else {
        (Vec::new(), spec::Atoms::default())
    };
    assert!(!use_req || use_facts, "--req needs --facts (row atoms are tested where the facts are lent)");
    let n_atoms = atoms.list.len();
    let aw = n_atoms.div_ceil(64).max(1);
    let atoms_ac = aho_corasick::AhoCorasick::new(&atoms.bytes).expect("byte literals");
    let mut file_bits = vec![0u64; aw];
    let mut clause_bits: Vec<u64> = Vec::new();
    let mut file_alive = vec![false; srules.len()];
    let mut req_bytes_time = 0f64;
    let want_why = flag(args, "--why-out").is_some();
    let skip_compare = want_why || args.contains(&"--skip-compare");
    // spec step: the (rule, file) pairs alive before (another run's `--alive-out`): the why lines are written for those
    let before: std::collections::HashSet<(String, String)> = flag(args, "--before-alive")
        .map(|p| std::fs::read_to_string(p).expect("before alive").lines().filter_map(|l| l.split_once('\t').map(|(r, f)| (r.to_owned(), f.to_owned()))).collect())
        .unwrap_or_default();
    let byte_scan = flag(args, "--byte-scan").unwrap_or_else(|| "each".to_owned());
    let (mut req_time, mut row_time) = (0f64, 0f64);
    let (mut row_tests, mut row_present) = (0u64, 0u64);
    let mut presence_only_fills = vec![0u64; 0];
    let mut why_lines = String::new();
    let mut pairs_lines = String::new();
    let (mut old_only, mut new_only, mut both_alive) = (0u64, 0u64, 0u64);
    let slots_n = fact_catalog.relations.len();
    presence_only_fills.resize(slots_n, 0);
    let mut store = facts::Store { remap_on: args.contains(&"--remap"), ..Default::default() };
    let token_pass = args.contains(&"--token-pass");
    let check_links = args.contains(&"--check-token-links");
    let token_slot = fact_catalog.names.iter().position(|n| n == "token-link");
    let kind_code = |n: &str| vocab.kinds.get(n).copied().expect("kind");
    let token_kinds = facts::TokenKinds {
        first: kind_code("name-token"),
        last: kind_code("colonequal"),
        error: kind_code("errortoken"),
        pairs: [(kind_code("lpar"), kind_code("rpar")), (kind_code("lsqb"), kind_code("rsqb")), (kind_code("lbrace"), kind_code("rbrace"))],
    };
    let link_ids = [u64::from(interner.id(b"next-token")), u64::from(interner.id(b"matching-delimiter")), u64::from(interner.id(b"enclosing-delimiter"))];
    let (mut token_time, mut token_files, mut model_only_files, mut token_only_files) = (0f64, 0u64, 0u64, 0u64);
    let (mut link_rows_provider, mut link_rows_mine, mut link_mismatch) = (0u64, 0u64, 0u64);
    let mut fwords = ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names");
    let mut filled = languages::FilledColumns::default();
    let (mut fill_time, mut write_time) = (vec![0f64; slots_n], vec![0f64; slots_n]);
    let (mut fill_files, mut fill_rows, mut unanswered) = (vec![0u64; slots_n], vec![0u64; slots_n], vec![0u64; slots_n]);
    let mut model_files = 0u64;
    let mut decide_time = 0f64;
    let word_src = flag(args, "--words").unwrap_or_else(|| "none".to_owned());
    let word_skip = args.contains(&"--word-skip");
    let compare = args.contains(&"--compare");
    let n_words = words.names.len();
    let ww = n_words.div_ceil(64).max(1);
    let name_token = vocab.kinds.get("name-token").copied().expect("name-token kind");
    let ac = aho_corasick::AhoCorasick::builder()
        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
        .build(words.names.iter().map(String::as_bytes))
        .expect("aho-corasick");
    let bytes_ac = aho_corasick::AhoCorasick::new([&b"\x08"[..], b"\x1a", b"\x1b", b"\x00", "\u{200b}".as_bytes(), b"\\"]).expect("byte patterns");
    let mut byte_files = [0u64; 8];
    let mut byte_hits = 0u64;
    let mut present = vec![0u64; ww];
    let mut table_present = vec![0u64; ww];
    let mut word_pairs: Vec<(u16, u32)> = Vec::new();
    let mut word_offsets: Vec<u32> = Vec::new();
    let mut word_pos: Vec<u32> = Vec::new();
    let (mut missing_in_src, mut extra_in_src, mut present_total, mut distinct_total) = (0u64, 0u64, 0u64, 0u64);
    let (mut alive_kinds, mut alive_both, mut rule_files) = (0u64, 0u64, 0u64);
    let mut word_entry_bound = 0u64;
    let mut alive_pairs: Vec<(usize, usize)> = Vec::new();
    let repeat: usize = flag(args, "--repeat").map_or(1, |b| b.parse().expect("repeat"));
    let by_name: HashMap<&str, usize> = rules.iter().enumerate().map(|(i, r)| (r.name.as_str(), i)).collect();
    let skip_of: Vec<Option<usize>> = frules.iter().map(|r| by_name.get(r.name.as_str()).copied()).collect();
    let fields_width = vocab.field_names.len();
    let set_of = |names: &[&str]| {
        let mut s = [0u64; 4];
        for n in names {
            if let Some(c) = vocab.kinds.get(*n) {
                s[usize::from(*c) / 64] |= 1 << (c % 64);
            }
        }
        s
    };
    let function = set_of(&["function-def", "async-function-def", "lambda"]);
    let class = set_of(&["class-def"]);
    let looping = set_of(&["for", "async-for", "while"]);
    let mut units = funnel::units(&frules);
    if args.contains(&"--only-knock-units") {
        units.retain(|u| u.knock);
    }
    let group_list = funnel::groups(&frules, &units);
    let structured = matches!(way, "structure" | "spec" | "knock" | "entry");
    let max_regs = frules.iter().flat_map(|r| r.plans.iter().map(|p| p.regs)).max().unwrap_or(1).max(1);
    let mut cands = vec![0u64; units.len()];
    let mut surv = vec![0u64; units.len()];
    let mut unit_steps = vec![0u64; units.len()];
    let mut knock_steps = 0u64;
    let mut hits: Vec<(usize, usize)> = Vec::new();
    let mut rule_cands = vec![0u64; frules.len()];
    let mut rule_surv = vec![0u64; frules.len()];
    let mut phases = Phases::default();
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let mut cols = Columns::default();
    let mut st = funnel::Structure::default();
    let mut regs = vec![0u32; max_regs + 1];
    let mut rows = funnel::Rows::default();
    let mut file_mask: Vec<u64> = Vec::new();
    let mut marked: Vec<u32> = Vec::new();
    let mut stamp = 0u32;
    let mut last = vec![0u32; units.len()];
    let mut buf: Vec<u32> = Vec::new();
    let mut remaining: Vec<u32> = Vec::new();
    let started = thread_cpu();
    for (file_no, path) in list.iter().enumerate() {
        let c0 = thread_cpu();
        let bytes = std::fs::read(path).expect("read");
        store.reset(slots_n);
        let mut visit_time = 0f64;
        let ok = if use_facts {
            // the provider's facts are lent only while the walk runs: decide here which files need them (some rule that
            // tests a fact survived the kinds-and-words skip), then fill those relations and write their columns
            ryo_python::fixtures::walking::walked(walk, path, &bytes, &[], &mut table, &mut scratch, &mut |file| {
                let v0 = thread_cpu();
                let ft = file.table;
                let n = node_table::element_count(ft).0;
                let kinds: Vec<u64> = node_table::kinds_present(ft).0.iter().map(|w| w.0).collect();
                let mut fp = vec![0u64; words_of(fields_width)];
                for i in 0..n as u32 {
                    let f = node_table::field_of(ft, Position(i)).0;
                    if usize::from(f) < fields_width {
                        fp[usize::from(f) / 64] |= 1 << (f % 64);
                    }
                }
                let mut wp = vec![0u64; ww];
                for p in node_table::ids_of_kind(ft, KindBits(name_token)) {
                    let sp = node_table::span_of(ft, *p);
                    if let Some(w) = words.ids.get(&bytes[sp.start.0 as usize..sp.end.0 as usize]) {
                        wp[usize::from(*w) / 64] |= 1 << (w % 64);
                    }
                }
                let mut needed = vec![false; slots_n];
                let mut for_plans = vec![false; slots_n];
                if use_req {
                    // stage 1: every atom but the provider rows (those count as present until tested)
                    let q0 = thread_cpu();
                    file_bits.iter_mut().for_each(|w| *w = 0);
                    let rel = path.strip_prefix(root.as_str()).unwrap_or(path).trim_start_matches('/');
                    let set = |b: &mut Vec<u64>, a: u32| b[a as usize / 64] |= 1 << (a % 64);
                    for (a, atom) in atoms.list.iter().enumerate() {
                        let on = match atom {
                            spec::Atom::Kind(k) => kinds.get(usize::from(*k) / 64).is_some_and(|w| w >> (k % 64) & 1 == 1),
                            spec::Atom::Field(f) => fp.get(usize::from(*f) / 64).is_some_and(|w| w >> (f % 64) & 1 == 1),
                            spec::Atom::Word(w) => funnel::bit(&wp, *w),
                            spec::Atom::Path(i, negated) => atoms.paths[*i].is_match(rel) != *negated,
                            spec::Atom::Row(_) | spec::Atom::Unknown => true,
                            spec::Atom::Bytes => false,
                        };
                        if on {
                            set(&mut file_bits, a as u32);
                        }
                    }
                    let qb = thread_cpu();
                    if byte_scan == "ac" {
                        for m in atoms_ac.find_overlapping_iter(&bytes[..]) {
                            set(&mut file_bits, atoms.byte_atom[m.pattern().as_usize()]);
                        }
                    } else {
                        // the file's byte set (one pass), then each longer literal until its first occurrence
                        let mut seen = [false; 256];
                        for b in &bytes {
                            seen[usize::from(*b)] = true;
                        }
                        for (b, a) in &atoms.single {
                            if seen[usize::from(*b)] {
                                set(&mut file_bits, *a);
                            }
                        }
                        for i in &atoms.multi {
                            if atoms.bytes[*i].iter().all(|b| seen[usize::from(*b)]) && atoms.finders[*i].is_match(&bytes[..]) {
                                set(&mut file_bits, atoms.byte_atom[*i]);
                            }
                        }
                    }
                    req_bytes_time += thread_cpu() - qb;
                    atoms.clause_bits(&file_bits, &mut clause_bits);
                    for (ri, sr) in srules.iter().enumerate() {
                        let reads = !frules[ri].slots.is_empty() || !sr.slots.is_empty() || (presence_all && !sr.req_slots.is_empty());
                        if !reads || !sr.alive(&clause_bits) {
                            continue;
                        }
                        for s in frules[ri].slots.iter().chain(&sr.slots) {
                            needed[*s] = true;
                            for_plans[*s] = true;
                        }
                        if presence_all {
                            for s in &sr.req_slots {
                                needed[*s] = true;
                            }
                        }
                    }
                    req_time += thread_cpu() - q0;
                } else {
                    for (ri, r) in frules.iter().enumerate() {
                        if r.slots.is_empty() || !skip_of[ri].is_none_or(|i| survives(&rules[i], &kinds, &fp)) || !r.words_alive(&wp) {
                            continue;
                        }
                        for s in &r.slots {
                            needed[*s] = true;
                            for_plans[*s] = true;
                        }
                    }
                }
                for s in 0..slots_n {
                    if needed[s] && !for_plans[s] {
                        presence_only_fills[s] += 1;
                    }
                }
                let v1 = thread_cpu();
                decide_time += v1 - v0;
                let model_needed = needed.iter().enumerate().any(|(i, x)| *x && Some(i) != token_slot);
                let tokens_needed = token_slot.is_some_and(|t| needed[t]);
                if model_needed {
                    model_files += 1;
                }
                if tokens_needed && !model_needed {
                    token_only_files += 1;
                }
                if model_needed && !tokens_needed {
                    model_only_files += 1;
                }
                if tokens_needed {
                    token_files += 1;
                }
                languages::facts::clear_words(&mut fwords);
                for slot in 0..slots_n {
                    if !needed[slot] {
                        continue;
                    }
                    if Some(slot) == token_slot && (token_pass || check_links) {
                        let a = thread_cpu();
                        store.token_links(&fact_catalog, slot, ft, &token_kinds, link_ids, n);
                        token_time += thread_cpu() - a;
                        if check_links {
                            let col = |name: &str| fact_catalog.column(slot, name);
                            let (sc, tc, kc) = (col("source"), col("target"), col("link-kind"));
                            let mine: std::collections::BTreeSet<(u64, u64, u64)> = (0..store.rows[slot]).map(|r| (store.cells[slot][sc][r], store.cells[slot][tc][r], store.cells[slot][kc][r])).collect();
                            link_rows_mine += store.rows[slot] as u64;
                            filled.0.clear();
                            let _ = file.facts.fill(fact_catalog.relations[slot], languages::Wanted::Whole, &mut fwords, &mut filled);
                            store.write(&fact_catalog, slot, &filled, &fwords, &mut interner, n);
                            let theirs: std::collections::BTreeSet<(u64, u64, u64)> = (0..store.rows[slot]).map(|r| (store.cells[slot][sc][r], store.cells[slot][tc][r], store.cells[slot][kc][r])).collect();
                            link_rows_provider += store.rows[slot] as u64;
                            link_mismatch += mine.symmetric_difference(&theirs).count() as u64;
                        }
                        continue;
                    }
                    let a = thread_cpu();
                    filled.0.clear();
                    let answer = file.facts.fill(fact_catalog.relations[slot], languages::Wanted::Whole, &mut fwords, &mut filled);
                    let b = thread_cpu();
                    fill_time[slot] += b - a;
                    fill_files[slot] += 1;
                    if matches!(answer, languages::FactAnswer::Answered(_)) {
                        store.write(&fact_catalog, slot, &filled, &fwords, &mut interner, n);
                        fill_rows[slot] += store.rows[slot] as u64;
                    } else {
                        unanswered[slot] += 1;
                    }
                    write_time[slot] += thread_cpu() - b;
                }
                if use_req {
                    // stage 2: the provider rows of the slots filled for this file
                    let r0 = thread_cpu();
                    let (t, p) = atoms.test_rows(&store, &interner.matches, &mut file_bits);
                    row_tests += t;
                    row_present += p;
                    atoms.clause_bits(&file_bits, &mut clause_bits);
                    for (ri, sr) in srules.iter().enumerate() {
                        file_alive[ri] = sr.alive(&clause_bits);
                    }
                    row_time += thread_cpu() - r0;
                }
                visit_time += thread_cpu() - v0;
            })
            .is_ok()
        } else {
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
            let _ = node_table::finished(&mut builder);
            ok
        };
        let t = &table;
        let c1 = thread_cpu();
        phases.add("walk+table", c1 - c0 - visit_time);
        if !ok {
            continue;
        }
        extract(t, &mut cols, true, true);
        funnel::parents(t, &cols, &mut st);
        field_index(&mut cols, fields_width.max(1));
        let c2 = thread_cpu();
        phases.add("extract columns + parents + field buckets", c2 - c1);
        if structured {
            for _ in 0..repeat {
                funnel::structure(t, &cols, &mut st, &function, &class, &looping);
            }
            phases.add("structure pass", (thread_cpu() - c2) / repeat as f64);
            if !fact_catalog.enc_sets.is_empty() {
                let e0 = thread_cpu();
                funnel::enclosing(&cols, &mut st, &fact_catalog.enc_sets);
                phases.add("enclosing columns (one per kind set)", thread_cpu() - e0);
            }
        }
        if way == "structure" {
            continue;
        }
        // the file's required words
        let cw = thread_cpu();
        if word_src != "none" {
            let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80;
            let from_table = |present: &mut Vec<u64>, pairs: &mut Vec<(u16, u32)>| {
                present.iter_mut().for_each(|w| *w = 0);
                pairs.clear();
                for p in node_table::ids_of_kind(t, KindBits(name_token)) {
                    let i = p.0 as usize;
                    if let Some(w) = words.ids.get(&bytes[cols.start[i] as usize..cols.end[i] as usize]) {
                        present[usize::from(*w) / 64] |= 1 << (w % 64);
                        pairs.push((*w, p.0));
                    }
                }
            };
            match word_src.as_str() {
                "table" => {
                    from_table(&mut present, &mut word_pairs);
                    // the index: name-token positions of each required word
                    word_offsets.clear();
                    word_offsets.resize(n_words + 1, 0);
                    for (w, _) in &word_pairs {
                        word_offsets[usize::from(*w) + 1] += 1;
                    }
                    for i in 0..n_words {
                        word_offsets[i + 1] += word_offsets[i];
                    }
                    word_pos.clear();
                    word_pos.resize(word_pairs.len(), 0);
                    let mut fill: Vec<u32> = word_offsets[..n_words].to_vec();
                    for (w, p) in &word_pairs {
                        word_pos[fill[usize::from(*w)] as usize] = *p;
                        fill[usize::from(*w)] += 1;
                    }
                }
                "all" => {
                    let mut set: std::collections::HashSet<&[u8]> = std::collections::HashSet::new();
                    for p in node_table::ids_of_kind(t, KindBits(name_token)) {
                        let i = p.0 as usize;
                        let _ = set.insert(&bytes[cols.start[i] as usize..cols.end[i] as usize]);
                    }
                    distinct_total += set.len() as u64;
                    present.iter_mut().for_each(|w| *w = 0);
                    for x in &set {
                        if let Some(w) = words.ids.get(*x) {
                            present[usize::from(*w) / 64] |= 1 << (w % 64);
                        }
                    }
                }
                "bytes" => {
                    present.iter_mut().for_each(|w| *w = 0);
                    let mut i = 0;
                    while i < bytes.len() {
                        if is_word(bytes[i]) {
                            let a = i;
                            while i < bytes.len() && is_word(bytes[i]) {
                                i += 1;
                            }
                            if let Some(w) = words.ids.get(&bytes[a..i]) {
                                present[usize::from(*w) / 64] |= 1 << (w % 64);
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
                "ac" => {
                    present.iter_mut().for_each(|w| *w = 0);
                    for m in ac.find_iter(&bytes) {
                        let (a, b) = (m.start(), m.end());
                        if (a == 0 || !is_word(bytes[a - 1])) && (b == bytes.len() || !is_word(bytes[b])) {
                            let w = m.pattern().as_usize();
                            present[w / 64] |= 1 << (w % 64);
                        }
                    }
                }
                "bytes6" => {
                    // the byte patterns the source-matches rules need (PLE2510..2515, W605): one multi-pattern scan
                    let mut hit = 0u64;
                    for m in bytes_ac.find_iter(&bytes) {
                        hit |= 1 << m.pattern().as_usize();
                    }
                    present[0] = hit;
                    byte_files[hit.count_ones() as usize] += 1;
                    byte_hits |= hit;
                }
                other => panic!("--words {other}"),
            }
            phases.add(if word_src == "bytes6" { "byte-pattern pass (aho-corasick, 6 patterns)" } else if word_src == "table" { "word pass (name tokens -> required-word bitset + index)" } else if word_src == "all" { "word pass (distinct word set of all name tokens)" } else if word_src == "bytes" { "word pass (identifier runs of the source bytes)" } else { "word pass (aho-corasick over the source bytes)" }, thread_cpu() - cw);
            present_total += present.iter().map(|w| u64::from(w.count_ones())).sum::<u64>();
            if compare {
                from_table(&mut table_present, &mut word_pairs);
                for i in 0..ww {
                    missing_in_src += u64::from((table_present[i] & !present[i]).count_ones());
                    extra_in_src += u64::from((present[i] & !table_present[i]).count_ones());
                }
            }
        }
        if way == "words" {
            continue;
        }
        let c3 = thread_cpu();
        let kinds: Vec<u64> = node_table::kinds_present(t).0.iter().map(|w| w.0).collect();
        let kinds_alive: Vec<bool> = if use_req && !skip_compare {
            Vec::new()
        } else {
            skip_of.iter().map(|i| i.is_none_or(|i| survives(&rules[i], &kinds, &cols.field_present))).collect()
        };
        let old_alive: Vec<bool> = if use_req && !skip_compare {
            Vec::new()
        } else if word_skip {
            kinds_alive.iter().zip(&frules).map(|(a, r)| *a && r.words_alive(&present)).collect()
        } else {
            kinds_alive.clone()
        };
        let alive: Vec<bool> = if use_req {
            let new: Vec<bool> = file_alive.clone();
            for (ri, (o, n)) in old_alive.iter().zip(&new).enumerate().filter(|_| skip_compare) {
                let o = if before.is_empty() { o } else { &before.contains(&(frules[ri].name.clone(), path.clone())) };
                match (*o, *n) {
                    (true, true) => both_alive += 1,
                    (false, true) => new_only += 1,
                    (true, false) if !want_why => old_only += 1,
                    (true, false) => {
                        old_only += 1;
                        // why: per branch, its first failing clause (atom classes in a fixed order of cost)
                        let mut why: Vec<String> = Vec::new();
                        for c in &srules[ri].req {
                            if let Some(cl) = c.iter().map(|i| &atoms.clauses[*i as usize]).find(|cl| !cl.iter().any(|a| funnel::bit32(&file_bits, *a))) {
                                let names: Vec<&str> = cl.iter().map(|a| atoms.names[*a as usize].as_str()).collect();
                                let class = spec::class(&atoms.list[cl[0] as usize]);
                                why.push(format!("{class}={}", names.join("|")));
                            }
                        }
                        why.sort();
                        why.dedup();
                        why_lines.push_str(&format!("{}\t{}\t{}\n", frules[ri].name, path, why.join(" ; ")));
                    }
                    (false, false) => {}
                }
            }
            new
        } else {
            old_alive.clone()
        };
        rule_files += frules.len() as u64;
        alive_kinds += kinds_alive.iter().filter(|a| **a).count() as u64;
        alive_both += alive.iter().filter(|a| **a).count() as u64;
        for (ri, a) in alive.iter().enumerate() {
            if *a {
                alive_pairs.push((ri, file_no));
            }
        }
        phases.add("skip tests", thread_cpu() - c3);
        let ctx = funnel::Ctx {
            table: t,
            src: &bytes,
            kind: &cols.kind,
            field: &cols.field,
            start: &cols.start,
            end: &cols.end,
            parent: &st.parent,
            pkind: if structured { &st.pkind } else { &[] },
            cmask: if structured { &st.cmask } else { &[] },
            facts: &store,
            enc: &st.enc,
            matches: &interner.matches,
        };
        if marked.len() < cols.kind.len() {
            marked.resize(cols.kind.len(), 0);
        }
        let fill = |key: &Key, buf: &mut Vec<u32>| {
            buf.clear();
            match *key {
                Key::Word(w) => buf.extend_from_slice(&word_pos[word_offsets[usize::from(w)] as usize..word_offsets[usize::from(w) + 1] as usize]),
                Key::Kind(k) => buf.extend(node_table::ids_of_kind(t, KindBits(k)).iter().map(|p| p.0)),
                Key::Field(f) => {
                    let (a, b) = (cols.field_offsets[usize::from(f)] as usize, cols.field_offsets[usize::from(f) + 1] as usize);
                    buf.extend_from_slice(&cols.by_field[a..b]);
                }
            }
        };
        if way == "entry" {
            // spec step: each live branch entered at one of its alternatives, chosen per file (fixed: the most specific
            // level first, then the fewest candidates; fewest: the fewest candidates, then the level), exact bucket sizes
            let e0 = thread_cpu();
            let key_len = |k: &Key| -> usize {
                match *k {
                    Key::Word(w) => (word_offsets[usize::from(w) + 1] - word_offsets[usize::from(w)]) as usize,
                    Key::Kind(k) => node_table::ids_of_kind(t, KindBits(k)).len(),
                    Key::Field(f) => (cols.field_offsets[usize::from(f) + 1] - cols.field_offsets[usize::from(f)]) as usize,
                }
            };
            for (ri, sr) in srules.iter().enumerate() {
                if !alive[ri] || sr.groups.is_empty() {
                    continue;
                }
                stamp = stamp.wrapping_add(1);
                let (mut rc, mut rs) = (0u64, 0u64);
                let mut chosen: Vec<(usize, usize)> = Vec::new();
                for (gi, g) in sr.groups.iter().enumerate() {
                    if use_req && !spec::holds(&g.req, &clause_bits) {
                        continue;
                    }
                    if word_skip && !funnel::cnf_holds(&g.words, &present) {
                        continue;
                    }
                    let mut best: Option<(u8, usize, usize)> = None;
                    for (ai, a) in g.alts.iter().enumerate() {
                        let n = match &a.key {
                            spec::EKey::Keys(ks) => ks.iter().map(key_len).sum(),
                            spec::EKey::Fact(f) => {
                                if !store.answered(f.slot) {
                                    continue;
                                }
                                store.key_nodes(f.slot, f.keys[0].0, &f.conds, &interner.matches, &mut buf);
                                buf.len()
                            }
                        };
                        let better = match best {
                            None => true,
                            Some((l, c, _)) => {
                                if entry_mode == "fewest" {
                                    (n, a.level) < (c, l)
                                } else {
                                    (a.level, n) < (l, c)
                                }
                            }
                        };
                        if better {
                            best = Some((a.level, n, ai));
                        }
                    }
                    let Some((_, _, ai)) = best else { continue };
                    chosen.push((gi, ai));
                }
                // the branches entered at the same bucket share it: filled once, each plan over the candidates no
                // earlier branch kept (as the units of the census form)
                chosen.sort_by_key(|(gi, ai)| sr.groups[*gi].alts[*ai].key_text.clone());
                let mut k = 0;
                while k < chosen.len() {
                    let key_text = &sr.groups[chosen[k].0].alts[chosen[k].1].key_text;
                    let mut end = k;
                    while end < chosen.len() && &sr.groups[chosen[end].0].alts[chosen[end].1].key_text == key_text {
                        end += 1;
                    }
                    let first = &sr.groups[chosen[k].0].alts[chosen[k].1];
                    match &first.key {
                        spec::EKey::Keys(ks) => {
                            buf.clear();
                            let mut part = Vec::new();
                            for key in ks {
                                fill(key, &mut part);
                                buf.extend_from_slice(&part);
                            }
                        }
                        spec::EKey::Fact(f) => store.key_nodes(f.slot, f.keys[0].0, &f.conds, &interner.matches, &mut buf),
                    }
                    rc += buf.len() as u64;
                    remaining.clear();
                    remaining.extend_from_slice(&buf);
                    for &(gi, ai) in &chosen[k..end] {
                        if remaining.is_empty() {
                            break;
                        }
                        let plan = &sr.groups[gi].alts[ai].plan;
                        let mut st = 0u64;
                        rows.r = plan.regs.max(1);
                        rows.d.clear();
                        rows.d.resize(remaining.len() * rows.r, 0);
                        rows.tag.clear();
                        for (j, n) in remaining.iter().enumerate() {
                            rows.d[j * rows.r] = *n;
                            rows.tag.push(j as u32);
                        }
                        for stage in &plan.stages {
                            if rows.len() == 0 {
                                break;
                            }
                            stage(&ctx, &mut rows, &mut st);
                        }
                        for j in 0..rows.len() {
                            let n = rows.d[j * rows.r] as usize;
                            if marked[n] != stamp {
                                marked[n] = stamp;
                                rs += 1;
                            }
                        }
                        remaining.retain(|n| marked[*n as usize] != stamp);
                    }
                    k = end;
                }
                rule_cands[ri] += rc;
                rule_surv[ri] += rs;
                if rs > 0 {
                    hits.push((ri, file_no));
                }
                if rc > 0 || rs > 0 {
                    pairs_lines.push_str(&format!("{}\t{}\t{rc}\t{rs}\n", frules[ri].name, path));
                }
            }
            phases.add("entries (choose, enter, prefixes)", thread_cpu() - e0);
            continue;
        }
        let c4 = thread_cpu();
        for r in 0..repeat {
            let first = r == 0;
            let mut file_surv = vec![0u64; units.len()];
            let k0 = thread_cpu();
            if way == "knock" {
                for u in &mut last {
                    *u = 0;
                }
                for g in &group_list {
                    file_mask.clear();
                    file_mask.resize(g.words(), 0);
                    let mut i = 0;
                    for (slot, u) in g.slot_unit.iter().enumerate() {
                        if alive[units[*u].rule] {
                            file_mask[slot / 64] |= 1 << (slot % 64);
                        }
                        i += 1;
                    }
                    let _ = i;
                    if file_mask.iter().all(|w| *w == 0) {
                        continue;
                    }
                    fill(&g.key, &mut buf);
                    let s = funnel::knock(&ctx, g, &buf, &file_mask, &mut file_surv, &mut last);
                    if first {
                        knock_steps += s;
                        for u in &g.units {
                            if alive[units[*u].rule] {
                                cands[*u] += buf.len() as u64;
                            }
                        }
                    }
                }
            }
            if way == "knock" {
                phases.add("prefixes: knock-out units", (thread_cpu() - k0) / repeat as f64);
            }
            for (ui, unit) in units.iter().enumerate() {
                if !alive[unit.rule] || (way == "knock" && unit.knock) {
                    continue;
                }
                let rule = &frules[unit.rule];
                if word_skip && !unit.plans.iter().any(|p| funnel::cnf_holds(&rule.plans[*p].words, &present)) {
                    continue;
                }
                fill(&unit.key, &mut buf);
                if first && word_src == "table" {
                    // what entering from the rarest required word would start from: per plan the fewest name tokens of
                    // a clause's words, never more than the bucket
                    for p in &unit.plans {
                        let plan = &rule.plans[*p];
                        if word_skip && !funnel::cnf_holds(&plan.words, &present) {
                            continue;
                        }
                        let fewest = plan
                            .words
                            .iter()
                            .map(|cl| cl.iter().map(|w| u64::from(word_offsets[usize::from(*w) + 1] - word_offsets[usize::from(*w)])).sum::<u64>())
                            .min()
                            .unwrap_or(u64::MAX);
                        word_entry_bound += fewest.min(buf.len() as u64);
                    }
                }
                let mut n_s = 0u64;
                let mut s = 0u64;
                match way {
                    "cands" => {}
                    "interp" => {
                        for &n in &buf {
                            for p in &unit.plans {
                                regs[0] = n;
                                if funnel::exec(&ctx, &rule.plans[*p].ops, 0, &mut regs, &mut s) {
                                    n_s += 1;
                                    break;
                                }
                            }
                        }
                    }
                    _ => {
                        // each plan runs over the candidates no earlier plan of the unit kept
                        stamp = stamp.wrapping_add(1);
                        remaining.clear();
                        remaining.extend_from_slice(&buf);
                        for p in &unit.plans {
                            if remaining.is_empty() {
                                break;
                            }
                            let plan = &rule.plans[*p];
                            if word_skip && !funnel::cnf_holds(&plan.words, &present) {
                                continue;
                            }
                            rows.r = plan.regs.max(1);
                            rows.d.clear();
                            rows.d.resize(remaining.len() * rows.r, 0);
                            rows.tag.clear();
                            for (j, n) in remaining.iter().enumerate() {
                                rows.d[j * rows.r] = *n;
                                rows.tag.push(j as u32);
                            }
                            for stage in &plan.stages {
                                if rows.len() == 0 {
                                    break;
                                }
                                stage(&ctx, &mut rows, &mut s);
                            }
                            for j in 0..rows.len() {
                                let n = rows.d[j * rows.r] as usize;
                                if marked[n] != stamp {
                                    marked[n] = stamp;
                                    n_s += 1;
                                }
                            }
                            remaining.retain(|n| marked[*n as usize] != stamp);
                        }
                    }
                }
                if first {
                    cands[ui] += buf.len() as u64;
                    file_surv[ui] += n_s;
                    unit_steps[ui] += s;
                }
            }
            if first {
                let mut any = vec![false; frules.len()];
                let mut per_rule = vec![0u64; frules.len()];
                for (ui, unit) in units.iter().enumerate() {
                    surv[ui] += file_surv[ui];
                    per_rule[unit.rule] += file_surv[ui];
                    if file_surv[ui] > 0 {
                        any[unit.rule] = true;
                    }
                }
                for (ri, n) in per_rule.iter().enumerate() {
                    if *n > 0 {
                        pairs_lines.push_str(&format!("{}\t{}\t0\t{n}\n", frules[ri].name, path));
                    }
                }
                for (ri, a) in any.iter().enumerate() {
                    if *a {
                        hits.push((ri, file_no));
                    }
                }
            }
        }
        phases.add("prefixes", (thread_cpu() - c4) / repeat as f64);
    }
    let total = thread_cpu() - started;
    println!(
        "words: {n_words} required words; word src {word_src}; present (file, word) {present_total}; distinct name-token words summed over files {distinct_total}; compare: in the table but not the source scan {missing_in_src}, in the source scan only {extra_in_src}"
    );
    println!("byte patterns: files by number of patterns present {byte_files:?}; patterns seen anywhere {byte_hits:#b}");
    println!("skip: rule-files {rule_files} alive by kinds {alive_kinds} by kinds and words {alive_both}; entry bound from the rarest required word {word_entry_bound}");
    if use_facts {
        println!("facts: files that needed the model (some fact-testing rule survived the skip) {model_files}; deciding {decide_time:.4} s");
        for slot in 0..slots_n {
            println!(
                "fact {}: files {} rows {} unanswered {} provider fill {:.4} s column write {:.4} s",
                fact_catalog.names[slot], fill_files[slot], fill_rows[slot], unanswered[slot], fill_time[slot], write_time[slot]
            );
        }
        println!(
            "files: needing the semantic model {model_files}, needing token links {token_files}, token links only {token_only_files}, model only {model_only_files}; token pass {token_time:.4} s; link check: mine {link_rows_mine} provider {link_rows_provider} differing {link_mismatch}; enclosing kind sets {}; text lookups {}",
            fact_catalog.enc_sets.len(),
            interner.lookups
        );
        println!("facts total: provider fills {:.4} s column writes {:.4} s; interned texts {}", fill_time.iter().sum::<f64>(), write_time.iter().sum::<f64>(), interner.matches.len());
    }
    let knock_units = units.iter().filter(|u| u.knock).count();
    let steps: u64 = unit_steps.iter().sum();
    println!(
        "funnel {way} repeat {repeat}: rules {} with plans {} plans {} units {} knock-out units {knock_units} groups {} (tests {}, words {})",
        frules.len(),
        frules.iter().filter(|r| !r.plans.is_empty()).count(),
        frules.iter().map(|r| r.plans.len()).sum::<usize>(),
        units.len(),
        group_list.len(),
        group_list.iter().map(funnel::Group::tests).sum::<usize>(),
        group_list.iter().map(funnel::Group::words).sum::<usize>()
    );
    println!("thread CPU total {total:.4} s");
    for (n, s) in &phases.0 {
        println!("phase {n}: {s:.4} s");
    }
    let knock_c: u64 = units.iter().enumerate().filter(|(_, u)| u.knock).map(|(i, _)| cands[i]).sum();
    let knock_s: u64 = units.iter().enumerate().filter(|(_, u)| u.knock).map(|(i, _)| surv[i]).sum();
    println!(
        "candidates {} survivors {} steps {steps} knock-steps {knock_steps} (knock-out units: candidates {knock_c} survivors {knock_s}) rule-files-with-survivors {}",
        cands.iter().sum::<u64>(),
        surv.iter().sum::<u64>(),
        hits.len()
    );
    if use_req || way == "entry" {
        println!(
            "spec: atoms {n_atoms} (byte literals {}, path tests {}, row tests {}); req pass {req_time:.4} s (of it the byte scan {req_bytes_time:.4} s); row presence {row_time:.4} s ({row_tests} tests, {row_present} present); rule-files alive old and new {both_alive}, old only {old_only}, new only {new_only}",
            atoms.bytes.len(),
            atoms.paths.len(),
            atoms.rows.len()
        );
        let po: Vec<String> = (0..slots_n).filter(|s| presence_only_fills[*s] > 0).map(|s| format!("{} {}", fact_catalog.names[s], presence_only_fills[s])).collect();
        println!("spec: files filled only for a presence test, by relation: {}", po.join(", "));
        if way == "entry" {
            println!("entry {entry_mode}: candidates {} survivors {} rule-files-with-survivors {}", rule_cands.iter().sum::<u64>(), rule_surv.iter().sum::<u64>(), hits.len());
        }
    }
    if let Some(path) = flag(args, "--why-out") {
        std::fs::write(path, &why_lines).expect("why out");
    }
    if let Some(path) = flag(args, "--pairs-out") {
        std::fs::write(path, &pairs_lines).expect("pairs out");
    }
    if way == "entry" {
        if let Some(path) = flag(args, "--rules-out") {
            let mut w = String::from("rule\tunits\tcandidates\tsurvivors\tsteps\n");
            for (ri, r) in frules.iter().enumerate() {
                w.push_str(&format!("{}\t{}\t{}\t{}\t0\n", r.name, srules[ri].groups.len(), rule_cands[ri], rule_surv[ri]));
            }
            std::fs::write(path, w).expect("rules out");
        }
    } else if let Some(path) = flag(args, "--rules-out") {
        let mut w = String::from("rule\tunits\tcandidates\tsurvivors\tsteps\n");
        for (ri, r) in frules.iter().enumerate() {
            let mine: Vec<usize> = (0..units.len()).filter(|u| units[*u].rule == ri).collect();
            let c: u64 = mine.iter().map(|u| cands[*u]).sum();
            let s: u64 = mine.iter().map(|u| surv[*u]).sum();
            let k: u64 = mine.iter().map(|u| unit_steps[*u]).sum();
            w.push_str(&format!("{}\t{}\t{c}\t{s}\t{k}\n", r.name, mine.len()));
        }
        std::fs::write(path, w).expect("rules out");
    }
    if let Some(path) = flag(args, "--alive-out") {
        let mut w = String::new();
        for (ri, f) in &alive_pairs {
            w.push_str(&format!("{}\t{}\n", frules[*ri].name, list[*f]));
        }
        std::fs::write(path, w).expect("alive out");
    }
    if let Some(path) = flag(args, "--hits-out") {
        let mut w = String::new();
        for (ri, f) in &hits {
            w.push_str(&format!("{}\t{}\n", frules[*ri].name, list[*f]));
        }
        std::fs::write(path, w).expect("hits out");
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
