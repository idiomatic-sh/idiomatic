//! Engine bake-off prototype (bead ryo-i8o1, throwaway): the fast-linter architecture measured with nothing between the parser's
//! tree and the rules. `walk <mode> <request.json> <root>`; modes: `read`, `parse`, `table`, `sema`, `provider-build`,
//! `provider-bind`, `rules <program.json>`. Four worker threads over the file list, one atomic cursor.

#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    missing_docs,
    unused_results,
    dead_code,
    unused,
    reason = "throwaway bake-off prototype, never landed (bead ryo-i8o1)"
)]

mod builtins;
mod derived;
mod model;
mod rows;
mod rules;
mod sema;
mod symbols;
mod table;
mod view;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub(crate) fn options() -> ruff_python_parser::ParseOptions {
    ruff_python_parser::ParseOptions::from(ruff_python_ast::PySourceType::Python)
        .with_target_version(ruff_python_ast::PythonVersion::PY310)
}

#[derive(Default)]
pub(crate) struct Totals {
    pub bytes: AtomicU64,
    pub nodes: AtomicU64,
    pub bindings: AtomicU64,
    pub resolved: AtomicU64,
    pub scopes: AtomicU64,
    pub kinds_present: AtomicU64,
    pub extra: [AtomicU64; 8],
    pub rows: [AtomicU64; 11],
    /// thread-ns per stage: 0 parse, 1 table, 2 lexical model, 3 flow model, 4 policy, 5.. rows per relation, 16 lexical resolution of every name load
    pub ns: [AtomicU64; 17],
    /// derived relations: rows and thread-ns per relation (derived::RELATIONS order)
    pub drows: [AtomicU64; 32],
    pub dns: [AtomicU64; 32],
    pub touch: AtomicU64,
}

/// Thread CPU time (CLOCK_THREAD_CPUTIME_ID): a stage's own CPU, not counting time the thread is preempted.
struct Clock(u64);
impl Clock {
    fn now() -> Self {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        Self(t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64)
    }
    fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(Self::now().0 - self.0)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).cloned().unwrap_or_default();
    let request: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("request")).expect("json");
    let root = std::path::PathBuf::from(&args[3]);
    let files: Vec<String> = request["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| f.as_str().unwrap().to_owned())
        .collect();
    let threads: usize = std::env::var("WALK_THREADS").ok().and_then(|t| t.parse().ok()).unwrap_or(4);
    let program = if mode == "rules" {
        Some(rules::Program::load(&args[4]))
    } else {
        None
    };
    let cursor = AtomicUsize::new(0);
    let totals = Totals::default();
    let outputs: Vec<Vec<String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                std::thread::Builder::new().stack_size(256 << 20).spawn_scoped(scope, || {
                    let mut out = Vec::new();
                    let mut worker = Worker::new(&mode, program.as_ref());
                    worker.root = root.clone();
                    loop {
                        let index = cursor.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = files.get(index) else { break };
                        let bytes = std::fs::read(root.join(path)).expect("read");
                        totals.bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                        worker.file(index, path, &bytes, &totals, &mut out);
                    }
                    worker.finish(&totals);
                    out
                })
                .expect("spawn")
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("worker")).collect()
    });
    if let Some(dir) = mode.strip_prefix("ddump:") {
        let mut files: Vec<std::io::BufWriter<std::fs::File>> = derived::RELATIONS
            .iter()
            .map(|r| std::io::BufWriter::new(std::fs::File::create(format!("{dir}/{r}.tsv")).expect("create")))
            .collect();
        use std::io::Write as _;
        for line in outputs.iter().flatten() {
            let (i, rest) = line.split_once('\t').expect("tag");
            writeln!(files[i.parse::<usize>().unwrap()], "{rest}").ok();
        }
    }
    if mode == "derived" || mode.starts_with("ddump:") {
        let rows: Vec<String> = derived::RELATIONS
            .iter()
            .enumerate()
            .map(|(i, r)| format!("\"{r}\":[{},{}]", totals.drows[i].load(Ordering::Relaxed), totals.dns[i].load(Ordering::Relaxed)))
            .collect();
        eprintln!(
            "{{\"mode\":\"{mode}\",\"ns\":{:?},\"files\":{},\"nodes\":{},\"touch\":{},\"relations\":{{{}}}}}",
            totals.ns.iter().map(|x| x.load(Ordering::Relaxed)).collect::<Vec<_>>(),
            files.len(),
            totals.nodes.load(Ordering::Relaxed),
            totals.touch.load(Ordering::Relaxed),
            rows.join(",")
        );
        return;
    }
    if let Some(dir) = mode.strip_prefix("dump:") {
        // per-relation files, lines tagged "<relation index>\t" by the workers
        let mut files: Vec<std::io::BufWriter<std::fs::File>> = rows::RELATIONS
            .iter()
            .map(|r| std::io::BufWriter::new(std::fs::File::create(format!("{dir}/{r}.tsv")).expect("create")))
            .collect();
        use std::io::Write as _;
        for line in outputs.iter().flatten() {
            let (i, rest) = line.split_once('\t').expect("tag");
            writeln!(files[i.parse::<usize>().unwrap()], "{rest}").ok();
        }
        let r: Vec<u64> = totals.rows.iter().map(|x| x.load(Ordering::Relaxed)).collect();
        eprintln!("{{\"mode\":\"dump\",\"rows\":{r:?}}}");
        return;
    }
    let mut lines: Vec<String> = outputs.into_iter().flatten().collect();
    lines.sort();
    use std::io::Write as _;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for line in &lines {
        writeln!(lock, "{line}").ok();
    }
    let e: Vec<u64> = totals.extra.iter().map(|x| x.load(Ordering::Relaxed)).collect();
    eprintln!(
        "{{\"mode\":\"{mode}\",\"ns\":{:?},\"rows\":{:?},\"files\":{},\"bytes\":{},\"nodes\":{},\"scopes\":{},\"bindings\":{},\"resolved\":{},\"kinds_present\":{},\"extra\":{:?},\"findings\":{}}}",
        totals.ns.iter().map(|x| x.load(Ordering::Relaxed)).collect::<Vec<_>>(),
        totals.rows.iter().map(|x| x.load(Ordering::Relaxed)).collect::<Vec<_>>(),
        files.len(),
        totals.bytes.load(Ordering::Relaxed),
        totals.nodes.load(Ordering::Relaxed),
        totals.scopes.load(Ordering::Relaxed),
        totals.bindings.load(Ordering::Relaxed),
        totals.resolved.load(Ordering::Relaxed),
        totals.kinds_present.load(Ordering::Relaxed),
        e,
        lines.len()
    );
}

struct Worker<'p> {
    mode: String,
    sema: sema::Sema,
    model: model::Model,
    scratch: String,
    rules: Option<rules::State<'p>>,
    provider: Option<ProviderState>,
    root: std::path::PathBuf,
}

struct ProviderState {
    linked: ryo::languages::Language,
    elements: ryo::languages::Elements,
    symbols: ryo::languages::Symbols,
    wanted: Vec<ryo::languages::Relation>,
}

impl<'p> Worker<'p> {
    fn new(mode: &str, program: Option<&'p rules::Program>) -> Self {
        let provider = mode.starts_with("provider").then(|| {
            let wanted = if mode == "provider-bind" {
                ryo::languages::all()
                    .iter()
                    .copied()
                    .filter(|r| {
                        let name = format!("{r:?}");
                        ["BindingEvent", "BindingRead", "BindingUse", "BindingResolution", "BindingShadow"].contains(&name.as_str())
                    })
                    .collect()
            } else {
                Vec::new()
            };
            ProviderState {
                linked: ryo_python::linked(),
                elements: ryo::languages::Elements::default(),
                symbols: ryo::languages::Symbols::default(),
                wanted,
            }
        });
        Self {
            mode: mode.to_owned(),
            sema: sema::Sema::default(),
            model: model::Model::default(),
            scratch: String::new(),
            rules: program.map(rules::State::new),
            provider,
            root: std::path::PathBuf::new(),
        }
    }

    fn file(&mut self, index: usize, path: &str, bytes: &[u8], totals: &Totals, out: &mut Vec<String>) {
        if let Some(provider) = &mut self.provider {
            let engine = provider.linked.parse_engine();
            let request = ryo::languages::ParseRequest {
                path: ryo::languages::SourcePath(path),
                file: ryo::languages::FileId(index as u32),
                bytes: ryo::languages::ByteSlice(bytes),
                notes: ryo::languages::NoteRequest::Unwanted,
                fields: ryo::languages::FieldRequest::Unwanted,
            };
            let mut discard = ryo::languages::contract::Discard::default();
            let result = ryo::languages::contract::read(
                engine.as_ref(),
                request,
                &mut provider.elements,
                &mut provider.symbols,
                &provider.wanted,
                &mut discard,
            );
            if result.is_ok() {
                totals.nodes.fetch_add(ryo::languages::contract::element_count(&provider.elements).0 as u64, Ordering::Relaxed);
            }
            return;
        }
        if self.mode == "read" {
            return;
        }
        let Ok(source) = std::str::from_utf8(bytes) else { return };
        let t_parse = Clock::now();
        let parsed = ruff_python_parser::parse_unchecked(source, options());
        totals.ns[0].fetch_add(t_parse.elapsed().as_nanos() as u64, Ordering::Relaxed);
        if self.mode == "parse" {
            totals.nodes.fetch_add(parsed.tokens().len() as u64, Ordering::Relaxed);
            return;
        }
        let t_table = Clock::now();
        let mut table = table::Table::default();
        table.build(parsed.syntax());
        totals.ns[1].fetch_add(t_table.elapsed().as_nanos() as u64, Ordering::Relaxed);
        totals.nodes.fetch_add(table.len() as u64, Ordering::Relaxed);
        totals.kinds_present.fetch_add(u64::from(table.present.count_ones()), Ordering::Relaxed);
        if self.mode == "table" {
            return;
        }
        if self.mode == "derived" || self.mode.starts_with("ddump:") {
            let incomplete = !parsed.errors().is_empty();
            let t = Clock::now();
            self.model.build_lexical(&table, incomplete);
            totals.ns[2].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
            let full = self.root.join(path);
            let ctx = derived::Ctx { path: &full, table: &table, source, model: &self.model, incomplete };
            let wanted: u64 = std::env::var("DERIVED_WANT").ok().map_or(u64::MAX, |w| {
                w.split(',').filter_map(derived::index_of).fold(0, |m, i| m | (1 << i))
            });
            let n = derived::RELATIONS.len();
            if self.mode == "derived" {
                let mut sink = derived::Count { counts: vec![0; n], touch: 0 };
                for i in 0..n {
                    if wanted & (1 << i) != 0 {
                        let t = Clock::now();
                        derived::produce(&ctx, 1 << i, &mut sink, &mut self.scratch);
                        totals.dns[i].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
                    }
                }
                for (i, c) in sink.counts.iter().enumerate() {
                    totals.drows[i].fetch_add(*c, Ordering::Relaxed);
                }
                totals.touch.fetch_add(sink.touch, Ordering::Relaxed);
            } else {
                let mut sink = derived::Dump { table: &table, file: index, lines: vec![Vec::new(); n], counts: vec![0; n] };
                derived::produce(&ctx, wanted, &mut sink, &mut self.scratch);
                for (i, c) in sink.counts.iter().enumerate() {
                    totals.drows[i].fetch_add(*c, Ordering::Relaxed);
                }
                for (i, v) in sink.lines.into_iter().enumerate() {
                    out.extend(v.into_iter().map(|l| format!("{i}\t{l}")));
                }
            }
            return;
        }
        if matches!(self.mode.as_str(), "lex" | "model" | "policy" | "rows" | "rows-lex") || self.mode.starts_with("dump:") {
            let incomplete = !parsed.errors().is_empty();
            let t = Clock::now();
            self.model.build_lexical(&table, incomplete);
            totals.ns[2].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
            let t = Clock::now();
            self.model.resolve_loads(&table);
            totals.ns[16].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
            if self.mode == "lex" {
                return;
            }
            let flow = self.mode != "rows-lex";
            if flow {
                let t = Clock::now();
                self.model.build_flow(&table);
                totals.ns[3].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
                totals.bindings.fetch_add(self.model.events.len() as u64, Ordering::Relaxed);
                if self.mode == "model" {
                    return;
                }
                let t = Clock::now();
                self.model.build_resolution(&table);
                totals.ns[4].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
                if self.mode == "policy" {
                    return;
                }
            }
            let wanted: u16 = std::env::var("SEMA_WANT").ok().and_then(|w| w.parse().ok()).unwrap_or(if flow { 0x7ff } else { 0x3f });
            let mut dump: Option<Box<[Vec<String>; 11]>> = self.mode.starts_with("dump:").then(|| Box::new(Default::default()));
            let mut o = rows::Out { lines: dump.as_deref_mut(), counts: [0; 11], path: path.to_owned() };
            for i in 0..11 {
                if wanted & (1 << i) != 0 {
                    let t = Clock::now();
                    rows::produce(&mut self.model, &table, index, 1 << i, &mut o, &mut self.scratch);
                    totals.ns[5 + i].fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
                }
            }
            for (i, n) in o.counts.iter().enumerate() {
                totals.rows[i].fetch_add(*n, Ordering::Relaxed);
            }
            if let Some(d) = dump {
                for (i, v) in d.into_iter().enumerate() {
                    out.extend(v.into_iter().map(|l| format!("{i}\t{l}")));
                }
            }
            return;
        }
        if self.mode == "sema" {
            self.sema.build(&table, parsed.syntax(), source);
            totals.scopes.fetch_add(self.sema.scopes.len() as u64, Ordering::Relaxed);
            totals.bindings.fetch_add(self.sema.bindings.len() as u64, Ordering::Relaxed);
            totals.resolved.fetch_add(self.sema.resolved_count as u64, Ordering::Relaxed);
            return;
        }
        if let Some(state) = &mut self.rules {
            state.file(path, source, &parsed, &table, &self.sema, totals, out);
        }
    }

    fn finish(&mut self, totals: &Totals) {
        if let Some(state) = &mut self.rules {
            state.finish(totals);
        }
    }
}
