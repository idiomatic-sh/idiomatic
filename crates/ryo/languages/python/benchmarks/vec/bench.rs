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
use std::rc::Rc;

/// (allocation calls, bytes allocated, -, live peak) so far, from the run's counter (`alloc_count`, a `vec-dhat` build);
/// zeros in any other.
fn alloc_counts() -> [usize; 4] {
    let c = alloc_count::counts();
    [c[0] as usize, c[1] as usize, 0, c[2] as usize]
}

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

use first::{Vocab, span_bytes};

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

/// The kinds-and-fields skip of each rule from the first half's spec prepared in the process (`first::prepare`): per rule its
/// name, whether it never matches, and its clauses of `k:<kind>` / `f:<field>` names. No anchors (the `run` modes read them).
fn rules_of_skips(skips: &[(String, bool, Vec<Vec<String>>)], vocab: &Vocab) -> Vec<Rule> {
    let mut missing = std::collections::BTreeSet::new();
    let (kw, fw) = (words(vocab.kind_names.len()), words(vocab.field_names.len()));
    let rules = skips
        .iter()
        .map(|(name, never, clauses)| {
            let clauses = clauses
                .iter()
                .map(|clause| {
                    let (mut k, mut f) = (vec![0u64; kw], vec![0u64; fw]);
                    for n in clause {
                        let (tag, name) = n.split_at(2);
                        let code = if tag == "k:" { vocab.kinds.get(name).map(|c| (&mut k, *c)) } else { vocab.fields.get(name).map(|c| (&mut f, *c)) };
                        match code {
                            Some((m, c)) => m[usize::from(c) / 64] |= 1 << (c % 64),
                            None => {
                                missing.insert(n.clone());
                            }
                        }
                    }
                    (k, f)
                })
                .collect();
            Rule { name: name.clone(), anchors: Vec::new(), clauses, never: *never, whole: None }
        })
        .collect();
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
#[path = "../../../../crates/engine/benches/vec/file_skip.rs"]
mod file_skip;
use file_skip::Skip;

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
    let _ = thread_cpu(); // Start the process-thread clock before reading the request.
    // redo-alloc: the allocation counter of a `vec-dhat` build (nothing in any other)
    let _counter = alloc_count::start();
    let args: Vec<String> = std::env::args().collect();
    // `cargo bench` passes `--bench`; drop it.
    let args: Vec<&str> = args.iter().map(String::as_str).filter(|a| *a != "--bench").collect();
    // ryo-fmx3.1 redo-lowering: the plan lowering in Rust, before any walk is made.
    match args.get(1).copied() {
        Some("lower") => return lower::run_lower(&args[2..]),
        Some("stage") => return lower::run_stage(&args[2..]),
        _ => {}
    }
    // 6f: `VEC_PY310` walks at the pack's configured target (benchmark.toml `python-target=3.10`); without it the walk is at
    // the parser's default (Latest), as every run before 6f was.
    let settings = target_settings();
    let walk = ryo_python::walk_with_settings(settings);
    let vocab = vocabulary(&walk);
    match args.get(1).copied() {
        Some("first") => first::run_first(&args[2..], &vocab, python_language),
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
                // redo-honest: `--finish-rules A,B` (the run's selected rules: only their findings are made and written) to
                // the second half's `--rules`
                if let Some(at) = args.iter().position(|a| *a == "--finish-rules") {
                    finish_args.extend(["--rules", args[at + 1]]);
                }
                // redo-honest: `--applicability F` (the request's rule_applicability) to the second half
                if let Some(at) = args.iter().position(|a| *a == "--applicability") {
                    finish_args.extend(["--applicability", args[at + 1]]);
                }
                // redo-general: `--request F` (the host's request: its project search roots) to the second half
                if let Some(at) = args.iter().position(|a| *a == "--request") {
                    finish_args.extend(["--request", args[at + 1]]);
                }
                // redo-lowering: `--finish <pack dir>` prepares the plan in this process from the rule text (the one parser,
                // the engine's lowering); `--finish <plan file>` reads a plan prepared beforehand (the oracle).
                let t0 = thread_cpu();
                type_expression::load_lists(finish_args[0]);
                let mut pack = None;
                let plan = if std::path::Path::new(finish_args[0]).is_dir() {
                    // `--request F`: the run's request, for the rule context the lowering reads
                    let request = args.iter().position(|a| *a == "--request").map(|i| args[i + 1]);
                    let read = lower::read_pack(std::path::Path::new(finish_args[0]), request::rule_context(request));
                    println!("start-up pack: read {:.4} parse {:.4} s", read.read, read.parse);
                    let same_pack = std::fs::canonicalize(args[4]).ok() == std::fs::canonicalize(finish_args[0]).ok();
                    let plan = lower::second_half_plan(&read, same_pack);
                    pack = Some((finish_args[0], read));
                    plan
                } else {
                    finish::load(finish_args[0])
                };
                println!("start-up total {:.4} s", thread_cpu() - t0);
                let mut fin = visit::Finisher::new(&plan, &walk, &vocab, &finish_args);
                run_funnel(&walk, settings, &vocab, &args[2..], Some(&mut fin), pack.as_ref().map(|(d, r)| (*d, r)));
                fin.end(0.0);
                // redo-union pass 2 c18: the plan (30 k nodes of JSON) is not dropped one by one at exit
                std::mem::forget(plan);
            } else {
                run_funnel(&walk, settings, &vocab, &args[2..], None, None);
            }
        }
        Some("finish") => run_finish(&walk, settings, &vocab, &args[2..]),
        Some("kfloor") => kernel_floor(&walk, args[2]),
        _ => eprintln!("usage: vec census <files> <literals> <out> | vec run <mode> <files> <anchors> [--batch N] [--repeat K] [--rules-out F]"),
    }
}

/// The first half's language knowledge, as the provider declares it (the column walk's per-node facts and the token order
/// this harness writes, the spelled columns, the identifier rule, the name-token kind).
fn python_language(vocab: &Vocab) -> first::Language {
    use finish::TokenOrder as _;
    let tokens=provider_tokens::Lexicon::new(&|n| vocab.kinds.get(n).copied());
    use ryo_python::fixtures::columns::{PER_NODE_FACTS, SPELLED_COLUMNS, Spelling, is_identifier};
    first::Language {
        host_len: finish::host::host_len,
        walk_facts: pair_provider::FACTS,
        text_kinds: vocab.kinds.values().copied().filter(|k|tokens.has_text(*k)).collect(),
        file_sets: ryo_python::fixtures::columns::FILE_SET_FACTS.iter().chain(host_columns::FILE_SETS.iter()).map(|s|(*s).to_string()).collect(),
        facts: PER_NODE_FACTS.iter().map(|s| (*s).to_owned()).collect(),
        spelled: SPELLED_COLUMNS.iter().map(|(r, c, how)| ((*r).to_owned(), (*c).to_owned(), match how { Spelling::Whole => None, Spelling::LastSegment(c) => Some(*c) })).collect(),
        is_identifier,
    }
}

/// The parser's settings from the run's request (`--request` or `--applicability`: its `parser_configuration`, the
/// provider's identity); with no request, `VEC_PY310` as before.
fn target_settings() -> ryo_python::settings::PythonParserSettings {
    let args: Vec<String> = std::env::args().collect();
    for option in ["--request", "--applicability"] {
        if let Some(path) = args.windows(2).find(|pair| pair[0] == option).map(|pair| &pair[1])
            && let Ok(text) = std::fs::read_to_string(path)
            && let Ok(request) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(identity) = request["parser_configuration"].as_str()
        {
            return ryo_python::settings::PythonParserSettings::from_identity(identity).unwrap_or_else(|| panic!("parser configuration {identity}"));
        }
    }
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

#[path = "../../../../crates/engine/benches/vec/alloc_count.rs"]
mod alloc_count;
#[path = "../../../../crates/engine/benches/vec/funnel.rs"]
mod funnel;
#[path = "../../../../crates/engine/benches/vec/spec.rs"]
mod spec;

#[path = "../../../../crates/engine/benches/vec/file_seeds.rs"]
mod file_seeds;
#[path = "../../../../crates/engine/benches/vec/facts.rs"]
mod facts;
#[path = "../../../../crates/engine/benches/vec/survivors.rs"]
mod survivors;
#[path = "../../../../crates/engine/benches/vec/finish.rs"]
mod finish;
#[path = "../../../../crates/engine/benches/vec/lower.rs"]
mod lower;
use request::rule_context;
#[path = "../../../../crates/engine/benches/vec/first.rs"]
mod first;
#[path = "../../../../crates/engine/benches/vec/pair_summary.rs"]
mod pair_summary;
#[path = "../../../../crates/engine/benches/vec/glob_pattern.rs"]
mod glob_pattern;
use glob_pattern::glob_pattern;
#[path = "../../../../crates/engine/benches/vec/pair_skip.rs"]
mod pair_skip;
#[path = "../../../../crates/engine/benches/vec/pair_checks.rs"]
mod pair_checks;
mod pair_provider;
#[path = "../../../../crates/engine/benches/vec/host_columns.rs"]
mod host_columns;
mod request;
mod completed_facts;
#[path = "../../../../crates/engine/benches/vec/finisher.rs"]
mod finisher;
mod visit;
#[path = "../../../../crates/engine/benches/vec/dump.rs"]
mod dump;
#[path = "type_expression.rs"]
mod type_expression;
#[path = "annotation_evaluation.rs"]
mod annotation_evaluation;
#[path = "deferred_visit.rs"]
mod deferred_visit;
#[path = "deferred_resolution.rs"]
mod deferred_resolution;
#[path = "direct_fill.rs"]
mod direct_fill;
#[path = "fact_catalog.rs"]
mod fact_catalog;
#[path = "composition_inputs.rs"]
mod composition_inputs;
#[path = "relative_import.rs"]
mod relative_import;
mod import_syntax;
#[path = "provider_packaging.rs"]
mod provider_packaging;
#[path = "provider_tokens.rs"]
mod provider_tokens;

/// The visit's facts as the finish's provider.
struct Facts<'v, 'f, 'w> {
    facts: &'v mut FileFacts<'f>,
    words: &'w mut languages::FileWords,
    filled: languages::FilledColumns,
    time: f64,
    rows: u64,
    by_rel: HashMap<String, (f64, u64, u64)>,
    file: u32,
    /// redo-general: the provider's project (its packaging knowledge, run-wide) and this file's bytes, for the relations it
    /// answers for a project.
    project: &'w mut provider_packaging::Project,
    src: &'w [u8],
}

/// 6f probe: the visit's facts through the direct-columns walk: the reader's fills, the provider's render, and the
/// evaluation region (`ColumnFile::evaluation_region`).
struct ColFacts<'v, 'a, 'f, 'w> {
    file: &'v mut ryo_python::fixtures::columns::ColumnFile<'a, 'f>,
    completed: &'w mut completed_facts::Completed,
    words: &'w mut languages::FileWords,
    filled: languages::FilledColumns,
    time: f64,
    rows: u64,
    by_rel: HashMap<String, (f64, u64, u64)>,
    file_no: u32,
    /// redo-binding pass 3: the provider relations the harness facts are written from, kept for the file once filled, so
    /// a fact and the steps that read the same relation share one fill.
    kept: composition_inputs::Tables,
    implicit: &'w mut Option<(u32, finish::Table)>,
    type_paths: &'w mut type_expression::Paths,
    input_layout: composition_inputs::Layout,
    input_columns: composition_inputs::Columns,
    prefix: Option<direct_fill::Prefix<'w>>,
    prefix_symbols: &'w mut Vec<u32>,
    direct_memo: &'w mut ryo_python::fixtures::columns::ColumnMemo,
    /// redo-general: the provider's project (its packaging knowledge, run-wide) and this file's bytes, for the relations it
    /// answers for a project.
    project: &'w mut provider_packaging::Project,
    src: &'w [u8],
}

enum Made {
    Owned(finish::Table),
    Shared(Rc<finish::Table>),
}

impl Made {
    fn table(&self) -> &finish::Table {
        match self { Self::Owned(t) => t, Self::Shared(t) => t }
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// A catalog relation of this file, filled whole (once a file for the relations the harness facts read).
    fn whole(&mut self, run: &mut finish::Run, rel: &str) -> Result<Rc<finish::Table>, String> {
        if let Some(t) = self.kept.get(rel) {
            return Ok(t.clone());
        }
        // redo-binding pass 5: the implicit-symbol catalog is the same for every file whose module node is the same (the
        // rows name the module scope and the file only; the file column is set by `fill`), so it is filled once a run
        let module = (rel == "implicit-symbol").then(|| self.module_position(run)).flatten();
        if let Some(m) = module
            && let Some(t) = self.implicit.as_ref().filter(|(at, _)| *at == m).map(|(_, t)| t.copy())
        {
            // the rows name the module scope and the file: the cached rows' file cells become this file's
            let mut t = t;
            let f = u64::from(self.file_no);
            for c in 0..t.tys.len() {
                if t.tys[c] == finish::Ty::File {
                    t.cols[c].iter_mut().for_each(|v| *v = f);
                }
            }
            let t = Rc::new(t);
            self.kept.insert(rel, t.clone());
            return Ok(t);
        }
        let mask = self.input_columns.get(rel).unwrap_or(u64::MAX);
        let selected = if mask == u64::MAX { languages::ColumnProjection::All } else { languages::ColumnProjection::First(mask) };
        let t = self.whole_fill(run, rel, &selected)?;
        if let Some(m) = module {
            *self.implicit = Some((m, t.copy()));
        }
        let t = Rc::new(t);
        self.kept.insert(rel, t.clone());
        Ok(t)
    }

    /// The position of the file's module node (the scope the implicit-symbol rows name).
    fn module_position(&self, run: &finish::Run) -> Option<u32> {
        let n = node_table::element_count(self.file.table).0 as u32;
        (0..n).find(|p| &*run.texts[run.kind_sym[usize::from(node_table::kind_of(self.file.table, Position(*p)).0)] as usize] == b"module")
    }

    fn whole_fill(&mut self, run: &mut finish::Run, rel: &str, columns: &languages::ColumnProjection) -> Result<finish::Table, String> {
        if let Some((Some(relation), _, _)) = run.catalog(rel)
            && let Some(rows) = self.completed.take(relation)
        {
            let timed = finish::fill_prof::timed();
            let start = if timed { thread_cpu() } else { 0.0 };
            let count = rows.0.first().map_or(0, |column| column.0.len());
            let answer = finish::owned_columns(run, rel, rows, self.words, columns, count)?;
            if timed {
                let cost = thread_cpu() - start;
                for label in ["convert", "completed"] {
                    let e = self.by_rel.entry(format!("{label}:{rel}")).or_default();
                    e.0 += cost;
                    e.1 += count as u64;
                    e.2 += 1;
                }
            }
            if finish::fill_prof::dumping() {
                let reference = self.fresh_fill(run, rel, columns)?;
                assert_eq!(answer.n, reference.n, "{rel} retained row count");
                for (c, words) in answer.cols.iter().enumerate() {
                    if columns.contains(c) { assert_eq!(*words, reference.cols[c], "{rel} retained column {c}"); }
                }
            }
            return Ok(answer);
        }
        self.fresh_fill(run, rel, columns)
    }

    fn fresh_fill(&mut self, run: &mut finish::Run, rel: &str, columns: &languages::ColumnProjection) -> Result<finish::Table, String> {
        let timed = finish::fill_prof::timed();
        // redo-fills pass 2: the provider's direct columns, no reader rows and no conversion (direct_fill.rs)
        if direct_fill::direct(rel)
            && let Some((t, write, make, reused)) = direct_fill::fill(self.file, run, rel, self.file_no, timed, columns.word(0), self.prefix, self.prefix_symbols, self.direct_memo)
        {
            if timed {
                let n = t.as_ref().map_or(0, |t| t.n as u64);
                if reused {
                    let e = self.by_rel.entry(format!("prefix:{rel}")).or_default();
                    e.1 += n;
                    e.2 += 1;
                }
                for (k, s) in [("reader", write), ("convert", make)] {
                    let e = self.by_rel.entry(format!("{k}:{rel}")).or_default();
                    e.0 += s;
                    e.1 += n;
                    e.2 += 1;
                }
            }
            if direct_fill::checking() {
                let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
                let relation = relation.ok_or_else(|| format!("{rel}: no reader relation"))?;
                self.filled.0.clear();
                let _ = self.file.fill(relation, languages::Wanted::Whole, self.words, &mut self.filled);
                let r = finish::filled_table(run, rel, &self.filled, self.words)?;
                direct_fill::check(run, rel, self.file_no, &r, t.as_ref().map_err(String::clone)?, columns.word(0));
            }
            return t;
        }
        let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
        let relation = relation.ok_or_else(|| format!("{rel}: no reader relation"))?;
        let t0 = if timed { thread_cpu() } else { 0.0 };
        let (answer, rows) = self.file.fill_columns(relation, columns, self.words, &mut self.filled);
        let t1 = if timed { thread_cpu() } else { 0.0 };
        let t = match answer {
            languages::FactAnswer::Answered(_) => finish::filled_columns(run, rel, &self.filled, self.words, columns, rows),
            _ => Err(format!("{rel} not answered")),
        };
        if timed {
            let t2 = thread_cpu();
            let n = t.as_ref().map_or(0, |t| t.n as u64);
            let e = self.by_rel.entry(format!("reader:{rel}")).or_default();
            e.0 += t1 - t0;
            e.1 += n;
            e.2 += 1;
            let e = self.by_rel.entry(format!("convert:{rel}")).or_default();
            e.0 += t2 - t1;
            e.1 += n;
            e.2 += 1;
        }
        t
    }

    /// The flow outcome is independent of the syntactic predecessor. Each possible binding or absence has its own row.
    fn read_resolution(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
        let snapshot = self.scheduled_read_predecessor(run)?;
        let (_, names, tys) = run.catalog(fact_catalog::RESOLUTION_REL).ok_or("no read-resolution")?;
        let mut rows = finish::Table::with_rows(names, tys, snapshot.n);
        let mut by_reference = vec![usize::MAX; node_table::element_count(self.file.table).0 as usize];
        for (index, &node) in snapshot.cols[0].iter().enumerate() {
            by_reference[node as usize] = index;
        }
        let mut missing = None;
        let complete = self.file.visit_read_outcomes(|reference, binding, outcome, basis| {
            let index = by_reference[reference as usize];
            if index != usize::MAX {
                for (col, value) in rows.cols.iter_mut().zip(&snapshot.cols) {
                    col.push(value[index]);
                }
                rows.cols[9].push(binding.map_or(finish::NULL, u64::from));
                rows.cols[10].push(u64::from(run.sym(outcome.as_bytes())));
                rows.cols[11].push(u64::from(run.sym(basis.as_bytes())));
                rows.n += 1;
            } else {
                missing = Some(reference);
            }
        });
        if let Some(reference) = missing {
            return Err(format!("read-resolution: missing scheduled read {reference}"));
        }
        if !complete {
            return Err("read-resolution: incomplete flow".to_owned());
        }
        Ok(rows)
    }

    /// A scheduled lookup can have one predecessor where control flow permits several reaching definitions.
    /// redo-binding probe: `read-resolution` from the provider's own lookup (`binding-resolution`, whose rows come per read
    /// nearest namespace first): the first row of each read, a deletion's only at depth 0 (its name is local), and a `none`
    /// row for every read or deletion (`binding-read`) no namespace of whose path holds an event of the name.
    fn scheduled_read_predecessor(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
        let res = self.whole(run, "binding-resolution")?;
        let reads = self.whole(run, "binding-read")?;
        let col = |t: &finish::Table, n: &str| t.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"));
        use pair_provider::predecessor::{LOOKUP, READ, WORDS, positions, project};
        let lookup = positions(LOOKUP, |name| col(&res, name))?;
        let read = positions(READ, |name| col(&reads, name))?;
        let (_, names, tys) = run.catalog(fact_catalog::PREDECESSOR_REL).ok_or("no scheduled-read-predecessor")?;
        let symbols = WORDS.map(|text| u64::from(run.sym(text.as_bytes())));
        let mut t = finish::Table::with_rows(names, tys, reads.n);
        project((0..res.n).map(|r| lookup.map(|c| res.cols[c][r])), (0..reads.n).map(|r| read.map(|c| reads.cols[c][r])),
            symbols, u64::from(self.file_no), |values| {
                for (col, value) in t.cols.iter_mut().zip(values) { col.push(value); }
                t.n += 1;
            });
        Ok(t)
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// redo-binding pass 3: `namespace-final` from the provider's events (`binding-event`) in schedule order, keeping the latest
    /// event of each (namespace, name) as the provider's resolution does (`binding_resolution::resolve`): a bind, annotation,
    /// cleanup, deletion, global or nonlocal is the name's state in its namespace; a global or nonlocal also holds in the
    /// declaring namespace. Reads and wildcards change nothing.
    fn namespace_final(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
        let ev = self.whole(run, "binding-event")?;
        let col = |n: &str| ev.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"));
        let (e_occ, e_name, e_scope, e_kind, e_ord, e_lex) =
            (col("occurrence")?, col("name")?, col("scope")?, col("kind")?, col("ordinal")?, col("lexical")?);
        let (read, wildcard) = (u64::from(run.sym(b"read")), u64::from(run.sym(b"wildcard")));
        let (global, nonlocal) = (u64::from(run.sym(b"global")), u64::from(run.sym(b"nonlocal")));
        let mut order: Vec<usize> = (0..ev.n).collect();
        order.sort_by_key(|r| ev.cols[e_ord][*r]);
        let mut latest: rustc_hash::FxHashMap<(u64, u64), usize> = rustc_hash::FxHashMap::default();
        for r in order {
            let kind = ev.cols[e_kind][r];
            if kind == read || kind == wildcard {
                continue;
            }
            let _ = latest.insert((ev.cols[e_scope][r], ev.cols[e_name][r]), r);
            if (kind == global || kind == nonlocal) && ev.cols[e_lex][r] != ev.cols[e_scope][r] {
                let _ = latest.insert((ev.cols[e_lex][r], ev.cols[e_name][r]), r);
            }
        }
        let (_, names, tys) = run.catalog(fact_catalog::FINAL_REL).ok_or("no namespace-final")?;
        let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(latest.len()); 6], n: 0 };
        for ((namespace, name), r) in latest {
            for (c, v) in [namespace, name, ev.cols[e_occ][r], ev.cols[e_kind][r], ev.cols[e_ord][r], u64::from(self.file_no)].into_iter().enumerate() {
                t.cols[c].push(v);
            }
            t.n += 1;
        }
        Ok(t)
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// redo-binding pass 4: `type-expression` (type_expression.rs) from the node table and four provider relations.
    fn type_expression(&mut self, run: &mut finish::Run, mask: u64) -> Result<finish::Table, String> {
        let qualified = self.whole(run, "qualified-name")?;
        let embedded = self.whole(run, "embedded-expression")?;
        // redo-fills pass 2: no root, no row, and the other two inputs are not filled
        if !type_expression::any_root(run, self.file.table, &qualified, embedded.n, type_expression::LISTS.get_or_init(type_expression::Lists::default), self.type_paths)? {
            let (_, names, tys) = run.catalog(fact_catalog::TYPE_EXPRESSION_REL).ok_or("no type-expression")?;
            return Ok(finish::Table { names, tys, cols: vec![Vec::new(); 3], n: 0 });
        }
        let arguments = self.whole(run, "call-argument")?;
        let keyword = run.sym(b"keyword");
        let has_keywords = run.kind_sym.iter().enumerate().any(|(i, sym)| *sym == keyword
            && !languages::node_table::ids_of_kind(self.file.table, languages::KindBits(i as u16)).is_empty());
        let identifiers = if has_keywords { Some(self.whole(run, "identifier")?) } else { None };
        let inputs = type_expression::Inputs { qualified: &qualified, arguments: &arguments, identifiers: identifiers.as_deref(), embedded: &embedded, lists: type_expression::LISTS.get_or_init(type_expression::Lists::default) };
        let rows = type_expression::rows(run, self.file.table, &inputs, self.type_paths)?;
        let (_, names, tys) = run.catalog(fact_catalog::TYPE_EXPRESSION_REL).ok_or("no type-expression")?;
        let mut t = finish::Table::projected(names, tys, rows.len(), mask);
        t.n = rows.len();
        if finish::request::wants(mask, 0) { t.cols[0].extend(rows.iter().map(|(id, _)| u64::from(*id))); }
        if finish::request::wants(mask, 1) {
            let (ty, value) = (u64::from(run.sym(b"type")), u64::from(run.sym(b"value")));
            t.cols[1].extend(rows.iter().map(|(_, typed)| if *typed { ty } else { value }));
        }
        if finish::request::wants(mask, 2) { t.cols[2].resize(t.n, u64::from(self.file_no)); }
        Ok(t)
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// redo-binding pass 4: `annotation-evaluation` (annotation_evaluation.rs) from the node table, the file's compiler features
    /// and target, and its embedded expressions.
    fn annotation_evaluation(&mut self, run: &mut finish::Run, mask: u64) -> Result<finish::Table, String> {
        // redo-fills pass 2: no annotation, no row (and no input filled)
        if !annotation_evaluation::has_annotation(run, self.file.table) {
            let (_, names, tys) = run.catalog(fact_catalog::ANNOTATION_EVALUATION_REL).ok_or("no annotation-evaluation")?;
            return Ok(finish::Table { names, tys, cols: vec![Vec::new(); 5], n: 0 });
        }
        let features = self.whole(run, "compiler-feature")?;
        let targets = self.whole(run, "language-target")?;
        let embedded = self.whole(run, "embedded-expression")?;
        let target = annotation_evaluation::target(run, &features, &targets)?;
        let rows = annotation_evaluation::rows(run, self.file.table, &target, &embedded)?;
        let (_, names, tys) = run.catalog(fact_catalog::ANNOTATION_EVALUATION_REL).ok_or("no annotation-evaluation")?;
        let mut t = finish::Table::projected(names, tys, rows.len(), mask);
        t.n = rows.len();
        if finish::request::wants(mask, 0) { t.cols[0].extend(rows.iter().map(|(id, _, _)| u64::from(*id))); }
        if finish::request::wants(mask, 1) { t.cols[1].extend(rows.iter().map(|(_, root, _)| u64::from(*root))); }
        for c in [2, 3] {
            if finish::request::wants(mask, c) {
                let words = annotation_evaluation::NAMES.map(|(e, m)| u64::from(run.sym(if c == 2 { e } else { m })));
                t.cols[c].extend(rows.iter().map(|(_, _, class)| words[*class]));
            }
        }
        if finish::request::wants(mask, 4) { t.cols[4].resize(t.n, u64::from(self.file_no)); }
        Ok(t)
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// redo-binding pass 4: a harness fact, made once a file (the facts that compose others read it from here).
    fn fact(&mut self, run: &mut finish::Run, rel: &str) -> Result<Rc<finish::Table>, String> {
        if let Some(t) = self.kept.get(rel) {
            return Ok(t.clone());
        }
        let timed = finish::fill_prof::timed();
        let t0 = if timed { thread_cpu() } else { 0.0 };
        let mask = self.input_columns.get(rel).unwrap_or(u64::MAX);
        let t = match rel {
            fact_catalog::TYPE_EXPRESSION_REL => self.type_expression(run, mask)?,
            fact_catalog::ANNOTATION_EVALUATION_REL => self.annotation_evaluation(run, mask)?,
            fact_catalog::FINAL_REL => self.namespace_final(run)?,
            fact_catalog::DEFERRED_RESOLUTION_REL => self.deferred_resolution(run)?,
            _ => self.deferred_visit(run, mask)?,
        };
        if timed {
            let e = self.by_rel.entry(format!("fact:{rel}")).or_default();
            e.0 += thread_cpu() - t0;
            e.1 += t.n as u64;
            e.2 += 1;
        }
        let t = Rc::new(t);
        self.kept.insert(rel, t.clone());
        Ok(t)
    }

    /// redo-binding pass 4: `deferred-resolution` (deferred_resolution.rs) from the provider's reads, events and lookup paths,
    /// the implicit symbols, and the facts namespace-final and deferred-visit.
    fn deferred_resolution(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
        let visits = self.fact(run, fact_catalog::DEFERRED_VISIT_REL)?;
        let (_, names, tys) = run.catalog(fact_catalog::DEFERRED_RESOLUTION_REL).ok_or("no deferred-resolution")?;
        if visits.n == 0 {
            // no postponed element: no row (and no fill of the inputs below)
            return Ok(finish::Table { names, tys, cols: vec![Vec::new(); 5], n: 0 });
        }
        let reads = self.whole(run, "binding-read")?;
        let events = self.whole(run, "binding-event")?;
        let implicit = self.whole(run, "implicit-symbol")?;
        let targets = self.whole(run, "language-target")?;
        let finals = self.fact(run, fact_catalog::FINAL_REL)?;
        let inputs = deferred_resolution::Inputs { reads: &reads, events: &events, implicit: &implicit, targets: &targets, finals: &finals, visits: &visits };
        let rows = deferred_resolution::rows(run, self.file.table, &inputs)?;
        let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(rows.len()); 5], n: rows.len() };
        for (reference, occurrence, kind, point) in rows {
            for (c, v) in [reference, occurrence, kind, point, u64::from(self.file_no)].into_iter().enumerate() {
                t.cols[c].push(v);
            }
        }
        Ok(t)
    }

    /// redo-binding pass 4: `deferred-visit` (deferred_visit.rs) from `type-expression`, `annotation-evaluation`, the node
    /// table, the file's embedded expressions, its name (a stub) and its annotations future.
    fn deferred_visit(&mut self, run: &mut finish::Run, mask: u64) -> Result<finish::Table, String> {
        let stub = run.texts[run.path_sym[self.file_no as usize] as usize].ends_with(b".pyi");
        let evaluations = self.fact(run, fact_catalog::ANNOTATION_EVALUATION_REL)?;
        let embedded = self.whole(run, "embedded-expression")?;
        let features = self.whole(run, "compiler-feature")?;
        let targets = self.whole(run, "language-target")?;
        let future = annotation_evaluation::target(run, &features, &targets)?.enabled;
        // redo-fills c2: outside a stub no element is deferred without a lazy annotation, an embedded expression, a type
        // parameter or a type alias (deferred_visit::rows' sources): such a file answers no row without the walk, and the
        // type expressions (read in a stub only) are never made outside one
        if !stub && embedded.n == 0 && !deferred_visit::lazy_any(run, &evaluations, future)? && !deferred_visit::has_type_parameters(run, self.file.table) {
            let (_, names, tys) = run.catalog(fact_catalog::DEFERRED_VISIT_REL).ok_or("no deferred-visit")?;
            return Ok(finish::Table { names, tys, cols: vec![Vec::new(); 3], n: 0 });
        }
        let types = if stub {
            self.fact(run, fact_catalog::TYPE_EXPRESSION_REL)?
        } else {
            let (_, names, tys) = run.catalog(fact_catalog::TYPE_EXPRESSION_REL).ok_or("no type-expression")?;
            Rc::new(finish::Table { names, tys, cols: vec![Vec::new(); 3], n: 0 })
        };
        let inputs = deferred_visit::Inputs { types: &types, evaluations: &evaluations, embedded: &embedded, stub, future };
        let rows = deferred_visit::rows(run, self.file.table, &inputs)?;
        let (_, names, tys) = run.catalog(fact_catalog::DEFERRED_VISIT_REL).ok_or("no deferred-visit")?;
        let mut t = finish::Table::projected(names, tys, rows.len(), mask);
        t.n = rows.len();
        if finish::request::wants(mask, 0) { t.cols[0].extend(rows.into_iter().map(u64::from)); }
        if finish::request::wants(mask, 1) { t.cols[1].resize(t.n, u64::from(run.sym(b"deferred"))); }
        if finish::request::wants(mask, 2) { t.cols[2].resize(t.n, u64::from(self.file_no)); }
        Ok(t)
    }
}

impl finish::Provider for ColFacts<'_, '_, '_, '_> {
    /// redo-fills pass 3: the file's one request answered in one call: each asked relation written once, in the request's
    /// order; the model parts are built at the first family that reads each (and never again in the file).
    fn answer(&mut self, run: &mut finish::Run, request: &finish::request::Request<'_>, out: &mut Vec<Result<finish::Table, String>>) -> Result<(), String> {
        // the model's parts the asked families read, built once before the first of them (timed apart as `model:`)
        let flow = request.relations.iter().any(|a| FLOW_READERS.contains(&a.rel));
        let resolution = request.relations.iter().any(|a| RESOLUTION_READERS.contains(&a.rel));
        if flow || resolution {
            let timed = finish::fill_prof::timed();
            let t0 = if timed { thread_cpu() } else { 0.0 };
            self.file.build_model(flow, resolution);
            if timed {
                let dt = thread_cpu() - t0;
                self.time += dt;
                let e = self.by_rel.entry(format!("model:{}", if flow { "flow" } else { "resolution" })).or_default();
                e.0 += dt;
                e.2 += 1;
            }
        }
        // the families first, the facts composed of them after (each input is made once and kept; the clock then gives
        // each its own work), the answer in the request's order
        let stub = run.paths[self.file_no as usize].ends_with(".pyi");
        self.input_columns = self.input_layout.for_file(request.relations, stub);
        let base = out.len();
        out.resize_with(base + request.relations.len(), || Err(String::new()));
        let mut pending = Vec::new();
        for composed in [false, true] {
            for (i, asked) in request.relations.iter().enumerate() {
                if COMPOSED.contains(&asked.rel) == composed {
                    out[base + i] = match asked.scope {
                        finish::Scope::Files(files) => self.project.fill(run, asked.rel, asked.columns.word(0), files, self.file_no, self.file.table, self.src),
                        finish::Scope::File(f) if f == self.file_no => match self.one(run, asked.rel, asked.columns) {
                            Ok(Made::Owned(t)) => Ok(t),
                            Ok(Made::Shared(t)) => { pending.push((i, t)); continue; }
                            Err(e) => Err(e),
                        },
                        finish::Scope::File(f) => Err(format!("asked for file {f}, this provider holds file {}", self.file_no)),
                    };
                }
            }
        }
        // Composed writers have finished reading their inputs. Hand the requested
        // tables to the executor in request order, moving their column buffers.
        self.kept.clear();
        for (i, t) in pending {
            // Repeated asks share the completed table until the last one takes ownership.
            out[base + i] = Ok(Rc::try_unwrap(t).unwrap_or_else(|t| t.copy()));
        }
        for (entry, table) in request.relations.iter().zip(&mut out[base..]) {
            if let Ok(t) = table {
                for (c, col) in t.cols.iter_mut().enumerate() {
                    if !entry.columns.contains(c) { col.clear(); }
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        self.render_node(node, context)
    }
}

/// redo-fills pass 3: the provider's facts composed of its other relations (made after them in an answer).
const COMPOSED: [&str; 9] = [fact_catalog::PREDECESSOR_REL, fact_catalog::REGION_REL, fact_catalog::RESOLUTION_REL, fact_catalog::TYPE_REGION_REL, fact_catalog::FINAL_REL, fact_catalog::TYPE_EXPRESSION_REL, fact_catalog::ANNOTATION_EVALUATION_REL, fact_catalog::DEFERRED_VISIT_REL, fact_catalog::DEFERRED_RESOLUTION_REL];

/// redo-fills pass 3: the provider's families that read the model's flow part (the binding facts, the evaluation regions,
/// and the facts composed of them), and those that read its resolution part.
const FLOW_READERS: [&str; 10] = ["binding-event", "binding-read", "binding-use", "binding-shadow", "lookup-scope", "binding-resolution", fact_catalog::REGION_REL, fact_catalog::TYPE_REGION_REL, fact_catalog::RESOLUTION_REL, fact_catalog::FINAL_REL];
const RESOLUTION_READERS: [&str; 1] = ["qualified-name"];

impl ColFacts<'_, '_, '_, '_> {
    fn make(&mut self, run: &mut finish::Run, rel: &str, selection: &languages::ColumnProjection) -> Result<Made, String> {
        let columns = selection.word(0);
        Ok(if rel == fact_catalog::REGION_REL {
            let (deferred, rows) = self.file.evaluation_region().ok_or("evaluation-region: the model is incomplete")?;
            let (_, names, tys) = run.catalog(rel).ok_or("no evaluation-region")?;
            let mut t = finish::Table::projected(names, tys, rows.len(), columns);
            t.n = rows.len();
            if finish::request::wants(columns, 0) { t.cols[0].extend(rows.iter().map(|r| u64::from(r.id))); }
            if finish::request::wants(columns, 1) { t.cols[1].extend(rows.iter().map(|r| u64::from(r.region))); }
            if finish::request::wants(columns, 2) { t.cols[2].extend(rows.iter().map(|r| u64::from(run.sym(r.kind.as_bytes())))); }
            if finish::request::wants(columns, 3) {
                t.cols[3].extend(rows.iter().map(|r| if r.annotation == u32::MAX { finish::NULL } else { u64::from(r.annotation) }));
            }
            if finish::request::wants(columns, 4) { t.cols[4].resize(t.n, u64::from(run.sym(if deferred { b"deferred" } else { b"immediate" }))); }
            if finish::request::wants(columns, 5) { t.cols[5].resize(t.n, u64::from(self.file_no)); }
            Made::Owned(t)
        } else if rel == fact_catalog::RELATIVE_IMPORT_REL {
            Made::Owned(relative_import::walked(run, self.project, self.file, self.file_no)?)
        } else if matches!(rel, import_syntax::NEIGHBORS | import_syntax::STATEMENTS | import_syntax::ALIASES | import_syntax::CONTEXTUAL_ALIASES | import_syntax::LINE_COMMENTS | import_syntax::ENTRIES) {
            Made::Owned(import_syntax::fill(run, self.file, self.src, self.project, self.file_no, rel)?)
        } else if rel == fact_catalog::PRE_DECLARATION_READ_REL {
            let (_, names, tys) = run.catalog(rel).ok_or("no pre-declaration-read")?;
            let mut rows = finish::Table::with_rows(names, tys, 0);
            let complete = self.file.visit_pre_declaration_reads(|at, scope, phase, ordinal| {
                rows.cols[0].push(u64::from(at));
                rows.cols[1].push(u64::from(scope));
                rows.cols[2].push(u64::from(phase));
                rows.cols[3].push(ordinal);
                rows.cols[4].push(u64::from(self.file_no));
                rows.n += 1;
            });
            if !complete {
                return Err("pre-declaration-read: incomplete flow".to_owned());
            }
            Made::Owned(rows)
        } else if matches!(rel, fact_catalog::ASSIGNMENT_SOURCE_REL | fact_catalog::NAMESPACE_SITES_REL) {
            let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no {rel}"))?;
            let mut rows = finish::Table::with_rows(names, tys, 0);
            let complete = pair_provider::sites::visit(rel, self.file, u64::from(self.file_no),
                |word| u64::from(run.sym(word.as_bytes())), |values| {
                    for (col, &value) in rows.cols.iter_mut().zip(values) { col.push(value); }
                    rows.n += 1;
                });
            if !complete { return Err(format!("{rel}: incomplete flow")); }
            Made::Owned(rows)
        } else if rel == fact_catalog::BINDING_SITES_REL {
            let (_, names, tys) = run.catalog(rel).ok_or("no binding-sites")?;
            let mut rows = finish::Table::with_rows(names, tys, 0);
            let complete = self.file.visit_binding_sites(|site| {
                for (column, value) in rows.cols.iter_mut().zip(pair_provider::site_cells(site, self.file_no as usize)) {
                    column.push(value);
                }
                rows.n += 1;
            });
            if !complete {
                return Err("binding-sites: incomplete flow".to_owned());
            }
            Made::Owned(rows)
        } else if rel == fact_catalog::PREDECESSOR_REL {
            Made::Owned(self.scheduled_read_predecessor(run)?)
        } else if rel == fact_catalog::RESOLUTION_REL {
            Made::Owned(self.read_resolution(run)?)
        } else if rel == fact_catalog::TYPE_REGION_REL {
            let ids = self.file.type_regions().ok_or("type-region: the model is incomplete")?;
            let (_, names, tys) = run.catalog(rel).ok_or("no type-region")?;
            let mut t = finish::Table::projected(names, tys, ids.len(), columns);
            t.n = ids.len();
            if finish::request::wants(columns, 0) { t.cols[0].extend(ids.into_iter().map(u64::from)); }
            if finish::request::wants(columns, 1) { t.cols[1].resize(t.n, u64::from(self.file_no)); }
            Made::Owned(t)
        } else if rel == fact_catalog::FINAL_REL
            || rel == fact_catalog::TYPE_EXPRESSION_REL
            || rel == fact_catalog::ANNOTATION_EVALUATION_REL
            || rel == fact_catalog::DEFERRED_VISIT_REL
            || rel == fact_catalog::DEFERRED_RESOLUTION_REL
        {
            Made::Shared(self.fact(run, rel)?)
        } else if self.input_columns.get(rel).is_some() {
            Made::Shared(self.whole(run, rel)?)
        } else {
            Made::Owned(self.whole_fill(run, rel, selection)?)
        })
    }

    /// One asked relation of this file: none made where the file's own contents say it has no row (`row_count`).
    fn one(&mut self, run: &mut finish::Run, rel: &str, columns: &languages::ColumnProjection) -> Result<Made, String> {
        if self.kept.get(rel).is_none()
            && let Some((Some(relation), names, tys)) = run.catalog(rel)
            && self.file.row_count(relation) == Some(0)
        {
            return Ok(Made::Owned(finish::Table::with_rows(names, tys, 0)));
        }
        let timed = finish::fill_prof::timed();
        let t0 = if timed { thread_cpu() } else { 0.0 };
        let made = self.make(run, rel, columns)?;
        let t = made.table();
        let dt = if timed { thread_cpu() - t0 } else { 0.0 };
        self.time += dt;
        self.rows += t.n as u64;
        let e = if let Some(e) = self.by_rel.get_mut(rel) { e } else { self.by_rel.entry(rel.to_owned()).or_default() };
        e.0 += dt;
        e.1 += t.n as u64;
        e.2 += 1;
        if finish::fill_prof::counting() {
            if t.n == 0 {
                self.by_rel.entry(format!("zero:{rel}")).or_default().2 += 1;
            }
            let site = finish::fill_prof::SITE.with(std::cell::Cell::get);
            self.by_rel.entry(format!("site:{site}:{rel}")).or_default().2 += 1;
        }
        if finish::fill_prof::dumping() {
            if t.cols.iter().any(|c| c.len() < t.n) {
                // The gate compares full-row digests across versions. Check every
                // delivered cell in order against the full writer before hashing it.
                let reference = match rel {
                    fact_catalog::TYPE_EXPRESSION_REL => self.type_expression(run, u64::MAX)?,
                    fact_catalog::ANNOTATION_EVALUATION_REL => self.annotation_evaluation(run, u64::MAX)?,
                    fact_catalog::DEFERRED_VISIT_REL => self.deferred_visit(run, u64::MAX)?,
                    fact_catalog::REGION_REL | fact_catalog::TYPE_REGION_REL => match self.make(run, rel, &languages::ColumnProjection::All)? {
                        Made::Owned(t) => t, Made::Shared(_) => unreachable!(),
                    },
                    _ => self.whole_fill(run, rel, &languages::ColumnProjection::All)?,
                };
                assert_eq!(t.n, reference.n, "{rel} rows");
                for c in 0..t.cols.len() {
                    if columns.contains(c) {
                        assert_eq!(t.cols[c], reference.cols[c], "{rel} column {}", t.names[c]);
                    }
                }
                finish::fill_prof::dump(run, self.file_no, rel, &reference);
            } else {
                finish::fill_prof::dump(run, self.file_no, rel, &t);
            }
        }
        Ok(made)
    }

    fn render_node(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        let context = match context {
            "assignment-target" => languages::RenderContext::AssignmentTarget,
            "attribute-receiver" => languages::RenderContext::AttributeReceiver,
            "positional-argument" => languages::RenderContext::PositionalArgument,
            other => return Err(format!("render context {other}")),
        };
        let node = ryo_relations::domain::relations::service::node_id(FileId(self.file_no), languages::Position(node));
        match self.file.render(languages::RenderRequest { node, context }) {
            languages::RenderOutcome::Known(t) => Ok(Some(t.0.as_bytes().to_vec())),
            languages::RenderOutcome::NotApplicable => Ok(None),
            languages::RenderOutcome::Unsupported(_) => Err("render unsupported".to_owned()),
        }
    }
}

impl finish::Provider for Facts<'_, '_, '_> {
    /// redo-fills pass 3: the reader's facts answer the file's one request relation by relation.
    fn answer(&mut self, run: &mut finish::Run, request: &finish::request::Request<'_>, out: &mut Vec<Result<finish::Table, String>>) -> Result<(), String> {
        for asked in request.relations {
            out.push(match asked.scope {
                finish::Scope::Files(files) => self.project.fill(run, asked.rel, asked.columns.word(0), files, self.file, self.facts.table, self.src),
                finish::Scope::File(f) if f == self.file => self.one(run, asked.rel, asked.columns),
                finish::Scope::File(f) => Err(format!("asked for file {f}, this provider holds file {}", self.file)),
            });
        }
        Ok(())
    }

    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        self.render_node(node, context)
    }
}

impl Facts<'_, '_, '_> {
    fn one(&mut self, run: &mut finish::Run, rel: &str, columns: &languages::ColumnProjection) -> Result<finish::Table, String> {
        if rel == fact_catalog::RELATIVE_IMPORT_REL {
            let imports = self.one(run, "static-import", &languages::ColumnProjection::All)?;
            return relative_import::targets(run, &imports, self.file, |run| {
                self.project.fill(run, provider_packaging::MODULE_IDENTITY_REL, u64::MAX, &[self.file], self.file, self.facts.table, self.src)
            });
        }
        let t0 = thread_cpu();
        let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
        let relation = relation.ok_or_else(|| format!("{rel} needs VEC_COLWALK"))?;
        let (answer, rows) = self.facts.facts.fill_columns(relation, languages::Wanted::Whole, columns, self.words, &mut self.filled);
        let t = match answer {
            languages::FactAnswer::Answered(_) => finish::filled_columns(run, rel, &self.filled, self.words, columns, rows),
            _ => Err(format!("{rel} not answered")),
        }?;
        let dt = thread_cpu() - t0;
        self.time += dt;
        self.rows += t.n as u64;
        let e = if let Some(e) = self.by_rel.get_mut(rel) { e } else { self.by_rel.entry(rel.to_owned()).or_default() };
        e.0 += dt;
        e.1 += t.n as u64;
        e.2 += 1;
        Ok(t)
    }

    fn render_node(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        let context = match context {
            "assignment-target" => languages::RenderContext::AssignmentTarget,
            "attribute-receiver" => languages::RenderContext::AttributeReceiver,
            "positional-argument" => languages::RenderContext::PositionalArgument,
            other => return Err(format!("render context {other}")),
        };
        let node = ryo_relations::domain::relations::service::node_id(FileId(self.file), languages::Position(node));
        match self.facts.facts.render(languages::RenderRequest { node, context }) {
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
fn run_finish(walk: &impl Walk, settings: ryo_python::settings::PythonParserSettings, vocab: &Vocab, args: &[&str]) {
    let plan = finish::load(args[0]);
    type_expression::load_lists(args[0]);
    let list = files(args[1]);
    let reader_needs = fact_catalog::walk_needs(Some(&plan), &[]);
    let mut fin = visit::Finisher::new(&plan, walk, vocab, args);
    fin.prepare_requests(vocab, &[]);
    let seed: Option<std::collections::HashSet<(String, String)>> =
        flag(args, "--seed").map(|p| std::fs::read_to_string(p).expect("seed").lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_owned(), b.to_owned())).collect());
    let mut seed_by_path: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for (r, f) in seed.iter().flatten() {
        seed_by_path.entry(f.as_str()).or_default().insert(r.as_str());
    }
    // 6f probe: `VEC_COLWALK` walks through the direct-columns walk (the provider's own model is reachable there, for the
    // evaluation region); `--dump F` writes every emit's rows instead of findings.
    let colwalk = std::env::var_os("VEC_COLWALK").is_some();
    let mut cwalk = ryo_python::fixtures::columns::ColumnWalk::with_settings(
        ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names"),
        settings,
    );
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
        if colwalk {
            let _ = cwalk.walked(path, &bytes, &mut table, &mut |file| fin.column_file(file_no, path, file, &bytes, &wanted, None));
        } else {
            let _ = ryo_python::fixtures::walking::walked_request(walk, WalkRequest { path: languages::SourcePath(path), file: FileId(file_no as u32), source: ByteSlice(&bytes) }, reader_needs, &mut table, &mut scratch, None, &mut |mut file| fin.reader_file(file_no, path, &mut file, &bytes, &wanted));
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

/// redo-general: the byte strings one of which a file's source must hold for a `source-matches` pattern of the plan to
/// match in it: each pattern's literal set (regex-syntax's extractor) where it is finite and holds no empty string;
/// distinct, in plan order. (A `matches` call tests a cell's text, which need not be the source's: not read here.)
fn source_literals(plan: &finish::Plan) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    for n in plan.nodes.iter().filter(|n| n.op == "source-matches") {
        let Some(p) = n.j["args"]["pattern"].as_str() else { continue };
        let Ok(hir) = regex_syntax::ParserBuilder::new().utf8(false).build().parse(p) else { continue };
        let seq = regex_syntax::hir::literal::Extractor::new().extract(&hir);
        let Some(lits) = seq.literals() else { continue };
        if lits.is_empty() || lits.iter().any(|l| l.as_bytes().is_empty()) {
            continue;
        }
        for l in lits {
            if !out.iter().any(|o| o == l.as_bytes()) {
                out.push(l.as_bytes().to_vec());
            }
        }
    }
    out
}

/// `vec funnel <way> <files> <anchors> <plans> [--repeat K] [--rules-out F] [--hits-out F]`: ways `cands`, `structure`,
/// `interp`, `spec`, `knock`.
/// `<anchors>` a directory: the pack; the first half's spec is prepared in this process from its parsed rules (`first.rs`), the
/// parse shared with the second half's when `--finish` names the same pack (`pack`); `<anchors>` and `<plans>` files: the
/// spec prepared beforehand (the oracle path).
fn run_funnel(walk: &impl Walk, settings: ryo_python::settings::PythonParserSettings, vocab: &Vocab, args: &[&str], mut fin: Option<&mut visit::Finisher<'_>>, pack: Option<(&str, &lower::ReadPack)>) {
    let way = args[0];
    let list = files(args[1]);
    let mut fact_catalog = facts::Catalog::default();
    let mut interner = facts::Interner { ids: facts::Table::named(&flag(args, "--hasher").unwrap_or_else(|| "std".to_owned())), ..Default::default() };
    let f0 = thread_cpu();
    let no_skip = std::env::var_os("VEC_NO_SKIP").is_some();
    let mut pairs = None;
    let mut pair_steps = 0;
    let mut pair_cpu = 0.0;
    let mut exact_rows = Vec::new();
    let (rules, frules, words) = if std::path::Path::new(args[2]).is_dir() {
        let owned;
        let read = match pack {
            Some((dir, read)) if std::fs::canonicalize(dir).ok() == std::fs::canonicalize(args[2]).ok() => read,
            _ => {
                let request = flag(args, "--request");
                owned = lower::read_pack(std::path::Path::new(args[2]), request::rule_context(request.as_deref()));
                println!("start-up first half pack: read {:.4} parse {:.4} s", owned.read, owned.parse);
                &owned
            }
        };
        let f1 = thread_cpu();
        // A separately loaded finish plan may use the same rule names for
        // different emits. Only the shared typed plan certifies its output.
        let shared_plan = read.typed.borrow().is_some();
        let shared_steps = shared_plan && pack.is_some_and(|(_, combined)| std::ptr::eq(read, combined));
        let mut prepared = first::prepare(read, python_language(vocab), vocab);
        if !shared_plan {
            prepared.pairs.exact.clear();
        }
        if !shared_steps { prepared.pairs.drivers.clear(); }
        let f2 = thread_cpu();
        let rules = rules_of_skips(&prepared.skips, vocab);
        let (frules, words) = funnel::build(&prepared.rules, vocab, &mut fact_catalog, &mut interner);
        pairs = Some(prepared.pairs);
        let f3 = thread_cpu();
        println!(
            "start-up first half: from the pack: analysis {:.4} load {:.4} total {:.4} s; rules {}",
            f2 - f1, f3 - f2, f3 - f0, frules.len()
        );
        (rules, frules, words)
    } else {
        let rules = load_rules(args[2], vocab);
        let (frules, words) = funnel::load(args[3], vocab, &mut fact_catalog, &mut interner);
        println!("start-up first half: from the files: load {:.4} s; rules {} node {} plans {}", thread_cpu() - f0, frules.len(), frules.iter().filter(|r| r.node).count(), frules.iter().map(|r| r.plans.len()).sum::<usize>());
        (rules, frules, words)
    };
    if let Some(fin) = fin.as_deref_mut() {
        let drivers = pairs.as_ref().filter(|_| !no_skip).map_or(&[][..], |program| program.drivers.as_slice());
        fin.prepare_requests(vocab, drivers);
    }

    let use_facts = args.contains(&"--facts");
    let reader_needs = fact_catalog::walk_needs(fin.as_deref().map(|f| f.plan()), &fact_catalog.relations);
    let slots_n = fact_catalog.relations.len();
    let mut store = facts::Store { remap_on: args.contains(&"--remap"), ..Default::default() };
    let token_pass = args.contains(&"--token-pass");
    let check_links = args.contains(&"--check-token-links");
    let token_slot = fact_catalog.names.iter().position(|n| n == "token-link");
    // redo-general: the provider's token order (its lexical model), the same one the second half reads
    let token_kinds = provider_tokens::Lexicon::new(&|n| vocab.kinds.get(n).copied());
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
    let mut cwalk = ryo_python::fixtures::columns::ColumnWalk::with_settings(
        ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names"),
        settings,
    );
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
    let name_token = vocab.kinds.get(ryo_python::fixtures::columns::NAME_KIND).copied().expect("name-token kind");
    // redo-general: the ASCII bytes the provider's identifier rule takes inside a word (after a first `_`)
    let word_byte: [bool; 128] = std::array::from_fn(|b| ryo_python::fixtures::columns::is_identifier(&format!("_{}", char::from(b as u8))));
    let ac = aho_corasick::AhoCorasick::builder()
        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
        .build(words.names.iter().map(String::as_bytes))
        .expect("aho-corasick");
    // redo-general: the byte patterns of the `bytes6` probe from the plan's own text patterns (`source_literals`), not a list
    // of particular rules' bytes; at most 64 (one bit each)
    let byte_patterns: Vec<Vec<u8>> = fin.as_deref().map(|f| source_literals(f.plan())).unwrap_or_default().into_iter().take(64).collect();
    let bytes_ac = aho_corasick::AhoCorasick::new(&byte_patterns).expect("byte patterns");
    if flag(args, "--words").as_deref() == Some("bytes6") {
        println!("byte patterns from the plan: {} {:?}", byte_patterns.len(), byte_patterns.iter().map(|p| String::from_utf8_lossy(p).into_owned()).collect::<Vec<_>>());
    }
    let mut byte_files = vec![0u64; byte_patterns.len() + 1];
    let mut byte_hits = 0u64;
    let mut present = vec![0u64; ww];
    let mut table_present = vec![0u64; ww];
    let mut word_pairs: Vec<(u16, u32)> = Vec::new();
    let mut word_offsets: Vec<u32> = Vec::new();
    let mut word_pos: Vec<u32> = Vec::new();
    // the walk's code of each name token, and the required word of each code met in the file
    let mut name_codes: Vec<u32> = Vec::new();
    let mut word_memo: Vec<u32> = Vec::new();
    let mut fixed_memo: Vec<u32> = Vec::new();
    let (mut missing_in_src, mut extra_in_src, mut present_total, mut distinct_total) = (0u64, 0u64, 0u64, 0u64);
    let (mut alive_kinds, mut alive_both, mut rule_files) = (0u64, 0u64, 0u64);
    let mut word_entry_bound = 0u64;
    let repeat: usize = flag(args, "--repeat").map_or(1, |b| b.parse().expect("repeat"));
    let by_name: HashMap<&str, usize> = rules.iter().enumerate().map(|(i, r)| (r.name.as_str(), i)).collect();
    let skip_of: Vec<Option<usize>> = frules.iter().map(|r| by_name.get(r.name.as_str()).copied()).collect();
    // the kinds-and-fields skip of each plan rule, flat: a clause is its kind words then its field words
    let mut skip = Skip::new(skip_of.iter().map(|i| i.map_or((false, &[][..]), |i| (rules[i].never, rules[i].clauses.as_slice()))), frules.iter().map(|r| r.cnfs.as_slice()), words_of(vocab.kind_names.len()), words_of(vocab.field_names.len()));
    let mut kinds: Vec<u64> = Vec::new();
    let fields_width = vocab.field_names.len();
    let mut units = funnel::units(&frules);
    if args.contains(&"--only-knock-units") {
        units.retain(|u| u.knock);
    }
    let group_list = funnel::groups(&frules, &units);
    let structured = matches!(way, "structure" | "spec" | "knock");
    // the table writes the structure the checks read (element sets, the plans' enclosing columns) as ids are numbered
    let enclosing: Vec<ryo::languages::KindSet> = fact_catalog.enc_sets.iter().map(|s| ryo::languages::KindSet(*s)).collect();
    if structured {
        cwalk.structured(enclosing.clone());
    }
    let max_regs = frules.iter().flat_map(|r| r.plans.iter().map(|p| p.regs)).max().unwrap_or(1).max(1);
    // each plan's place in `plan_on`: whether this file enters its disjunct at it (always, for a disjunct of one plan)
    let plan_base: Vec<usize> = frules.iter().scan(0, |a, r| { let b = *a; *a += r.plans.len(); Some(b) }).collect();
    let mut plan_on = vec![true; frules.iter().map(|r| r.plans.len()).sum::<usize>()];
    let mut cands = vec![0u64; units.len()];
    let mut surv = vec![0u64; units.len()];
    let mut unit_steps = vec![0u64; units.len()];
    let mut knock_steps = 0u64;
    let mut sv = survivors::Survivors::default();
    let mut phases = Phases::default();
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let mut cols = Columns::default();
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
    // Only a rule/file bit crosses this seam. Explicit enumeration or diagnostic
    // outputs retain the exhaustive evaluator and its per-unit accounting.
    let seed_only = (fin.is_some() || args.contains(&"--file-seeds")) && way == "spec" && direct && use_facts
        && word_src == "table" && word_skip && repeat == 1
        && !ask_all && !check_direct && !per_fact && !plant && dump_out.is_none()
        && !["--enumerate-nodes", "--rules-out", "--surv-out", "--hits-out", "--alive-out", "--only-knock-units"].iter().any(|flag| args.contains(flag))
        && std::env::var_os("VEC_NO_SKIP").is_none() && std::env::var_os("VEC_NO_PREFIX").is_none();
    let mut file_seeds = seed_only.then(|| file_seeds::FileSeeds::new(&frules));
    let seed_out = flag(args, "--seed-out");
    let mut seed_dump = String::new();
    let started = thread_cpu();
    // the second half inside this walk (`--finish`): each plan emit's rules of this plan set by name (the seed's rule is a
    // name), and the file's seed as a set over the rules
    assert!(fin.is_none() || (use_facts && direct), "--finish in the first half's walk needs --facts --direct");
    let emit_rules: Vec<Vec<usize>> = fin.as_ref().map(|f| f.emit_names().map(|(_, n)| frules.iter().enumerate().filter(|(_, r)| r.name == n).map(|(i, _)| i).collect()).collect()).unwrap_or_default();
    let mut in_seed = vec![false; frules.len()];
    let node_class: Vec<bool> = frules.iter().map(|r| r.node).collect();
    let mut exact_counts = vec![(0u64, 0u64, 0u64); frules.len()];
    let mut exact_log = flag(args, "--exact-out").map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("exact pairs")));
    let fin0 = fin.as_ref().map_or(0.0, |f| f.time);
    println!("start-up to first file {:.4} s", thread_cpu());
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
        let mut codes_taken = false;
        macro_rules! prepare {
            ($t:expr,$catalog_file:expr,$fill:expr) => {{
                let pt: &NodeTable = $t;
                let c1 = thread_cpu();
                fields_present(pt, &mut cols, fields_width.max(1));
                // the structure (element sets, enclosing columns) is written by the table as ids are numbered (redo-onepass
                // pass 2), so prefix cycle 3's structure pass here is gone
                phases.add("field buckets + structure pass", thread_cpu() - c1);
                if way != "structure" {
                // the file's required words
                let cw = thread_cpu();
                if word_src != "none" {
                    // redo-general: a byte that may sit inside a word: an ASCII byte the provider's identifier rule takes after a
                    // first letter, or any byte of a multi-byte character (the run is then looked up whole)
                    let is_word = |b: u8| b >= 0x80 || word_byte[usize::from(b)];
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
                            // redo-onepass pass 2: the walk's name codes when the visit took them, else by bytes
                            let tokens = node_table::ids_of_kind(pt, KindBits(name_token)).iter().copied();
                            let (starts, ends) = (node_table::starts(pt), node_table::ends(pt));
                            let spelling = |i: usize| &bytes[starts[i] as usize..ends[i] as usize];
                            funnel::file_words(tokens, spelling, if codes_taken { &name_codes } else { &[] }, &words, &mut word_memo, &mut fixed_memo, &mut present, &mut word_pairs);
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
                            // the byte patterns of the plan's text patterns (`byte_patterns`): one multi-pattern scan
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
                    phases.add(if word_src == "bytes6" { "byte-pattern pass (aho-corasick, the plan's literals)" } else if word_src == "table" { "word pass (name tokens -> required-word bitset + index)" } else if word_src == "all" { "word pass (distinct word set of all name tokens)" } else if word_src == "bytes" { "word pass (identifier runs of the source bytes)" } else { "word pass (aho-corasick over the source bytes)" }, thread_cpu() - cw);
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
                skip.file(&kinds, &cols.field_present);
                kinds_alive.extend((0..frules.len()).map(|ri| skip.alive(ri)));
                // the words only of a rule the kinds and fields keep
                words_alive.clear();
                words_alive.extend(kinds_alive.iter().enumerate().map(|(ri, k)| *k && skip.words_alive(ri, &present)));
                alive.clear();
                alive.extend(kinds_alive.iter().zip(&words_alive).map(|(k, w)| *k && (!word_skip || *w)));
                // redo-honest: `VEC_NO_SKIP` keeps every rule alive in every file (a check of the skip: findings must not change)
                if no_skip {
                    alive.iter_mut().for_each(|a| *a = true);
                }
                exact_rows.clear();
                if !no_skip && let Some(program) = &pairs {
                    let start = thread_cpu();
                    let exact = fin.as_ref().filter(|f| f.can_finish_exact()).map(|_| &mut exact_rows);
                    pair_steps += if let Some(fin) = fin.as_deref_mut() {
                        let (run, words, mut completed) = fin.prefix_storage(file_no);
                        program.file(pt, &bytes, Some(run.paths[file_no].as_bytes()), words, |rel, words, out| {
                            if let pair_summary::Source::Catalog(catalog) = rel
                                && let Some(answer) = host_columns::fill(catalog, Some(run), file_no, out) { return answer; }
                            ($fill)(rel, words, out)
                        }, &mut alive, $catalog_file, Some(&mut completed), exact)
                    } else {
                        program.file(pt, &bytes, None, &mut fwords, $fill, &mut alive, $catalog_file, None, exact)
                    };
                    pair_cpu += thread_cpu() - start;
                }
                rule_files += frules.len() as u64;
                alive_kinds += kinds_alive.iter().filter(|a| **a).count() as u64;
                alive_both += alive.iter().filter(|a| **a).count() as u64;
                for (ri, a) in alive.iter().enumerate() {
                    if *a && !seed_only {
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
                if dump_out.is_some() {
                    dump::table(&mut dumped, file_no, t, vocab.kind_names.len());
                    dump::structure(&mut dumped, file_no, t);
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
                        sets: node_table::element_sets(t),
                        facts: &store,
                        enc: node_table::enclosing_ids(t),
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
                    // no census: each disjunct with alternatives enters this file at the one whose buckets hold the fewest of
                    // the file's nodes (any of them covers every node that can match)
                    let bucket_len = |key: &Key| match *key {
                        Key::Word(w) => (word_offsets[usize::from(w) + 1] - word_offsets[usize::from(w)]) as usize,
                        Key::Kind(k) => node_table::ids_of_kind(t, KindBits(k)).len(),
                        Key::Field(f) => node_table::ids_of_field(t, languages::FieldCode(f)).len(),
                    };
                    for (ri, rule) in frules.iter().enumerate() {
                        if !alive[ri] {
                            continue;
                        }
                        for alts in &rule.alts {
                            let size = |p: &usize| rule.plans[*p].keys.iter().map(bucket_len).sum::<usize>();
                            let best = alts.iter().min_by_key(|p| size(p)).copied();
                            for p in alts {
                                plan_on[plan_base[ri] + p] = Some(*p) == best;
                            }
                        }
                    }
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
                            let on = |p: &usize| plan_on[plan_base[unit.rule] + p];
                            if !unit.plans.iter().any(on) {
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
                                        for p in unit.plans.iter().filter(|p| on(p)) {
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
                                        if !on(p) || (word_skip && !funnel::cnf_holds(&plan.words, &present)) {
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
                    file.name_token_codes(&mut name_codes);
                    codes_taken = true;
                    // Whole rows kept for the provider are born with their real file identity.
                    file.set_file(FileId(file_no as u32));
                    prepare!(ft, FileId(file_no as u32), |rel,words,out| pair_provider::read(file, file_no, rel, words, out));
                    file.set_file(FileId(0));
                    prepared = true;
                    let index = file_seeds::Index { table: ft, offsets: &word_offsets, words: &word_pos };
                    if let Some(seeds) = &mut file_seeds {
                        let now = thread_cpu();
                        seeds.choose(&frules, &index, &alive, &present, &mut in_seed);
                        phases.add("seed candidates", thread_cpu() - now);
                        let now = thread_cpu();
                        let ctx = funnel::Ctx { table: ft, src: &bytes, sets: node_table::element_sets(ft),
                            enc: node_table::enclosing_ids(ft), facts: &store, matches: &interner.matches };
                        seeds.probe(&frules, &index, &ctx, &mut in_seed);
                        phases.add("seed checks", thread_cpu() - now);
                    }
                    let v0 = thread_cpu();
                    let mut needed = vec![ask_all; slots_n];
                    let mut valued = vec![ask_all; slots_n];
                    let mut key_needed: Vec<(usize, usize)> = Vec::new();
                    if let Some(seeds) = &file_seeds {
                        seeds.requests(&mut needed, &mut valued, &mut key_needed);
                    } else {
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
                    if let Some(seeds) = &mut file_seeds {
                        let now = thread_cpu();
                        let ctx = funnel::Ctx { table: ft, src: &bytes, sets: node_table::element_sets(ft),
                            enc: node_table::enclosing_ids(ft), facts: &store, matches: &interner.matches };
                        seeds.resolve(&frules, &index, &ctx, &mut in_seed);
                        phases.add("seed checks", thread_cpu() - now);
                    } else {
                        prefix!(ft);
                        if fin.is_some() || seed_out.is_some() {
                            sv.seed_of_file(hits0, alive0, &node_class, &mut in_seed);
                        }
                    }
                    prefixed = true;
                    let handoff = thread_cpu();
                    if seed_out.is_some() {
                        use std::fmt::Write as _;
                        for (r, seeded) in in_seed.iter().enumerate() {
                            if *seeded { writeln!(seed_dump, "{file_no}\t{}", frules[r].name).expect("write file seed"); }
                        }
                    }
                    if let Some(fin) = fin.as_deref_mut() {
                        // redo-honest: `VEC_NO_PREFIX` seeds every rule the skip kept, survivors or not (a check of the
                        // prefix: a node-class rule's findings must not depend on its prefix plans finding survivors)
                        if std::env::var_os("VEC_NO_PREFIX").is_some() {
                            for (r, _) in &sv.alive[alive0..] {
                                in_seed[*r as usize] = true;
                            }
                        }
                        let before_wanted = fin.time;
                        let mut wanted = fin.wanted(path, &|e| emit_rules[e].iter().any(|r| in_seed[*r]));
                        if !exact_rows.is_empty() {
                            wanted.retain(|&e| {
                                let [ri] = emit_rules[e].as_slice() else { return true };
                                let Ok(at) = exact_rows.binary_search_by_key(ri, |(r, _)| *r) else { return true };
                                if let Some(n) = fin.emit_exact(file_no, ft, e, &exact_rows[at].1) {
                                    if let Some(log) = &mut exact_log {
                                        use std::io::Write as _;
                                        writeln!(log, "{}\t{}\t{n}", frules[*ri].name, fin.run.paths[file_no]).expect("write exact pair");
                                    }
                                    exact_counts[*ri].0 += 1;
                                    exact_counts[*ri].1 += n;
                                    exact_counts[*ri].2 += u64::from(n > 0);
                                }
                                false
                            });
                        }
                        // wanted() already charges its own time to the second half.
                        if seed_only { phases.add("seed handoff", (thread_cpu() - handoff - (fin.time - before_wanted)).max(0.0)); }
                        for (i, (slot, _)) in ask_of.iter().enumerate() {
                            let t = &store.direct.tables[i];
                            if !asks[i].exists && t.answered { fin.prefix_table(&fact_catalog.names[*slot], t, &interner); }
                        }
                        let prefix = direct_fill::Prefix { asks: &asks, tables: &store.direct.tables, words: &interner };
                        fin.column_file(file_no, path, file, &bytes, &wanted, Some(prefix));
                    } else if seed_only {
                        phases.add("seed handoff", thread_cpu() - handoff);
                    }
                    visit_time += thread_cpu() - p0;
                })
                .is_ok()
        } else if use_facts {
            // the provider's facts are lent only while the walk runs: decide here which files need them (some rule that
            // tests a fact survived the kinds-and-words skip), then fill those relations and write their columns
            ryo_python::fixtures::walking::walked_structured(walk, path, &bytes, reader_needs, &mut table, &mut scratch, structured.then_some(enclosing.as_slice()), &mut |file| {
                let p0 = thread_cpu();
                let ft = file.table;
                let n = node_table::element_count(ft).0;
                prepare!(ft, FileId(0), |rel,words,out| matches!(rel, pair_summary::Source::Catalog(r) if matches!(file.facts.fill(r,languages::Wanted::Whole,words,out),languages::FactAnswer::Answered(_))));
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
            let mut builder = if structured { node_table::structured(&mut table, walk_kinds(walk), &enclosing) } else { node_table::builder(&mut table, walk_kinds(walk)) };
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
            prepare!(t, FileId(0), |_,_,_| false);
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
    if let Some(program) = &pairs {
        for (ri, _) in &program.exact {
            println!("EXACT\t{}\t{}\t{}\t{}", frules[*ri].name, exact_counts[*ri].0, exact_counts[*ri].1, exact_counts[*ri].2);
        }
    }
    println!("pair summary checks {pair_steps} CPU {pair_cpu:.4} s");
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
    if let Some(path) = seed_out { std::fs::write(path, seed_dump).expect("write file seeds"); }
    if let Some(seeds) = file_seeds {
        let c = seeds.counts;
        println!("file seeds: {} elements, {} disjuncts deferred to facts; survivors count one witness per node rule and file", c.elements, c.deferred);
        println!("candidates {} survivors {} steps {} knock-steps 0 rule-files-with-survivors {}", c.candidates, c.witnesses, c.steps, c.witnesses);
    } else {
        let knock_c: u64 = units.iter().enumerate().filter(|(_, u)| u.knock).map(|(i, _)| cands[i]).sum();
        let knock_s: u64 = units.iter().enumerate().filter(|(_, u)| u.knock).map(|(i, _)| surv[i]).sum();
        println!(
            "candidates {} survivors {} steps {steps} knock-steps {knock_steps} (knock-out units: candidates {knock_c} survivors {knock_s}) rule-files-with-survivors {}",
            cands.iter().sum::<u64>(),
            surv.iter().sum::<u64>(),
            sv.hits.len()
        );
    }
    if let Some(path) = flag(args, "--rules-out") {
        let mut w = String::from("rule\tunits\tcandidates\tsurvivors\tsteps\tfiles\n");
        let mut files_of = vec![0u64; frules.len()];
        for (r, _) in &sv.alive {
            files_of[*r as usize] += 1;
        }
        for (ri, r) in frules.iter().enumerate() {
            let mine: Vec<usize> = (0..units.len()).filter(|u| units[*u].rule == ri).collect();
            let c: u64 = mine.iter().map(|u| cands[*u]).sum();
            let s: u64 = mine.iter().map(|u| surv[*u]).sum();
            let k: u64 = mine.iter().map(|u| unit_steps[*u]).sum();
            w.push_str(&format!("{}\t{}\t{c}\t{s}\t{k}\t{}\n", r.name, mine.len(), files_of[ri]));
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
#[path = "../../../../crates/engine/benches/vec/kernels.rs"]
mod kernels;
