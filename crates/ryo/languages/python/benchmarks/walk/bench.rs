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
        "{{\"mode\":\"{mode}\",\"files\":{},\"bytes\":{},\"nodes\":{},\"scopes\":{},\"bindings\":{},\"resolved\":{},\"kinds_present\":{},\"extra\":{:?},\"findings\":{}}}",
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
    rules: Option<rules::State<'p>>,
    provider: Option<ProviderState>,
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
            rules: program.map(rules::State::new),
            provider,
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
        let parsed = ruff_python_parser::parse_unchecked(source, options());
        if self.mode == "parse" {
            totals.nodes.fetch_add(parsed.tokens().len() as u64, Ordering::Relaxed);
            return;
        }
        let mut table = table::Table::default();
        table.build(parsed.syntax());
        totals.nodes.fetch_add(table.len() as u64, Ordering::Relaxed);
        totals.kinds_present.fetch_add(u64::from(table.present.count_ones()), Ordering::Relaxed);
        if self.mode == "table" {
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
