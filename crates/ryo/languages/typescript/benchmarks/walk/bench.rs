//! Engine bake-off prototype (bead ryo-i8o1, throwaway): reference points for the TypeScript provider's parser.
//! `walk <parse|table|semantic> <file list> <root>`: four threads; parse with `oxc_parser`; `table` adds one pre-order walk writing
//! a flat node table (kind, parent, subtree end, span), per-kind counts and a kinds-present bitset; `semantic` adds the provider's
//! semantic library (`oxc_semantic`, scopes, symbols, references).

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

use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_span::GetSpan as _;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Default)]
struct Table {
    kind: Vec<u16>,
    parent: Vec<u32>,
    end: Vec<u32>,
    start: Vec<u32>,
    stop: Vec<u32>,
    stack: Vec<u32>,
    present: [u64; 4],
    counts: Vec<u32>,
}

impl<'a> Visit<'a> for Table {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let id = self.kind.len() as u32;
        let k = kind.ty() as u16;
        self.kind.push(k);
        self.present[(k as usize >> 6) & 3] |= 1 << (k & 63);
        self.parent.push(self.stack.last().copied().unwrap_or(u32::MAX));
        self.end.push(0);
        let span = kind.span();
        self.start.push(span.start);
        self.stop.push(span.end);
        self.stack.push(id);
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        if let Some(id) = self.stack.pop() {
            self.end[id as usize] = self.kind.len() as u32;
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args[1].clone();
    let files: Vec<String> = std::fs::read_to_string(&args[2]).expect("list").lines().map(str::to_owned).collect();
    let root = std::path::PathBuf::from(&args[3]);
    let cursor = AtomicUsize::new(0);
    let nodes = AtomicU64::new(0);
    let symbols = AtomicU64::new(0);
    let bytes_total = AtomicU64::new(0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                let mut table = Table::default();
                loop {
                    let index = cursor.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(index) else { break };
                    let full = root.join(path);
                    let source = std::fs::read_to_string(&full).expect("read");
                    bytes_total.fetch_add(source.len() as u64, Ordering::Relaxed);
                    let allocator = oxc_allocator::Allocator::default();
                    let source_type = oxc_span::SourceType::from_path(&full).unwrap_or_default();
                    let parsed = oxc_parser::Parser::new(&allocator, &source, source_type).parse();
                    if mode == "parse" {
                        nodes.fetch_add(parsed.program.body.len() as u64, Ordering::Relaxed);
                        continue;
                    }
                    table.kind.clear();
                    table.parent.clear();
                    table.end.clear();
                    table.start.clear();
                    table.stop.clear();
                    table.stack.clear();
                    table.present = [0; 4];
                    table.visit_program(&parsed.program);
                    table.counts.clear();
                    table.counts.resize(1024, 0);
                    for &k in &table.kind {
                        table.counts[k as usize] += 1;
                    }
                    nodes.fetch_add(table.kind.len() as u64, Ordering::Relaxed);
                    if mode == "table" {
                        continue;
                    }
                    let semantic = oxc_semantic::SemanticBuilder::new().build(&parsed.program);
                    symbols.fetch_add(semantic.semantic.scoping().symbols_len() as u64, Ordering::Relaxed);
                }
            });
        }
    });
    eprintln!(
        "{{\"mode\":\"{mode}\",\"files\":{},\"bytes\":{},\"nodes\":{},\"symbols\":{}}}",
        files.len(),
        bytes_total.load(Ordering::Relaxed),
        nodes.load(Ordering::Relaxed),
        symbols.load(Ordering::Relaxed)
    );
}
