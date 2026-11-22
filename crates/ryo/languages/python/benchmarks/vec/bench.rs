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

/// redo-union: the step clock. The thread's CPU time is a system call (157 ns a read here, two a timed step: about
/// 2.3 s of django's 2.4 s system time); the monotonic clock is read in user space (19 ns). Runs are single thread pinned
/// to one core under the timing lock, so the two agree; `VEC_THREAD_CLOCK` restores the CPU-time clock.
fn thread_cpu() -> f64 {
    thread_local! {
        static ORIGIN: cpu_time::ThreadTime = cpu_time::ThreadTime::now();
        static WALL: std::time::Instant = std::time::Instant::now();
    }
    static CPU: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *CPU.get_or_init(|| std::env::var_os("VEC_THREAD_CLOCK").is_some()) {
        return ORIGIN.with(|origin| origin.elapsed().as_secs_f64());
    }
    WALL.with(|origin| origin.elapsed().as_secs_f64())
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

/// One pass: the fields present in a file, read off the table's field lists (a step a field code).
fn fields_present(table: &NodeTable, cols: &mut Columns, fields: usize) {
    cols.field_present.clear();
    cols.field_present.resize(words(fields), 0);
    for f in 0..fields {
        if !node_table::ids_of_field(table, languages::FieldCode(f as u16)).is_empty() {
            cols.field_present[f / 64] |= 1 << (f % 64);
        }
    }
}

/// An element's bytes, from the node table's own span column.
fn span_bytes(table: &NodeTable, i: usize) -> std::ops::Range<usize> {
    node_table::starts(table)[i] as usize..node_table::ends(table)[i] as usize
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

/// The kinds-and-fields skip of a run's plan rules, flat: per rule whether it never matches and its clauses, a clause its
/// kind words then its field words; a rule with no anchor line always passes.
struct Skip {
    kw: usize,
    fw: usize,
    masks: Vec<u64>,
    rules: Vec<(bool, std::ops::Range<usize>)>,
}

impl Skip {
    fn new(rules: &[Rule], skip_of: &[Option<usize>], kw: usize, fw: usize) -> Self {
        let mut masks = Vec::new();
        let rules = skip_of
            .iter()
            .map(|i| {
                let a = masks.len();
                if let Some(i) = i {
                    for (k, f) in &rules[*i].clauses {
                        masks.extend_from_slice(k);
                        masks.extend_from_slice(f);
                    }
                }
                (i.is_some_and(|i| rules[i].never), a..masks.len())
            })
            .collect();
        Skip { kw, fw, masks, rules }
    }

    /// Whether every clause of the rule meets a kind or a field the file has (`survives` over the flat clauses).
    #[inline]
    fn alive(&self, rule: usize, kinds: &[u64], fields: &[u64]) -> bool {
        let (never, range) = &self.rules[rule];
        !never
            && self.masks[range.clone()].chunks_exact(self.kw + self.fw).all(|c| {
                c[..self.kw].iter().zip(kinds).any(|(a, b)| a & b != 0) || c[self.kw..].iter().zip(fields).any(|(a, b)| a & b != 0)
            })
    }
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
    // 6f: `VEC_PY310` walks at the pack's configured target (benchmark.toml `python-target=3.10`); without it the walk is at
    // the parser's default (Latest), as every run before 6f was.
    let walk = ryo_python::walk_with_settings(target_settings());
    let vocab = vocabulary(&walk);
    match args.get(1).copied() {
        Some("census") => census(&walk, &vocab, args[2], args[3], args[4]),
        Some("run") => run(&walk, &vocab, &args[2..]),
        Some("funnel") => {
            // with `--finish <plan> <root> [--findings-out F]`, the second half runs in this process on the first half's
            // survivors, file by file inside its walk (the seed of each file as `fileseed.py` makes it: `Survivors::seed_of_file`)
            if let Some(at) = args.iter().position(|a| *a == "--finish") {
                // pass 2: the second half runs file by file inside the first half's walk, on the same parsed file (one
                // parse a file)
                let mut finish_args = vec![args[at + 1], args[3], args[at + 2]];
                if let Some(at) = args.iter().position(|a| *a == "--findings-out") {
                    finish_args.extend(["--out", args[at + 1]]);
                }
                // redo-int: `--finish-stats F` writes the second half's per-step stats (its `--stats`)
                if let Some(at) = args.iter().position(|a| *a == "--finish-stats") {
                    finish_args.extend(["--stats", args[at + 1]]);
                }
                let plan = finish::load(finish_args[0]);
                let mut fin = finisher::Finisher::new(&plan, &walk, &vocab, &finish_args);
                run_funnel(&walk, &vocab, &args[2..], Some(&mut fin));
                fin.end(0.0);
            } else {
                run_funnel(&walk, &vocab, &args[2..], None);
            }
        }
        Some("finish") => run_finish(&walk, &vocab, &args[2..]),
        Some("kfloor") => kernel_floor(&walk, args[2]),
        _ => eprintln!("usage: vec census <files> <literals> <out> | vec run <mode> <files> <anchors> [--batch N] [--repeat K] [--rules-out F]"),
    }
}

fn target_settings() -> ryo_python::settings::PythonParserSettings {
    if std::env::var_os("VEC_PY310").is_some() {
        ryo_python::settings::PythonParserSettings::new(ryo_python::settings::PythonTargetVersion::Py310)
    } else {
        ryo_python::settings::PythonParserSettings::default()
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

#[path = "funnel.rs"]
mod funnel;
#[path = "facts.rs"]
mod facts;
#[path = "survivors.rs"]
mod survivors;
#[path = "finish.rs"]
mod finish;
#[path = "finisher.rs"]
mod finisher;
#[path = "dump.rs"]
mod dump;

/// The visit's facts as the finish's provider.
struct Facts<'v, 'f, 'w> {
    facts: &'v mut FileFacts<'f>,
    words: &'w mut languages::FileWords,
    filled: languages::FilledColumns,
    time: f64,
    rows: u64,
    by_rel: HashMap<String, (f64, u64, u64)>,
    file: u32,
}

/// 6f probe: the visit's facts through the direct-columns walk: the reader's fills, the provider's render, and the
/// evaluation region (`ColumnFile::evaluation_region`).
struct ColFacts<'v, 'a, 'f, 'w> {
    file: &'v mut ryo_python::fixtures::columns::ColumnFile<'a, 'f>,
    words: &'w mut languages::FileWords,
    filled: languages::FilledColumns,
    time: f64,
    rows: u64,
    by_rel: HashMap<String, (f64, u64, u64)>,
    file_no: u32,
}

impl ColFacts<'_, '_, '_, '_> {
    /// A catalog relation of this file, filled whole.
    fn whole(&mut self, run: &mut finish::Run, rel: &str) -> Result<finish::Table, String> {
        let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
        self.filled.0.clear();
        match self.file.fill(relation, languages::Wanted::Whole, self.words, &mut self.filled) {
            languages::FactAnswer::Answered(_) => finish::filled_table(run, rel, &self.filled, self.words),
            _ => Err(format!("{rel} not answered")),
        }
    }

    /// redo-binding probe: `read-resolution` from the provider's own lookup (`binding-resolution`, whose rows come per read
    /// nearest namespace first): the first row of each read, a deletion's only at depth 0 (its name is local), and a `none`
    /// row for every read or deletion (`binding-read`) no namespace of whose path holds an event of the name.
    fn read_resolution(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
        let res = self.whole(run, "binding-resolution")?;
        let reads = self.whole(run, "binding-read")?;
        let col = |t: &finish::Table, n: &str| t.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"));
        let (r_ref, r_ev, r_kind, r_scope, r_lex, r_depth, r_ord, r_ctx) = (
            col(&res, "reference")?,
            col(&res, "event")?,
            col(&res, "kind")?,
            col(&res, "scope")?,
            col(&res, "lexical")?,
            col(&res, "depth")?,
            col(&res, "ordinal")?,
            col(&res, "context")?,
        );
        let (b_ref, b_scope, b_ctx) = (col(&reads, "reference")?, col(&reads, "scope")?, col(&reads, "context")?);
        let (_, names, tys) = run.catalog(finish::RESOLUTION_REL).ok_or("no read-resolution")?;
        let deletion = u64::from(run.sym(b"deletion"));
        let (own, routed, none) = (u64::from(run.sym(b"own")), u64::from(run.sym(b"routed")), u64::from(run.sym(b"none")));
        let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(reads.n); 9], n: 0 };
        let mut seen: rustc_hash::FxHashSet<u64> = rustc_hash::FxHashSet::with_capacity_and_hasher(reads.n, Default::default());
        for r in 0..res.n {
            let reference = res.cols[r_ref][r];
            let depth = res.cols[r_depth][r];
            if (res.cols[r_ctx][r] == deletion && depth != 0) || !seen.insert(reference) {
                continue;
            }
            let ns = if res.cols[r_lex][r] == res.cols[r_scope][r] { own } else { routed };
            for (c, v) in [reference, res.cols[r_ev][r], res.cols[r_kind][r], ns, res.cols[r_scope][r], depth, res.cols[r_ord][r], res.cols[r_ctx][r], 0].into_iter().enumerate() {
                t.cols[c].push(v);
            }
            t.n += 1;
        }
        for r in 0..reads.n {
            let reference = reads.cols[b_ref][r];
            if !seen.insert(reference) {
                continue;
            }
            for (c, v) in [reference, finish::NULL, none, none, reads.cols[b_scope][r], finish::NULL, finish::NULL, reads.cols[b_ctx][r], 0].into_iter().enumerate() {
                t.cols[c].push(v);
            }
            t.n += 1;
        }
        Ok(t)
    }
}

impl finish::Provider for ColFacts<'_, '_, '_, '_> {
    fn fill(&mut self, run: &mut finish::Run, rel: &str) -> Result<finish::Table, String> {
        let t0 = thread_cpu();
        let mut t = if rel == finish::REGION_REL {
            let (deferred, rows) = self.file.evaluation_region().ok_or("evaluation-region: the model is incomplete")?;
            let (_, names, tys) = run.catalog(rel).ok_or("no evaluation-region")?;
            let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(rows.len()); 6], n: rows.len() };
            let timing = u64::from(run.sym(if deferred { b"deferred" } else { b"immediate" }));
            for r in &rows {
                let kind = u64::from(run.sym(r.kind.as_bytes()));
                t.cols[0].push(u64::from(r.id));
                t.cols[1].push(u64::from(r.region));
                t.cols[2].push(kind);
                t.cols[3].push(if r.annotation == u32::MAX { finish::NULL } else { u64::from(r.annotation) });
                t.cols[4].push(timing);
                t.cols[5].push(0);
            }
            t
        } else if rel == finish::RESOLUTION_REL {
            self.read_resolution(run)?
        } else {
            let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
            self.filled.0.clear();
            let answer = self.file.fill(relation, languages::Wanted::Whole, self.words, &mut self.filled);
            match answer {
                languages::FactAnswer::Answered(_) => finish::filled_table(run, rel, &self.filled, self.words),
                _ => Err(format!("{rel} not answered")),
            }?
        };
        for c in 0..t.tys.len() {
            if t.tys[c] == finish::Ty::File {
                t.cols[c].iter_mut().for_each(|v| *v = u64::from(self.file_no));
            }
        }
        let dt = thread_cpu() - t0;
        self.time += dt;
        self.rows += t.n as u64;
        let e = self.by_rel.entry(rel.to_owned()).or_default();
        e.0 += dt;
        e.1 += t.n as u64;
        e.2 += 1;
        Ok(t)
    }

    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        let context = match context {
            "assignment-target" => languages::RenderContext::AssignmentTarget,
            "attribute-receiver" => languages::RenderContext::AttributeReceiver,
            "positional-argument" => languages::RenderContext::PositionalArgument,
            other => return Err(format!("render context {other}")),
        };
        match self.file.render(languages::RenderRequest { node: languages::NodeId(u64::from(node)), context }) {
            languages::RenderOutcome::Known(t) => Ok(Some(t.0.as_bytes().to_vec())),
            languages::RenderOutcome::NotApplicable => Ok(None),
            languages::RenderOutcome::Unsupported(_) => Err("render unsupported".to_owned()),
        }
    }
}

impl finish::Provider for Facts<'_, '_, '_> {
    fn fill(&mut self, run: &mut finish::Run, rel: &str) -> Result<finish::Table, String> {
        if rel == finish::REGION_REL {
            return Err("evaluation-region needs VEC_COLWALK".to_owned());
        }
        let t0 = thread_cpu();
        let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
        self.filled.0.clear();
        let answer = self.facts.facts.fill(relation, languages::Wanted::Whole, self.words, &mut self.filled);
        let mut t = match answer {
            languages::FactAnswer::Answered(_) => finish::filled_table(run, rel, &self.filled, self.words),
            _ => Err(format!("{rel} not answered")),
        }?;
        for c in 0..t.tys.len() {
            if t.tys[c] == finish::Ty::File {
                t.cols[c].iter_mut().for_each(|v| *v = u64::from(self.file));
            }
        }
        let dt = thread_cpu() - t0;
        self.time += dt;
        self.rows += t.n as u64;
        let e = self.by_rel.entry(rel.to_owned()).or_default();
        e.0 += dt;
        e.1 += t.n as u64;
        e.2 += 1;
        Ok(t)
    }

    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        let context = match context {
            "assignment-target" => languages::RenderContext::AssignmentTarget,
            "attribute-receiver" => languages::RenderContext::AttributeReceiver,
            "positional-argument" => languages::RenderContext::PositionalArgument,
            other => return Err(format!("render context {other}")),
        };
        match self.facts.facts.render(languages::RenderRequest { node: languages::NodeId(u64::from(node)), context }) {
            languages::RenderOutcome::Known(t) => Ok(Some(t.0.as_bytes().to_vec())),
            languages::RenderOutcome::NotApplicable => Ok(None),
            languages::RenderOutcome::Unsupported(_) => Err("render unsupported".to_owned()),
        }
    }
}

/// `vec kfloor <files>`: the floor of the finish's batch steps per row on Django's real node tables (each file's own
/// columns, cache-resident as a step would see them): gather, filter, position read, hash build and probe, set knock-out,
/// and a next-token walk. Each kernel runs over every file; ns per row-op, thread clock, median of 5.
fn kernel_floor(walk: &impl Walk, list: &str) {
    let list = files(list);
    let mut tables: Vec<(Vec<u64>, Vec<u32>, Vec<u32>)> = Vec::new();
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    for path in &list {
        let bytes = std::fs::read(path).expect("read");
        let mut builder = node_table::builder(&mut table, walk_kinds(walk));
        let _ = walk.walk(WalkRequest { path: languages::SourcePath(path), file: FileId(0), source: ByteSlice(&bytes) }, RelationNeeds(&[]), &mut builder, &mut scratch, &mut NoVisit);
        let t = node_table::finished(&mut builder);
        let n = node_table::element_count(t).0 as u32;
        let kind: Vec<u64> = (0..n).map(|i| u64::from(node_table::kind_of(t, Position(i)).0)).collect();
        let parent: Vec<u32> = (0..n).map(|i| node_table::parent_of(t, Position(i)).0).collect();
        let ids: Vec<u32> = (0..n).collect();
        tables.push((kind, parent, ids));
    }
    let rows: usize = tables.iter().map(|t| t.0.len()).sum();
    let time = |name: &str, ops: usize, f: &mut dyn FnMut() -> u64| {
        let mut v = Vec::new();
        let mut sink = 0;
        for _ in 0..5 {
            let a = thread_cpu();
            sink ^= f();
            v.push(thread_cpu() - a);
        }
        v.sort_by(f64::total_cmp);
        println!("{name}: {:.2} ns a row-op ({ops} row-ops, {:.4} s) [{sink}]", v[2] * 1e9 / ops as f64, v[2]);
    };
    let mut sel: Vec<u32> = Vec::new();
    let mut out: Vec<u64> = Vec::new();
    time("filter (two != tests on a column, selection out)", rows, &mut || {
        let mut c = 0;
        for (kind, _, _) in &tables {
            sel.clear();
            sel.extend((0..kind.len() as u32).filter(|i| kind[*i as usize] != 7 && kind[*i as usize] != 9));
            c += sel.len() as u64;
        }
        c
    });
    time("gather (a column by a selection)", rows, &mut || {
        let mut c = 0;
        for (kind, _, ids) in &tables {
            out.clear();
            out.extend(ids.iter().map(|i| kind[*i as usize]));
            c += out.len() as u64;
        }
        c
    });
    time("position read join (parent of each row, then its kind)", rows, &mut || {
        let mut c = 0u64;
        for (kind, parent, ids) in &tables {
            out.clear();
            out.extend(ids.iter().map(|i| kind[parent[*i as usize] as usize]));
            c += out.len() as u64;
        }
        c
    });
    time("hash build (one key a row) + probe (one a row)", 2 * rows, &mut || {
        let mut c = 0u64;
        for (kind, parent, _) in &tables {
            let mut m: rustc_hash::FxHashMap<u64, u32> = rustc_hash::FxHashMap::default();
            m.reserve(kind.len());
            for (i, p) in parent.iter().enumerate() {
                m.insert(u64::from(*p) << 8 | kind[i], i as u32);
            }
            for (i, p) in parent.iter().enumerate() {
                c += u64::from(m.get(&(u64::from(*p) << 8 | kind[i])).is_some());
            }
        }
        c
    });
    time("set knock-out (antijoin: build a set of half, test every row)", rows + rows / 2, &mut || {
        let mut c = 0u64;
        for (_, parent, ids) in &tables {
            let set: rustc_hash::FxHashSet<u32> = parent.iter().step_by(2).copied().collect();
            c += ids.iter().filter(|i| !set.contains(i)).count() as u64;
        }
        c
    });
    time("chain walk (nearest next row of a kind set, per row)", rows, &mut || {
        let mut c = 0u64;
        for (kind, _, _) in &tables {
            let n = kind.len();
            let mut next = vec![u32::MAX; n];
            let mut last = u32::MAX;
            for i in (0..n).rev() {
                next[i] = last;
                if kind[i] != 7 {
                    last = i as u32;
                }
            }
            c += next.iter().filter(|x| **x != u32::MAX).count() as u64;
        }
        c
    });
}

/// `vec finish <plan.json> <files> <root> [--rules A,B] [--seed rule-file.tsv] [--out findings.jsonl] [--stats stats.tsv]
/// [--check-bases]`: each rule's whole lowered query run per file (`finish.rs`); findings as the reference writes them.
fn run_finish(walk: &impl Walk, vocab: &Vocab, args: &[&str]) {
    let plan = finish::load(args[0]);
    let list = files(args[1]);
    let mut fin = finisher::Finisher::new(&plan, walk, vocab, args);
    let seed: Option<std::collections::HashSet<(String, String)>> =
        flag(args, "--seed").map(|p| std::fs::read_to_string(p).expect("seed").lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_owned(), b.to_owned())).collect());
    let mut seed_by_path: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for (r, f) in seed.iter().flatten() {
        seed_by_path.entry(f.as_str()).or_default().insert(r.as_str());
    }
    // 6f probe: `VEC_COLWALK` walks through the direct-columns walk (the provider's own model is reachable there, for the
    // evaluation region); `--dump F` writes every emit's rows instead of findings.
    let colwalk = std::env::var_os("VEC_COLWALK").is_some();
    let mut cwalk = ryo_python::fixtures::columns::ColumnWalk::new(ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names"));
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let mut walk_time = 0f64;
    let mut rss_peak = 0u64;
    for (file_no, path) in list.iter().enumerate() {
        let w0 = thread_cpu();
        let in_fin = fin.time;
        let bytes = std::fs::read(path).expect("read");
        // 7: the (rule, file) seeds read by file once, not a key copied per rule
        let seed_here = seed_by_path.get(path.as_str());
        let wanted = fin.wanted(path, &|e| seed.is_none() || seed_here.is_some_and(|s| s.contains(plan.emits[e].0.as_str())));
        if !fin.visits(&wanted) {
            continue;
        }
        if colwalk {
            let _ = cwalk.walked(path, &bytes, &mut table, &mut |file| fin.column_file(file_no, path, file, &bytes, &wanted));
        } else {
            let _ = ryo_python::fixtures::walking::walked(walk, path, &bytes, &[], &mut table, &mut scratch, &mut |mut file| fin.reader_file(file_no, path, &mut file, &bytes, &wanted));
        }
        walk_time += thread_cpu() - w0 - (fin.time - in_fin);
        // 7: `VEC_RSS_TRACE`: the files after which the process's resident peak rose by 32 MiB or more
        if std::env::var_os("VEC_RSS_TRACE").is_some() {
            let hwm = std::fs::read_to_string("/proc/self/status").ok().and_then(|st| st.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())).unwrap_or(0);
            if hwm >= rss_peak + 32 * 1024 {
                eprintln!("rss peak {} MiB after {}", hwm / 1024, path);
                rss_peak = hwm;
            }
        }
    }
    fin.end(walk_time);
}

/// `vec funnel <way> <files> <anchors> <plans> [--repeat K] [--rules-out F] [--hits-out F]`: ways `cands`, `structure`,
/// `interp`, `spec`, `knock`.
fn run_funnel(walk: &impl Walk, vocab: &Vocab, args: &[&str], mut fin: Option<&mut finisher::Finisher<'_>>) {
    let way = args[0];
    let list = files(args[1]);
    let rules = load_rules(args[2], vocab);
    let mut fact_catalog = facts::Catalog::default();
    let mut interner = facts::Interner { ids: facts::Table::named(&flag(args, "--hasher").unwrap_or_else(|| "std".to_owned())), ..Default::default() };
    let (frules, words) = funnel::load(args[3], vocab, &mut fact_catalog, &mut interner);
    let use_facts = args.contains(&"--facts");
    let slots_n = fact_catalog.relations.len();
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
    // cols prototype: the provider writes the facts directly as per-node columns and keyed lists
    let direct = args.contains(&"--direct");
    let check_direct = args.contains(&"--check-direct");
    // ablations: every fact asked whole (no yes/no form); every fact asked in every file (no per-file decision)
    let no_yes_no = args.contains(&"--no-yes-no");
    let ask_all = args.contains(&"--ask-all");
    let no_project = args.contains(&"--no-project");
    let plant = std::env::var("COLS_PLANT").is_ok_and(|v| v == "1");
    let mut cwalk = ryo_python::fixtures::columns::ColumnWalk::new(ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names"));
    store.direct_on = direct;
    if args.contains(&"--regex-set") {
        interner.set = Some(regex::bytes::RegexSet::new(interner.regexes.iter().map(|r| r.as_str())).expect("regex set"));
    }
    if let Some(t) = token_slot {
        store.token_slot = Some((t, fact_catalog.column(t, "source"), fact_catalog.column(t, "target"), u64::from(interner.id(b"next-token"))));
    }
    let mut asks: Vec<ryo_python::fixtures::columns::Ask> = Vec::new();
    let mut ask_of: Vec<(usize, usize)> = Vec::new();
    let (mut direct_fill, mut direct_write, mut direct_files) = (0f64, 0f64, vec![0u64; slots_n]);
    let mut model_time = 0f64;
    let per_fact = args.contains(&"--per-fact");
    let mut fact_time = vec![0f64; slots_n];
    let (mut direct_rows, mut direct_exists_files) = (vec![0u64; slots_n], vec![0u64; slots_n]);
    // the check: rows compared and differing, by slot; yes/no answers compared and differing; answers differing; reference
    // cells that do not fit 32 bits
    let (mut chk_rows, mut chk_diff, mut chk_yes, mut chk_yes_diff, mut chk_answer_diff, mut chk_wide) =
        (vec![0u64; slots_n], vec![0u64; slots_n], vec![0u64; slots_n], vec![0u64; slots_n], vec![0u64; slots_n], vec![0u64; slots_n]);
    let (mut chk_multi, mut chk_shown) = (vec![0u64; slots_n], 0usize);
    let mut chk_cols: Vec<Vec<String>> = vec![Vec::new(); slots_n];
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
    let repeat: usize = flag(args, "--repeat").map_or(1, |b| b.parse().expect("repeat"));
    let by_name: HashMap<&str, usize> = rules.iter().enumerate().map(|(i, r)| (r.name.as_str(), i)).collect();
    let skip_of: Vec<Option<usize>> = frules.iter().map(|r| by_name.get(r.name.as_str()).copied()).collect();
    // the kinds-and-fields skip of each plan rule, flat: a clause is its kind words then its field words
    let skip = Skip::new(&rules, &skip_of, words_of(vocab.kind_names.len()), words_of(vocab.field_names.len()));
    let mut kinds: Vec<u64> = Vec::new();
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
    let structured = matches!(way, "structure" | "spec" | "knock");
    let max_regs = frules.iter().flat_map(|r| r.plans.iter().map(|p| p.regs)).max().unwrap_or(1).max(1);
    let mut cands = vec![0u64; units.len()];
    let mut surv = vec![0u64; units.len()];
    let mut unit_steps = vec![0u64; units.len()];
    let mut knock_steps = 0u64;
    let mut sv = survivors::Survivors::default();
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
    let (mut kinds_alive, mut words_alive, mut alive): (Vec<bool>, Vec<bool>, Vec<bool>) = (Vec::new(), Vec::new(), Vec::new());
    // one pass: `--dump <file>` writes per file a hash of each part the first half reads (dump.rs)
    let dump_out = flag(args, "--dump");
    let mut dumped = String::new();
    let mut reader_dump = String::new();
    let started = thread_cpu();
    // the second half inside this walk (`--finish`): each plan emit's rules of this plan set by name (the seed's rule is a
    // name), and the file's seed as a set over the rules
    assert!(fin.is_none() || (use_facts && direct), "--finish in the first half's walk needs --facts --direct");
    let emit_rules: Vec<Vec<usize>> = fin.as_ref().map(|f| f.emit_names().map(|(_, n)| frules.iter().enumerate().filter(|(_, r)| r.name == n).map(|(i, _)| i).collect()).collect()).unwrap_or_default();
    let mut in_seed = vec![false; frules.len()];
    let node_class: Vec<bool> = frules.iter().map(|r| r.node).collect();
    let fin0 = fin.as_ref().map_or(0.0, |f| f.time);
    for (file_no, path) in list.iter().enumerate() {
        let c0 = thread_cpu();
        let bytes = std::fs::read(path).expect("read");
        let (hits0, alive0) = (sv.hits.len(), sv.alive.len());
        store.reset(slots_n);
        reader_dump.clear();
        let mut visit_time = 0f64;
        // the file's skip, made once (inside the walk when the provider's facts are decided there, else after it): the
        // columns read, the required words present, and which rules can match by kinds and fields and by words
        let mut prepared = false;
        macro_rules! prepare {
            ($t:expr) => {{
                let pt: &NodeTable = $t;
                let c1 = thread_cpu();
                fields_present(pt, &mut cols, fields_width.max(1));
                phases.add("field buckets", thread_cpu() - c1);
                if way != "structure" {
                // the file's required words
                let cw = thread_cpu();
                if word_src != "none" {
                    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80;
                    let from_table = |present: &mut Vec<u64>, pairs: &mut Vec<(u16, u32)>| {
                        present.iter_mut().for_each(|w| *w = 0);
                        pairs.clear();
                        for p in node_table::ids_of_kind(pt, KindBits(name_token)) {
                            let i = p.0 as usize;
                            if let Some(w) = words.ids.get(&bytes[span_bytes(pt, i)]) {
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
                            for p in node_table::ids_of_kind(pt, KindBits(name_token)) {
                                let i = p.0 as usize;
                                let _ = set.insert(&bytes[span_bytes(pt, i)]);
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
                let c3 = thread_cpu();
                kinds.clear();
                kinds.extend(node_table::kinds_present(pt).0.iter().map(|w| w.0));
                kinds_alive.clear();
                kinds_alive.extend((0..frules.len()).map(|ri| skip.alive(ri, &kinds, &cols.field_present)));
                // the words only of a rule the kinds and fields keep
                words_alive.clear();
                words_alive.extend(frules.iter().zip(&kinds_alive).map(|(r, k)| *k && r.words_alive(&present)));
                alive.clear();
                alive.extend(kinds_alive.iter().zip(&words_alive).map(|(k, w)| *k && (!word_skip || *w)));
                rule_files += frules.len() as u64;
                alive_kinds += kinds_alive.iter().filter(|a| **a).count() as u64;
                alive_both += alive.iter().filter(|a| **a).count() as u64;
                for (ri, a) in alive.iter().enumerate() {
                    if *a {
                        sv.alive.push((ri as u32, file_no as u32));
                    }
                }
                phases.add("skip tests", thread_cpu() - c3);
                }
            }};
        }
        // the file's prefix: the structure pass, then each live unit's plans over its bucket (inside the fact visit when the
        // walk lends the file there, so the second half can run on the same parsed file after it)
        let mut prefixed = false;
        macro_rules! prefix {
            ($t:expr) => {{
                let t: &NodeTable = $t;
                let c2 = thread_cpu();
                if structured {
                    for _ in 0..repeat {
                        funnel::structure(t, &mut st, &function, &class, &looping);
                    }
                    phases.add("structure pass", (thread_cpu() - c2) / repeat as f64);
                    if !fact_catalog.enc_sets.is_empty() {
                        let e0 = thread_cpu();
                        funnel::enclosing(t, &mut st, &fact_catalog.enc_sets);
                        phases.add("enclosing columns (one per kind set)", thread_cpu() - e0);
                    }
                }
                if dump_out.is_some() {
                    dump::table(&mut dumped, file_no, t, vocab.kind_names.len());
                    let mut h = dump::Hashed::default();
                    h.rows(node_table::parent_ids(t));
                    h.rows(&st.pkind);
                    h.rows(&st.cmask);
                    h.rows(&st.sibling);
                    h.rows(&st.enc_function);
                    h.rows(&st.enc_class);
                    h.rows(&st.enc_loop);
                    h.rows(&st.enc);
                    h.line(&mut dumped, file_no, "structure");
                    let mut h = dump::Hashed::default();
                    // the field buckets as the harness's counting sort laid them out: offsets, then the ids, then the present bits
                    let mut offsets = vec![0u32];
                    let mut ids: Vec<u32> = Vec::new();
                    for f in 0..fields_width.max(1) {
                        ids.extend(node_table::ids_of_field(t, languages::FieldCode(f as u16)).iter().map(|p| p.0));
                        offsets.push(ids.len() as u32);
                    }
                    h.rows(&offsets);
                    h.rows(&ids);
                    h.rows(&cols.field_present);
                    h.line(&mut dumped, file_no, "fields");
                    let mut h = dump::Hashed::default();
                    h.rows(&present);
                    h.rows(&word_pairs);
                    h.rows(&word_offsets);
                    h.rows(&word_pos);
                    h.line(&mut dumped, file_no, "words");
                    for (i, (slot, kc)) in ask_of.iter().enumerate() {
                        let mut h = dump::Hashed::default();
                        h.row(&asks[i]);
                        h.row(&store.direct.tables[i]);
                        h.line(&mut dumped, file_no, &format!("fact/{}/{kc}/{}", fact_catalog.names[*slot], asks[i].exists));
                    }
                    dumped.push_str(&reader_dump);
                }
                if way != "structure" && way != "words" {
                    let ctx = funnel::Ctx {
                        table: t,
                        src: &bytes,
                        pkind: if structured { &st.pkind } else { &[] },
                        cmask: if structured { &st.cmask } else { &[] },
                        cfmask: if structured { &st.cfmask } else { &[] },
                        facts: &store,
                        enc: &st.enc,
                        matches: &interner.matches,
                    };
                    let elements = node_table::element_count(t).0 as usize;
                    if marked.len() < elements {
                        marked.resize(elements, 0);
                    }
                    let fill = |key: &Key, buf: &mut Vec<u32>| {
                        buf.clear();
                        match *key {
                            Key::Word(w) => buf.extend_from_slice(&word_pos[word_offsets[usize::from(w)] as usize..word_offsets[usize::from(w) + 1] as usize]),
                            Key::Kind(k) => buf.extend(node_table::ids_of_kind(t, KindBits(k)).iter().map(|p| p.0)),
                            Key::Field(f) => buf.extend(node_table::ids_of_field(t, languages::FieldCode(f)).iter().map(|p| p.0)),
                        }
                    };
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
                                    // each plan runs over the candidates no earlier plan of the unit kept: the first from the
                                    // bucket itself, the next ones from what is left (made only when a later plan reads it)
                                    stamp = stamp.wrapping_add(1);
                                    let mut left = false;
                                    for (pi, p) in unit.plans.iter().enumerate() {
                                        let from: &[u32] = if left { &remaining } else { &buf };
                                        if from.is_empty() {
                                            break;
                                        }
                                        let plan = &rule.plans[*p];
                                        if word_skip && !funnel::cnf_holds(&plan.words, &present) {
                                            continue;
                                        }
                                        rows.enter(plan.regs.max(1), from);
                                        for stage in &plan.stages {
                                            if rows.len() == 0 {
                                                break;
                                            }
                                            stage(&ctx, &mut rows, &mut s);
                                        }
                                        let mut kept = 0;
                                        for j in 0..rows.len() {
                                            let n = rows.d[j * rows.r] as usize;
                                            if marked[n] != stamp {
                                                marked[n] = stamp;
                                                kept += 1;
                                                if first {
                                                    sv.nodes.push(n as u32);
                                                }
                                            }
                                        }
                                        n_s += kept;
                                        if pi + 1 < unit.plans.len() && kept > 0 {
                                            if left {
                                                remaining.retain(|n| marked[*n as usize] != stamp);
                                            } else {
                                                remaining.clear();
                                                remaining.extend(buf.iter().filter(|n| marked[**n as usize] != stamp));
                                                left = true;
                                            }
                                        }
                                    }
                                }
                            }
                            if first {
                                sv.close(unit.rule, file_no);
                            }
                            if first {
                                cands[ui] += buf.len() as u64;
                                file_surv[ui] += n_s;
                                unit_steps[ui] += s;
                            }
                        }
                        if first {
                            let mut any = vec![false; frules.len()];
                            for (ui, unit) in units.iter().enumerate() {
                                surv[ui] += file_surv[ui];
                                if file_surv[ui] > 0 {
                                    any[unit.rule] = true;
                                }
                            }
                            for (ri, a) in any.iter().enumerate() {
                                if *a {
                                    sv.hits.push((ri as u32, file_no as u32));
                                }
                            }
                        }
                    }
                    phases.add("prefixes", (thread_cpu() - c4) / repeat as f64);
                }
            }};
        }
        let ok = if use_facts && direct {
            cwalk
                .walked(path, &bytes, &mut table, &mut |file| {
                    let p0 = thread_cpu();
                    let ft = file.table;
                    let n = node_table::element_count(ft).0;
                    prepare!(ft);
                    prepared = true;
                    let v0 = thread_cpu();
                    let mut needed = vec![ask_all; slots_n];
                    let mut valued = vec![ask_all; slots_n];
                    let mut key_needed: Vec<(usize, usize)> = Vec::new();
                    for (ri, r) in frules.iter().enumerate() {
                        if r.slots.is_empty() || !kinds_alive[ri] || !words_alive[ri] {
                            continue;
                        }
                        for s in &r.slots {
                            needed[*s] = true;
                        }
                        for s in &r.valued {
                            valued[*s] = true;
                        }
                        for k in &r.keyed {
                            if !key_needed.contains(k) {
                                key_needed.push(*k);
                            }
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
                    let n = n as usize;
                    asks.clear();
                    ask_of.clear();
                    for slot in 0..slots_n {
                        if !needed[slot] || Some(slot) == token_slot {
                            continue;
                        }
                        let exists = !valued[slot] && !check_direct && !no_yes_no;
                        if exists {
                            direct_exists_files[slot] += 1;
                        }
                        direct_files[slot] += 1;
                        let full = check_direct || no_project || ask_all;
                        let columns = if full { u64::MAX } else { fact_catalog.read_columns[slot] };
                        for &kc in &fact_catalog.key_columns[slot] {
                            if !full && !key_needed.contains(&(slot, kc)) {
                                continue;
                            }
                            asks.push(ryo_python::fixtures::columns::Ask { relation: fact_catalog.relations[slot], key: kc, exists, columns });
                            ask_of.push((slot, kc));
                            if check_direct {
                                asks.push(ryo_python::fixtures::columns::Ask { relation: fact_catalog.relations[slot], key: kc, exists: true, columns });
                                ask_of.push((slot, kc));
                            }
                        }
                    }
                    let m = thread_cpu();
                    file.prepare(&asks);
                    let a = thread_cpu();
                    model_time += a - m;
                    if per_fact {
                        store.direct.tables.resize_with(asks.len().max(store.direct.tables.len()), Default::default);
                        for (i, ask) in asks.iter().enumerate() {
                            let c = thread_cpu();
                            file.column(ask, &mut interner, &mut store.direct.tables[i]);
                            fact_time[ask_of[i].0] += thread_cpu() - c;
                        }
                    } else {
                        file.columns(&asks, &mut interner, &mut store.direct);
                    }
                    let b = thread_cpu();
                    direct_fill += b - a;
                    for (i, (slot, kc)) in ask_of.iter().enumerate() {
                        if check_direct && asks[i].exists {
                            continue;
                        }
                        store.direct_of[*slot].push((*kc, i));
                        store.answered[*slot] = true;
                        let t = &store.direct.tables[i];
                        direct_rows[*slot] += t.rows as u64;
                        // the regex bits of the texts the plans test by regex, once a text
                        for &c in &fact_catalog.re_columns[*slot] {
                            let Some(col) = t.columns.get(c) else { continue };
                            if col.is_empty() {
                                continue;
                            }
                            match t.layout {
                                ryo_python::fixtures::columns::Layout::Keyed => {
                                    for v in col {
                                        interner.ensure_matched(*v);
                                    }
                                }
                                ryo_python::fixtures::columns::Layout::PerNode => {
                                    for (wi, w) in t.present.iter().enumerate() {
                                        let mut w = *w;
                                        while w != 0 {
                                            let k = wi * 64 + w.trailing_zeros() as usize;
                                            w &= w - 1;
                                            interner.ensure_matched(col[k]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    direct_write += thread_cpu() - b;
                    if let Some(t) = token_slot
                        && tokens_needed
                    {
                        let a = thread_cpu();
                        store.token_order(ft, &token_kinds, n);
                        store.answered[t] = true;
                        token_time += thread_cpu() - a;
                    }
                    if check_direct {
                        languages::facts::clear_words(&mut fwords);
                        let keyed_rows = |t: &ryo_python::fixtures::columns::FactTable, cols: &[usize]| -> Vec<Vec<u32>> {
                            let mut out = Vec::new();
                            for k in 0..n {
                                if !t.present.get(k / 64).is_some_and(|w| w >> (k % 64) & 1 == 1) {
                                    continue;
                                }
                                for r in t.rows_of(k) {
                                    let r = t.row(r);
                                    let mut row = vec![k as u32];
                                    row.extend(cols.iter().map(|c| t.columns[*c][r]));
                                    out.push(row);
                                }
                            }
                            out
                        };
                        let mut kept = std::mem::take(&mut store.direct);
                        // the plant (`COLS_PLANT=1`): one value of the first written column of each whole table is changed, and
                        // one yes/no bit dropped; the check must count them
                        if plant {
                            for (i, t) in kept.tables.iter_mut().enumerate().take(asks.len()) {
                                if asks[i].exists {
                                    if let Some(w) = t.present.iter_mut().find(|w| **w != 0) {
                                        *w &= *w - 1;
                                    }
                                    continue;
                                }
                                let first = t.present.iter().enumerate().find(|(_, w)| **w != 0).map(|(wi, w)| wi * 64 + w.trailing_zeros() as usize);
                                if let Some(k) = first {
                                    let r = t.rows_of(k).start;
                                    let r = t.row(r);
                                    if let Some(col) = t.columns.iter_mut().find(|c| c.len() > r) {
                                        col[r] ^= 1;
                                    }
                                }
                            }
                        }
                        let mut i = 0;
                        while i < asks.len() {
                            let (slot, kc) = ask_of[i];
                            let (whole, yes) = (&kept.tables[i], &kept.tables[i + 1]);
                            filled.0.clear();
                            let answer = file.fill(fact_catalog.relations[slot], languages::Wanted::Whole, &mut fwords, &mut filled);
                            let answered = matches!(answer, languages::FactAnswer::Answered(_));
                            if answered != whole.answered || answered != yes.answered {
                                chk_answer_diff[slot] += 1;
                            }
                            if answered && whole.answered {
                                store.write(&fact_catalog, slot, &filled, &fwords, &mut interner, n);
                                let cols: Vec<usize> = (0..whole.columns.len())
                                    .filter(|c| match whole.layout {
                                        ryo_python::fixtures::columns::Layout::PerNode => whole.columns[*c].len() == n && n > 0,
                                        ryo_python::fixtures::columns::Layout::Keyed => !whole.columns[*c].is_empty(),
                                    })
                                    .collect();
                                for c in &cols {
                                    let name = fact_catalog.column_name(slot, *c);
                                    if !chk_cols[slot].contains(&name) {
                                        chk_cols[slot].push(name);
                                    }
                                }
                                let mut mine = keyed_rows(whole, &cols);
                                let cells = &store.cells[slot];
                                let mut theirs: Vec<Vec<u32>> = (0..store.rows[slot])
                                    .map(|r| {
                                        let mut row = vec![cells[kc][r] as u32];
                                        for c in &cols {
                                            if cells[*c][r] >> 32 != 0 {
                                                chk_wide[slot] += 1;
                                            }
                                            row.push(cells[*c][r] as u32);
                                        }
                                        row
                                    })
                                    .collect();
                                mine.sort_unstable();
                                theirs.sort_unstable();
                                let mut keys: Vec<u32> = theirs.iter().map(|r| r[0]).collect();
                                let all = keys.len();
                                keys.dedup();
                                chk_multi[slot] += (all - keys.len()) as u64;
                                chk_rows[slot] += theirs.len() as u64;
                                let (mut x, mut y, mut d) = (0, 0, 0u64);
                                while x < mine.len() || y < theirs.len() {
                                    match (mine.get(x), theirs.get(y)) {
                                        (Some(a), Some(b)) if a == b => {
                                            x += 1;
                                            y += 1;
                                        }
                                        (Some(a), Some(b)) if a < b => {
                                            if chk_shown < 20 {
                                                chk_shown += 1;
                                                eprintln!("check {} {}: only direct {a:?} (cols {cols:?})", fact_catalog.names[slot], path);
                                            }
                                            d += 1;
                                            x += 1;
                                        }
                                        (Some(_), None) => {
                                            d += 1;
                                            x += 1;
                                        }
                                        (_, Some(b)) => {
                                            if chk_shown < 20 {
                                                chk_shown += 1;
                                                eprintln!("check {} {}: only reader {b:?} (cols {cols:?})", fact_catalog.names[slot], path);
                                            }
                                            d += 1;
                                            y += 1;
                                        }
                                        (None, None) => break,
                                    }
                                }
                                chk_diff[slot] += d;
                                // the yes/no answers: the nodes with a row
                                let yes_nodes: Vec<u32> = (0..n as u32).filter(|k| yes.present.get(*k as usize / 64).is_some_and(|w| w >> (k % 64) & 1 == 1)).collect();
                                chk_yes[slot] += keys.len() as u64;
                                let (mut x, mut y) = (0, 0);
                                while x < yes_nodes.len() || y < keys.len() {
                                    match (yes_nodes.get(x), keys.get(y)) {
                                        (Some(a), Some(b)) if a == b => {
                                            x += 1;
                                            y += 1;
                                        }
                                        (Some(a), Some(b)) if a < b => {
                                            chk_yes_diff[slot] += 1;
                                            x += 1;
                                        }
                                        (Some(_), None) => {
                                            chk_yes_diff[slot] += 1;
                                            x += 1;
                                        }
                                        (_, Some(_)) => {
                                            chk_yes_diff[slot] += 1;
                                            y += 1;
                                        }
                                        (None, None) => break,
                                    }
                                }
                            }
                            i += 2;
                        }
                        store.direct = kept;
                        // the token order against the provider's next-token rows
                        if let Some(t) = token_slot
                            && tokens_needed
                        {
                            filled.0.clear();
                            let _ = file.fill(fact_catalog.relations[t], languages::Wanted::Whole, &mut fwords, &mut filled);
                            store.write(&fact_catalog, t, &filled, &fwords, &mut interner, n);
                            let (sc, tc, kc) = (fact_catalog.column(t, "source"), fact_catalog.column(t, "target"), fact_catalog.column(t, "link-kind"));
                            let next_id = u64::from(interner.id(b"next-token"));
                            let cells = &store.cells[t];
                            let mut theirs: Vec<(u32, u32)> = (0..store.rows[t]).filter(|r| cells[kc][*r] == next_id).map(|r| (cells[sc][r] as u32, cells[tc][r] as u32)).collect();
                            let mut mine: Vec<(u32, u32)> = store.next_token.iter().enumerate().filter(|(_, v)| **v != u32::MAX).map(|(k, v)| (k as u32, *v)).collect();
                            let mut back: Vec<(u32, u32)> = store.prev_token.iter().enumerate().filter(|(_, v)| **v != u32::MAX).map(|(k, v)| (*v, k as u32)).collect();
                            theirs.sort_unstable();
                            mine.sort_unstable();
                            back.sort_unstable();
                            chk_rows[t] += theirs.len() as u64;
                            chk_diff[t] += u64::from(mine != theirs) * (mine.len().abs_diff(theirs.len()).max(1) as u64) + u64::from(back != theirs);
                        }
                    }
                    if dump_out.is_some() {
                        // the reader's rows of every relation the plans ask, whole
                        for slot in 0..slots_n {
                            languages::facts::clear_words(&mut fwords);
                            filled.0.clear();
                            let answer = file.fill(fact_catalog.relations[slot], languages::Wanted::Whole, &mut fwords, &mut filled);
                            let mut h = dump::Hashed::default();
                            h.row(&answer);
                            h.rows(&filled.0);
                            h.line(&mut reader_dump, file_no, &format!("reader/{}", fact_catalog.names[slot]));
                        }
                    }
                    // one parse a file: the prefix and then the second half on this walked file, before the walk drops it
                    prefix!(ft);
                    prefixed = true;
                    if let Some(fin) = fin.as_deref_mut() {
                        sv.seed_of_file(hits0, alive0, &node_class, &mut in_seed);
                        let wanted = fin.wanted(path, &|e| emit_rules[e].iter().any(|r| in_seed[*r]));
                        if fin.visits(&wanted) {
                            fin.column_file(file_no, path, file, &bytes, &wanted);
                        }
                    }
                    visit_time += thread_cpu() - p0;
                })
                .is_ok()
        } else if use_facts {
            // the provider's facts are lent only while the walk runs: decide here which files need them (some rule that
            // tests a fact survived the kinds-and-words skip), then fill those relations and write their columns
            ryo_python::fixtures::walking::walked(walk, path, &bytes, &[], &mut table, &mut scratch, &mut |file| {
                let p0 = thread_cpu();
                let ft = file.table;
                let n = node_table::element_count(ft).0;
                prepare!(ft);
                prepared = true;
                let v0 = thread_cpu();
                let mut needed = vec![false; slots_n];
                for (ri, r) in frules.iter().enumerate() {
                    if r.slots.is_empty() || !kinds_alive[ri] || !words_alive[ri] {
                        continue;
                    }
                    for s in &r.slots {
                        needed[*s] = true;
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
                visit_time += thread_cpu() - p0;
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
        if !prepared {
            prepare!(t);
        }
        if !prefixed {
            prefix!(t);
        }
    }
    // the second half's time inside this walk is not the first half's
    let fin_spent = fin.as_ref().map_or(0.0, |f| f.time) - fin0;
    let total = thread_cpu() - started - fin_spent;
    if let Some(path) = &dump_out {
        std::fs::write(path, &dumped).expect("dump out");
    }
    if fin.is_some() {
        println!("one walk: the second half inside the first half's visit {fin_spent:.4} s");
    }
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
        if direct {
            for slot in 0..slots_n {
                println!(
                    "direct {}: files {} (yes/no only {}) rows {} time {:.4} s check: rows {} differing {} keys with more than one row {} yes/no keys {} differing {} answers differing {} wide cells {} columns compared beside the key {:?}",
                    fact_catalog.names[slot], direct_files[slot], direct_exists_files[slot], direct_rows[slot], fact_time[slot], chk_rows[slot], chk_diff[slot], chk_multi[slot], chk_yes[slot], chk_yes_diff[slot], chk_answer_diff[slot], chk_wide[slot], chk_cols[slot]
                );
            }
            println!(
                "direct total: model build {model_time:.4} s provider columns {direct_fill:.4} s our writes (regex bits) {direct_write:.4} s token order {token_time:.4} s deciding {decide_time:.4} s; unwritten reads {}; check rows {} differing {}",
                store.unwritten.get(),
                chk_rows.iter().sum::<u64>(),
                chk_diff.iter().sum::<u64>() + chk_yes_diff.iter().sum::<u64>() + chk_answer_diff.iter().sum::<u64>()
            );
        }
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
        sv.hits.len()
    );
    if let Some(path) = flag(args, "--rules-out") {
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
    // the dumps for comparing, after the timed part
    let names: Vec<String> = frules.iter().map(|r| r.name.clone()).collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    if let Some(path) = flag(args, "--alive-out") {
        std::fs::write(path, survivors::Survivors::pairs_text(&sv.alive, &name_refs, &list)).expect("alive out");
    }
    if let Some(path) = flag(args, "--surv-out") {
        std::fs::write(path, sv.surv_text(&name_refs, &list)).expect("survivors out");
    }
    if let Some(path) = flag(args, "--hits-out") {
        std::fs::write(path, survivors::Survivors::pairs_text(&sv.hits, &name_refs, &list)).expect("hits out");
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
