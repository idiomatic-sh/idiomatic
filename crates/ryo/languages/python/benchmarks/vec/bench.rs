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
                    let v: Vec<String> = json_input::literals(eq);
                    Lit::Eq(v.into_iter().map(String::into_bytes).collect())
                } else if let Some(re) = lit.strip_prefix("re:") {
                    let v: Vec<String> = json_input::literals(re);
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
        if node_table::field_count(table, languages::FieldCode(f as u16)).0 != 0 {
            cols.field_present[f / 64] |= 1 << (f % 64);
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
        .map(|l| json_input::literal(l))
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
        w.push_str(&format!("text\t{}\t{}\t{c}\n", finish::json_output::quoted(&lits[l]), vocab.kind_names[usize::from(k)]));
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
                let t0 = thread_cpu();
                type_expression::load_lists(finish_args[0]);
                let request_path = args.iter().position(|a| *a == "--request").map(|i| args[i + 1]);
                let mut document = request_path.map(json_input::metadata);
                let context = document.as_mut().map(|d| std::mem::take(&mut d.context)).unwrap_or_default();
                let read = lower::read_pack(std::path::Path::new(finish_args[0]), context);
                println!("start-up pack: read {:.4} parse {:.4} s", read.read, read.parse);
                let same_pack = std::fs::canonicalize(args[4]).ok() == std::fs::canonicalize(finish_args[0]).ok();
                let plan = lower::second_half_plan(&read, same_pack);
                println!("start-up total {:.4} s", thread_cpu() - t0);
                let mut fin = visit::Finisher::new(&plan, &walk, &vocab, &finish_args, request_path.zip(document.as_ref()));
                let workers = document.as_ref().and_then(|doc| doc.workers);
                drop(document);
                run_funnel(&walk, settings, &vocab, &args[2..], Some(&mut fin), Some((finish_args[0], &read)), workers);
                fin.end();
                // The process exits with the immutable plan still in use.
                std::mem::forget(plan);
            } else {
                let workers = flag(&args, "--request").and_then(|p| json_input::metadata(&p).workers);
                run_funnel(&walk, settings, &vocab, &args[2..], None, None, workers);
            }
        }
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
        fact_family: pair_provider::family,
        host_len: finish::host::host_len,
        walk_facts: pair_provider::FACTS,
        text_kinds: vocab.kinds.values().copied().filter(|k|tokens.has_text(*k)).collect(),
        file_sets: ryo_python::fixtures::columns::FILE_SET_FACTS.iter().chain([&provider_packaging::MODULE_IDENTITY_REL]).chain(host_columns::FILE_SETS.iter()).map(|s|(*s).to_string()).collect(),
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
            && let Some(identity) = json_input::configuration(path)
        {
            return ryo_python::settings::PythonParserSettings::from_identity(&identity).unwrap_or_else(|| panic!("parser configuration {identity}"));
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

#[path = "../../../../crates/engine/benches/vec/finish.rs"]
mod finish;
#[path = "../../../../crates/engine/benches/vec/lower.rs"]
mod lower;
use json_input::rule_context;
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
#[path = "../../../../crates/engine/benches/vec/summary_schedule.rs"]
mod summary_schedule;
#[path="../../../../crates/engine/benches/vec/summary_rules.rs"]
mod summary_rules;
#[path="../../../../crates/engine/benches/vec/summary_literals.rs"]
mod summary_literals;
#[path = "../../../../crates/engine/benches/vec/summary_proofs.rs"]
mod summary_proofs;
#[path = "../../../../crates/engine/benches/vec/summary_values.rs"]
mod summary_values;
mod summary_walk;
mod column_pair;
mod source_batch;
mod pair_provider;
#[path = "../../../../crates/engine/benches/vec/host_columns.rs"]
mod host_columns;
mod json_input;
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
#[path = "relative_import.rs"]
mod relative_import;
mod import_syntax;
mod expression_parts;
mod lexical_lines;
#[path = "provider_packaging.rs"]
mod provider_packaging;
#[path = "provider_tokens.rs"]
mod provider_tokens;

thread_local! {
    /// redo-binding pass 5: the implicit-symbol catalog of this run (module position, table), filled once.
    static IMPLICIT_CATALOG: std::cell::RefCell<Option<(u32, finish::Table)>> = const { std::cell::RefCell::new(None) };
}

/// The visit's facts as the finish's provider.
struct ColFacts<'v, 'a, 'f, 'w> {
    file: &'v mut ryo_python::fixtures::columns::ColumnFile<'a, 'f>,
    words: &'w mut languages::FileWords,
    filled: languages::FilledColumns,
    time: f64,
    rows: u64,
    by_rel: HashMap<String, (f64, u64, u64)>,
    file_no: u32,
    /// redo-binding pass 3: the provider relations the harness facts are written from, kept for the file once filled, so
    /// a fact and the steps that read the same relation share one fill.
    import_comments: import_syntax::LineComments,
    kept: HashMap<String, finish::Table>,
    /// redo-general: the provider's project (its packaging knowledge, run-wide) and this file's bytes, for the relations it
    /// answers for a project.
    project: &'w mut provider_packaging::Project,
    src: &'w [u8],
}

impl<'v, 'a, 'f, 'w> ColFacts<'v, 'a, 'f, 'w> {
    fn for_visit(
        file: &'v mut ryo_python::fixtures::columns::ColumnFile<'a, 'f>,
        file_no: usize,
        src: &'w [u8],
        words: &'w mut languages::FileWords,
        project: &'w mut provider_packaging::Project,
        kept: HashMap<String, finish::Table>,
    ) -> Self {
        Self {
            file,
            file_no: file_no as u32,
            src,
            words,
            project,
            kept,
            import_comments: Default::default(),
            filled: Default::default(),
            time: 0.0,
            rows: 0,
            by_rel: HashMap::new(),
        }
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// A catalog relation of this file, filled whole (once a file for the relations the harness facts read).
    fn whole(&mut self, run: &mut finish::Run, rel: &str) -> Result<finish::Table, String> {
        if let Some(t) = self.kept.get(rel) {
            return Ok(t.clone());
        }
        // redo-binding pass 5: the implicit-symbol catalog is the same for every file whose module node is the same (the
        // rows name the module scope and the file only; the file column is set by `fill`), so it is filled once a run
        let module = (rel == "implicit-symbol").then(|| self.module_position(run)).flatten();
        if let Some(m) = module
            && let Some(t) = IMPLICIT_CATALOG.with(|c| c.borrow().as_ref().filter(|(at, _)| *at == m).map(|(_, t)| t.clone()))
        {
            // the rows name the module scope and the file: the cached rows' file cells become this file's
            let mut t = t;
            let f = u64::from(self.file_no);
            for c in 0..t.tys.len() {
                if t.tys[c] == finish::Ty::File {
                    t.cols[c].iter_mut().for_each(|v| *v = f);
                }
            }
            let _ = self.kept.insert(rel.to_owned(), t.clone());
            return Ok(t);
        }
        let t = self.whole_fill(run, rel)?;
        if let Some(m) = module {
            IMPLICIT_CATALOG.with(|c| *c.borrow_mut() = Some((m, t.clone())));
        }
        if matches!(rel, "binding-event" | "binding-read" | "binding-resolution" | "qualified-name" | "call-argument" | "identifier" | "embedded-expression" | "compiler-feature" | "language-target" | "lookup-scope" | "implicit-symbol") {
            let _ = self.kept.insert(rel.to_owned(), t.clone());
        }
        Ok(t)
    }

    /// The position of the file's module node (the scope the implicit-symbol rows name).
    fn module_position(&self, run: &finish::Run) -> Option<u32> {
        let n = node_table::element_count(self.file.table).0 as u32;
        (0..n).find(|p| &*run.texts[run.kind_sym[usize::from(node_table::kind_of(self.file.table, Position(*p)).0)] as usize] == b"module")
    }

    fn whole_fill(&mut self, run: &mut finish::Run, rel: &str) -> Result<finish::Table, String> {
        let timed = finish::fill_prof::timed();
        // redo-fills pass 2: the provider's direct columns, no reader rows and no conversion (direct_fill.rs)
        if direct_fill::direct(rel)
            && let Some((t, write, make)) = direct_fill::fill(self.file, run, rel, self.file_no, timed)
        {
            if timed {
                let n = t.as_ref().map_or(0, |t| t.n as u64);
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
                direct_fill::check(run, rel, self.file_no, &r, t.as_ref().map_err(String::clone)?);
            }
            return t;
        }
        let (relation, _, _) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
        let relation = relation.ok_or_else(|| format!("{rel}: no reader relation"))?;
        self.filled.0.clear();
        let t0 = if timed { thread_cpu() } else { 0.0 };
        let answer = self.file.fill(relation, languages::Wanted::Whole, self.words, &mut self.filled);
        let t1 = if timed { thread_cpu() } else { 0.0 };
        let t = match answer {
            languages::FactAnswer::Answered(_) => finish::filled_table(run, rel, &self.filled, self.words),
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
    fn type_expression(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
        let qualified = self.whole(run, "qualified-name")?;
        let embedded = self.whole(run, "embedded-expression")?;
        // redo-fills pass 2: no root, no row, and the other two inputs are not filled
        if !type_expression::any_root(run, self.file.table, &qualified, embedded.n, type_expression::LISTS.get_or_init(type_expression::Lists::default))? {
            let (_, names, tys) = run.catalog(fact_catalog::TYPE_EXPRESSION_REL).ok_or("no type-expression")?;
            return Ok(finish::Table { names, tys, cols: vec![Vec::new(); 3], n: 0 });
        }
        let arguments = self.whole(run, "call-argument")?;
        let identifiers = self.whole(run, "identifier")?;
        let inputs = type_expression::Inputs { qualified: &qualified, arguments: &arguments, identifiers: &identifiers, embedded: &embedded, lists: type_expression::LISTS.get_or_init(type_expression::Lists::default) };
        let rows = type_expression::rows(run, self.file.table, &inputs)?;
        let (_, names, tys) = run.catalog(fact_catalog::TYPE_EXPRESSION_REL).ok_or("no type-expression")?;
        let (ty, value) = (u64::from(run.sym(b"type")), u64::from(run.sym(b"value")));
        let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(rows.len()); 3], n: rows.len() };
        for (id, typed) in rows {
            t.cols[0].push(u64::from(id));
            t.cols[1].push(if typed { ty } else { value });
            t.cols[2].push(u64::from(self.file_no));
        }
        Ok(t)
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// redo-binding pass 4: `annotation-evaluation` (annotation_evaluation.rs) from the node table, the file's compiler features
    /// and target, and its embedded expressions.
    fn annotation_evaluation(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
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
        let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(rows.len()); 5], n: rows.len() };
        for (id, root, evaluation, mode) in rows {
            let (e, m) = (u64::from(run.sym(evaluation)), u64::from(run.sym(mode)));
            for (c, v) in [u64::from(id), u64::from(root), e, m, u64::from(self.file_no)].into_iter().enumerate() {
                t.cols[c].push(v);
            }
        }
        Ok(t)
    }
}

impl ColFacts<'_, '_, '_, '_> {
    /// redo-binding pass 4: a harness fact, made once a file (the facts that compose others read it from here).
    fn fact(&mut self, run: &mut finish::Run, rel: &str) -> Result<finish::Table, String> {
        if let Some(t) = self.kept.get(rel) {
            return Ok(t.clone());
        }
        let timed = finish::fill_prof::timed();
        let t0 = if timed { thread_cpu() } else { 0.0 };
        let t = match rel {
            fact_catalog::TYPE_EXPRESSION_REL => self.type_expression(run)?,
            fact_catalog::ANNOTATION_EVALUATION_REL => self.annotation_evaluation(run)?,
            fact_catalog::FINAL_REL => self.namespace_final(run)?,
            fact_catalog::DEFERRED_RESOLUTION_REL => self.deferred_resolution(run)?,
            _ => self.deferred_visit(run)?,
        };
        if timed {
            let e = self.by_rel.entry(format!("fact:{rel}")).or_default();
            e.0 += thread_cpu() - t0;
            e.1 += t.n as u64;
            e.2 += 1;
        }
        let _ = self.kept.insert(rel.to_owned(), t.clone());
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
    fn deferred_visit(&mut self, run: &mut finish::Run) -> Result<finish::Table, String> {
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
            finish::Table { names, tys, cols: vec![Vec::new(); 3], n: 0 }
        };
        let inputs = deferred_visit::Inputs { types: &types, evaluations: &evaluations, embedded: &embedded, stub, future };
        let rows = deferred_visit::rows(run, self.file.table, &inputs)?;
        let (_, names, tys) = run.catalog(fact_catalog::DEFERRED_VISIT_REL).ok_or("no deferred-visit")?;
        let deferred = u64::from(run.sym(b"deferred"));
        let mut t = finish::Table { names, tys, cols: vec![Vec::with_capacity(rows.len()); 3], n: rows.len() };
        for id in rows {
            t.cols[0].push(u64::from(id));
            t.cols[1].push(deferred);
            t.cols[2].push(u64::from(self.file_no)); // redo-fills pass 2: the supplier writes the file its rows name
        }
        Ok(t)
    }
}

impl finish::Provider for ColFacts<'_, '_, '_, '_> {
    fn fill_len(&mut self, run: &mut finish::Run, rel: &str, scope: finish::Scope<'_>) -> Option<usize> {
        if !matches!(scope, finish::Scope::File(f) if f == self.file_no) || self.kept.contains_key(rel) {
            return None;
        }
        if matches!(rel, import_syntax::HEADS | import_syntax::LINE_COMMENTS) {
            let timed = finish::fill_prof::timed();
            let start = if timed { thread_cpu() } else { 0.0 };
            let count = if rel == import_syntax::HEADS {
                Some(self.file.import_statement_count())
            } else {
                self.import_comments.known_count().or_else(|| (!self.file.can_have_import_line_comments()).then_some(0))
            };
            if timed {
                let elapsed = thread_cpu() - start;
                let kind = if count == Some(0) { "empty" } else { "other" };
                let e = self.by_rel.entry(format!("length-{kind}:{rel}")).or_default();
                e.0 += elapsed;
                e.2 += 1;
            }
            return count;
        }
        let (relation, _, _) = run.catalog(rel)?;
        self.file.row_count(relation?)
    }

    fn fill(&mut self, run: &mut finish::Run, rel: &str, scope: finish::Scope<'_>) -> Result<finish::Table, String> {
        // redo-general: a project's relations from the provider's project (its packaging knowledge)
        let file = match scope {
            finish::Scope::File(file) => file,
            finish::Scope::Files(files) => {
                let start = finish::fill_prof::timed().then(thread_cpu);
                let result = self.project.fill(run, rel, files, self.file_no, self.file.table, self.src);
                if let (Some(start), Ok(table)) = (start, &result) {
                    let stat = self.by_rel.entry(rel.to_owned()).or_default();
                    stat.0 += thread_cpu() - start;
                    stat.1 += table.n as u64;
                    stat.2 += 1;
                }
                return result;
            }
        };
        if file != self.file_no {
            return Err(format!("{rel}: asked for file {file}, this provider holds file {}", self.file_no));
        }
        // The syntax walk indexed every relative form. Report the fixed cost of a proven empty answer separately.
        if rel == fact_catalog::RELATIVE_IMPORT_REL && !self.file.has_relative_imports() {
            let timed = finish::fill_prof::timed();
            let start = if timed { thread_cpu() } else { 0.0 };
            let (_, names, tys) = run.catalog(rel).ok_or("no relative-import-target")?;
            let table = finish::Table::with_rows(names, tys, 0);
            if timed {
                let e = self.by_rel.entry("empty-proof:relative-import-target".to_owned()).or_default();
                e.0 += thread_cpu() - start;
                e.2 += 1;
            }
            return Ok(table);
        }
        let timed = finish::fill_prof::timed();
        let t0 = if timed { thread_cpu() } else { 0.0 };
        let mut t = if rel == fact_catalog::REGION_REL {
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
                t.cols[5].push(u64::from(self.file_no));
            }
            t
        } else if rel == fact_catalog::RELATIVE_IMPORT_REL {
            relative_import::walked(run, self.project, self.file, self.file_no)?
        } else if matches!(rel, lexical_lines::COMMENT | lexical_lines::MULTILINE | lexical_lines::BREAKS) {
            lexical_lines::fill(run, self.file, self.src, self.file_no, rel)?
        } else if matches!(
            rel,
            expression_parts::CALL_SITE
                | expression_parts::BINARY_OPERANDS
                | expression_parts::BOOLEANS
                | expression_parts::WITH_HEADER
                | expression_parts::COMPARISONS
                | expression_parts::DICT_KEYS
                | expression_parts::ASSIGN_NAMES
                | expression_parts::NESTED_DELIMITERS
                | expression_parts::EXPR_STATEMENTS
                | expression_parts::CONSTANT_PREDECESSOR
                | expression_parts::ANNOTATION_STRINGS
                | expression_parts::SUBSCRIPT
                | expression_parts::SCHEDULE_END
                | expression_parts::POSITIONAL_ARGUMENT
                | expression_parts::ATTRIBUTE_PREFIX
                | expression_parts::DISPLAY_ELEMENT
        ) {
            if rel == expression_parts::NESTED_DELIMITERS && let Some(rows) = self.kept.remove(rel) {
                rows
            } else {
                expression_parts::fill(run, self.file, self.src, self.file_no, rel)?
            }
        } else if matches!(rel, import_syntax::NEIGHBORS | import_syntax::HEADS | import_syntax::STATEMENTS | import_syntax::ALIASES | import_syntax::CONTEXTUAL_ALIASES | import_syntax::LINE_COMMENTS | import_syntax::ENTRIES) {
            import_syntax::fill(run, self.file, self.src, self.project, self.file_no, rel, &mut self.import_comments)?
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
            rows
        } else if matches!(rel, fact_catalog::ASSIGNMENT_SOURCE_REL | fact_catalog::NAMESPACE_SITES_REL | fact_catalog::BINDING_READ_REL) {
            let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no {rel}"))?;
            let mut rows = finish::Table::with_rows(names, tys, 0);
            let complete = pair_provider::sites::visit(rel, self.file, u64::from(self.file_no),
                |word| u64::from(run.sym(word.as_bytes())), |values| {
                    for (col, &value) in rows.cols.iter_mut().zip(values) { col.push(value); }
                    rows.n += 1;
                });
            if !complete { return Err(format!("{rel}: incomplete flow")); }
            rows
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
            rows
        } else if rel == fact_catalog::PREDECESSOR_REL {
            self.scheduled_read_predecessor(run)?
        } else if rel == fact_catalog::RESOLUTION_REL {
            self.read_resolution(run)?
        } else if rel == fact_catalog::TYPE_REGION_REL {
            let ids = self.file.type_regions().ok_or("type-region: the model is incomplete")?;
            let (_, names, tys) = run.catalog(rel).ok_or("no type-region")?;
            finish::Table { names, tys, n: ids.len(), cols: vec![ids.iter().map(|i| u64::from(*i)).collect(), vec![u64::from(self.file_no); ids.len()]] }
        } else if rel == fact_catalog::FINAL_REL
            || rel == fact_catalog::TYPE_EXPRESSION_REL
            || rel == fact_catalog::ANNOTATION_EVALUATION_REL
            || rel == fact_catalog::DEFERRED_VISIT_REL
            || rel == fact_catalog::DEFERRED_RESOLUTION_REL
        {
            self.fact(run, rel)?
        } else {
            self.whole(run, rel)?
        };
        let dt = if timed { thread_cpu() - t0 } else { 0.0 };
        self.time += dt;
        self.rows += t.n as u64;
        let e = self.by_rel.entry(rel.to_owned()).or_default();
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
        finish::fill_prof::dump(run, self.file_no, rel, &t);
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

/// Finite, nonempty literal alternatives in the plan's source-matching expressions.
fn source_literals(plan: &finish::Plan) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    for n in plan.nodes.iter().filter(|n| n.op == "source-matches") {
        let Some(p) = n.args()["pattern"].as_str() else { continue };
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

/// Prepare the file funnel from a rule pack. A finish stage over the same pack reuses its parsed programs.
fn run_funnel(walk: &impl Walk, settings: ryo_python::settings::PythonParserSettings, vocab: &Vocab, args: &[&str], mut fin: Option<&mut visit::Finisher<'_>>, pack: Option<(&str, &lower::ReadPack)>, workers: Option<std::num::NonZeroUsize>) {
    let way = args[0];
    let list = files(args[1]);
    let f0 = thread_cpu();
    let no_skip = std::env::var_os("VEC_NO_SKIP").is_some();
    let mut pairs = None;
    let mut pair_steps = 0;
    let mut pair_cpu = 0.0;
    let mut exact_rows = Vec::new();
    let (rules, frules, words) = {
        let owned;
        let read = match pack {
            Some((dir, read)) if std::fs::canonicalize(dir).ok() == std::fs::canonicalize(args[2]).ok() => read,
            _ => {
                let request = flag(args, "--request");
                owned = lower::read_pack(std::path::Path::new(args[2]), json_input::rule_context(request.as_deref()));
                println!("start-up first half pack: read {:.4} parse {:.4} s", owned.read, owned.parse);
                &owned
            }
        };
        let f1 = thread_cpu();
        // A separately loaded finish plan may use the same rule names for
        // different emits. Only the shared typed plan certifies its output.
        let shared_plan = read.typed.borrow().is_some();
        let mut prepared = first::prepare(read, python_language(vocab), vocab);
        if !shared_plan {
            prepared.pairs.exact.clear();
        }
        let f2 = thread_cpu();
        let rules = rules_of_skips(&prepared.skips, vocab);
        let (frules, words) = funnel::build(&prepared.rules);
        pairs = Some(prepared.pairs);
        let f3 = thread_cpu();
        println!(
            "start-up first half: from the pack: analysis {:.4} load {:.4} total {:.4} s; rules {}",
            f2 - f1, f3 - f2, f3 - f0, frules.len()
        );
        (rules, frules, words)
    };
    let use_facts = args.contains(&"--facts");
    let direct = args.contains(&"--direct");
    let mut cwalk = ryo_python::fixtures::columns::ColumnWalk::with_settings(
        ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names"),
        settings,
    );
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
    let byte_patterns: Vec<Vec<u8>> = if word_src == "bytes6" {
        fin.as_deref().map(|f| source_literals(f.plan())).unwrap_or_default().into_iter().take(64).collect()
    } else {
        Vec::new()
    };
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
    let by_name: HashMap<&str, usize> = rules.iter().enumerate().map(|(i, r)| (r.name.as_str(), i)).collect();
    let skip_of: Vec<Option<usize>> = frules.iter().map(|r| by_name.get(r.name.as_str()).copied()).collect();
    // the kinds-and-fields skip of each plan rule, flat: a clause is its kind words then its field words
    let mut skip = Skip::new(skip_of.iter().map(|i| i.map_or((false, &[][..]), |i| (rules[i].never, rules[i].clauses.as_slice()))), frules.iter().map(|r| r.cnfs.as_slice()), words_of(vocab.kind_names.len()), words_of(vocab.field_names.len()));
    let mut kinds: Vec<u64> = Vec::new();
    let fields_width = vocab.field_names.len();
    let structured = matches!(way, "structure" | "spec" | "knock");
    // the table writes the structure the checks read (element sets, the plans' enclosing columns) as ids are numbered
    let enclosing = Vec::new();
    let mut phases = Phases::default();
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let mut cols = Columns::default();
    let (mut kinds_alive, mut words_alive, mut alive): (Vec<bool>, Vec<bool>, Vec<bool>) = (Vec::new(), Vec::new(), Vec::new());
    // one pass: `--dump <file>` writes per file a hash of each part the first half reads (dump.rs)
    let dump_out = flag(args, "--dump");
    let mut dumped = String::new();
    let alive_out = flag(args, "--alive-out");
    let rules_out = flag(args, "--rules-out");
    let mut alive_pairs = Vec::new();
    let mut files_per_rule = vec![0u64; frules.len()];
    let carry_file = (fin.is_some() || args.contains(&"--seed-out"))
        && way == "spec" && direct && use_facts && word_src == "table" && word_skip
        && !no_skip && !compare && dump_out.is_none() && alive_out.is_none() && rules_out.is_none();
    let prepare_summary = thread_cpu();
    let mut query = pairs.as_ref().map(|program| {
        let (file_words,codes) = summary_values::vocabulary(walk,program);
        let schedule = summary_schedule::Schedule::new(program,Some((&words.ids,name_token)),codes);
        (program,schedule,file_words,pair_checks::Scratch::default())
    });
    if query.is_some() {println!("summary evaluation preparation {:.6} s",thread_cpu()-prepare_summary);}
    if carry_file && let Some((program,schedule,words_for_file,scratch))=&mut query {
        let root = fin.as_ref().map(|f| f.run.root_dir.clone()).or_else(|| flag(args, "--summary-root"));
        let parser_words = ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names");
        let mut other = ryo_python::fixtures::columns::ColumnWalk::with_settings(parser_words.fresh(), settings);
        summary_walk::run(&list, settings, vocab, &parser_words, workers, program,&mut skip,&words,fields_width,words_for_file,
            &mut cwalk,&mut other,fin,root.as_deref(),flag(args,"--seed-out").as_deref(),flag(args,"--exact-out").as_deref(),schedule,scratch);
        return;
    }
    let mut summary_names = vec![0u64;words.names.len().div_ceil(64)];
    if structured { cwalk.structured(enclosing.clone()); }
    let seed_out = flag(args, "--seed-out");
    let mut seed_dump = String::new();
    let started = thread_cpu();
    // the second half inside this walk (`--finish`): each plan emit's rules of this plan set by name (the seed's rule is a
    // name), and the file's seed as a set over the rules
    assert!(fin.is_none() || (use_facts && direct), "--finish in the first half's walk needs --facts --direct");
    let emit_rules: Vec<Vec<usize>> = fin.as_ref().map(|f| {
        let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, rule) in frules.iter().enumerate() {
            by_name.entry(&rule.name).or_default().push(i);
        }
        f.emit_names().map(|(_, name)| by_name.get(name).cloned().unwrap_or_default()).collect()
    }).unwrap_or_default();
    let mut wanted = Vec::new();
    let mut exact_counts = vec![(0u64, 0u64, 0u64); frules.len()];
    let mut exact_log = flag(args, "--exact-out").map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("exact pairs")));
    let fin0 = fin.as_ref().map_or(0.0, |f| f.time);
    println!("start-up to first file {:.4} s", thread_cpu());
    for (file_no, path) in list.iter().enumerate() {
        let c0 = thread_cpu();
        let bytes = std::fs::read(path).expect("read");
        let mut visit_time = 0f64;
        // the file's skip, made once (inside the walk when the provider's facts are decided there, else after it): the
        // columns read, the required words present, and which rules can match by kinds and fields and by words
        let mut prepared = false;
        let mut codes_taken = false;
        macro_rules! prepare {
            ($t:expr,$fill:expr) => {{
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
                if !no_skip && let Some((program,schedule,words_for_file,scratch)) = &mut query {
                    let start = thread_cpu();
                    let relative_path = fin.as_deref().map(|f| f.run.paths[file_no].clone());
                    let exact = fin.as_ref().filter(|f| f.can_finish_exact()).map(|_| &mut exact_rows);
                    schedule.literals.names(pt,&bytes,&mut summary_names,&mut scratch.literals);
                    pair_steps += program.eval(scratch,schedule,pt, &bytes, relative_path.as_deref().map(str::as_bytes), None, words_for_file, |rel, request, words, out| {
                        if !matches!(request,pair_checks::Query::Rows){return None;}
                        if matches!(rel, pair_summary::Source::Catalog(languages::Relation::ModuleIdentity)) {
                            return Some(fin.as_deref_mut().is_some_and(|f| f.identity(file_no, words, out)));
                        }
                        if let pair_summary::Source::Catalog(catalog) = rel
                            && let Some(result) = host_columns::fill(catalog, fin.as_deref().map(|f| &f.run), file_no, out) {
                            return Some(result);
                        }
                        Some(($fill)(rel, words, out))
                    }, &mut alive, exact);
                    pair_cpu += thread_cpu() - start;
                }
                rule_files += frules.len() as u64;
                alive_kinds += kinds_alive.iter().filter(|a| **a).count() as u64;
                alive_both += alive.iter().filter(|a| **a).count() as u64;
                if alive_out.is_some() || rules_out.is_some() {
                    for (ri, can_run) in alive.iter().enumerate() {
                        if *can_run {
                            files_per_rule[ri] += 1;
                            if alive_out.is_some() { alive_pairs.push((ri, file_no)); }
                        }
                    }
                }
                phases.add("skip tests", thread_cpu() - c3);
                }
            }};
        }
        macro_rules! dump_table {
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
                }
            }};
        }
        let mut dumped_table = false;
        let ok = if use_facts && direct {
            cwalk
                .walked(path, &bytes, &mut table, &mut |file| {
                    let p0 = thread_cpu();
                    let ft = file.table;
                    file.name_token_codes(&mut name_codes);
                    codes_taken = true;
                    let mut walk_tables = HashMap::new();
                    prepare!(ft, |rel,words,out| {
                        if let pair_summary::Source::Walk(name) = rel
                            && pair_provider::visit_table(name) {
                            fin.as_deref_mut().is_some_and(|f| f.walk_table(file_no, file, &bytes, name, words, out, &mut walk_tables))
                        } else {
                            pair_provider::read(file, file_no, rel, words, out)
                        }
                    });
                    prepared = true;
                    if dump_out.is_some() {
                        dump_table!(ft);
                        dumped_table = true;
                    }
                    let handoff = thread_cpu();
                    if seed_out.is_some() {
                        use std::fmt::Write as _;
                        for (r, seeded) in alive.iter().enumerate() {
                            if *seeded { writeln!(seed_dump, "{file_no}\t{}", frules[r].name).expect("write file seed"); }
                        }
                    }
                    if let Some(fin) = fin.as_deref_mut() {
                        let before_wanted = fin.time;
                        fin.wanted_into( |e| emit_rules[e].iter().any(|r| alive.get(*r) == Some(&true)), &mut wanted);
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
                        phases.add("seed handoff", (thread_cpu() - handoff - (fin.time - before_wanted)).max(0.0));
                        if fin.visits(&wanted) {
                            fin.column_file(file_no, file, &bytes, &wanted, walk_tables);
                        }
                    } else {
                        phases.add("seed handoff", thread_cpu() - handoff);
                    }
                    visit_time += thread_cpu() - p0;
                })
                .is_ok()
        } else if use_facts {
            ryo_python::fixtures::walking::walked_structured(walk, path, &bytes, &[], &mut table, &mut scratch, structured.then_some(enclosing.as_slice()), &mut |file| {
                let p0 = thread_cpu();
                prepare!(file.table, |rel, words, out| matches!(rel, pair_summary::Source::Catalog(r) if matches!(file.facts.fill(r, languages::Wanted::Whole, words, out), languages::FactAnswer::Answered(_))));
                prepared = true;
                visit_time += thread_cpu() - p0;
            }).is_ok()
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
            prepare!(t, |_,_,_| false);
        }
        if dump_out.is_some() && !dumped_table {
            dump_table!(t);
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
    println!("skip: rule-files {rule_files} alive by kinds {alive_kinds} by kinds and words {alive_both}");
    if let Some(program) = &pairs {
        for (ri, _) in &program.exact {
            println!("EXACT\t{}\t{}\t{}\t{}", frules[*ri].name, exact_counts[*ri].0, exact_counts[*ri].1, exact_counts[*ri].2);
        }
    }
    if let Some((p, _, _, scratch)) = &query && let Some(meter) = &scratch.meter {
        meter.report(p);
    }
    println!("pair summary checks {pair_steps} CPU {pair_cpu:.4} s");
    println!("thread CPU total {total:.4} s");
    for (n, s) in &phases.0 {
        println!("phase {n}: {s:.4} s");
    }
    if let Some(path) = seed_out { std::fs::write(path, seed_dump).expect("write file seeds"); }
    if let Some(path) = rules_out {
        let mut rows = String::from("rule\tunits\tcandidates\tsurvivors\tsteps\tfiles\n");
        for (r, files) in frules.iter().zip(files_per_rule) {
            use std::fmt::Write as _;
            writeln!(rows, "{}\t0\t0\t0\t0\t{files}", r.name).unwrap();
        }
        std::fs::write(path, rows).expect("rules out");
    }
    if let Some(path) = alive_out {
        use std::fmt::Write as _;
        let mut rows = String::new();
        for (rule, file) in alive_pairs {
            writeln!(rows, "{}\t{}", frules[rule].name, list[file]).unwrap();
        }
        std::fs::write(path, rows).expect("alive out");
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
