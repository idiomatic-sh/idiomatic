//! Engine bake-off prototype (bead ryo-i8o1, throwaway): reference points for the Rust provider's parser.
//! `walk <parse|table> <file list> <root>`: four threads; parse with `ra_ap_syntax`; `table` adds one pre-order walk of the
//! lossless tree (nodes and tokens) writing a flat node table (kind, parent, subtree end, span) and a kinds-present bitset.

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

use ra_ap_syntax::{AstNode as _, Edition, NodeOrToken, SourceFile, WalkEvent};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args[1].clone();
    let files: Vec<String> = std::fs::read_to_string(&args[2]).expect("list").lines().map(str::to_owned).collect();
    let root = std::path::PathBuf::from(&args[3]);
    let cursor = AtomicUsize::new(0);
    let nodes = AtomicU64::new(0);
    let bytes_total = AtomicU64::new(0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                let (mut kind, mut parent, mut end, mut start, mut stop, mut stack) =
                    (Vec::<u16>::new(), Vec::<u32>::new(), Vec::<u32>::new(), Vec::<u32>::new(), Vec::<u32>::new(), Vec::<u32>::new());
                let mut present = [0u64; 8];
                loop {
                    let index = cursor.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(index) else { break };
                    let source = std::fs::read_to_string(root.join(path)).expect("read");
                    bytes_total.fetch_add(source.len() as u64, Ordering::Relaxed);
                    let parse = SourceFile::parse(&source, Edition::Edition2021);
                    let file = parse.tree();
                    if mode == "parse" {
                        nodes.fetch_add(u64::from(u32::from(file.syntax().text_range().len())), Ordering::Relaxed);
                        continue;
                    }
                    for v in [&mut parent, &mut end, &mut start, &mut stop, &mut stack] {
                        v.clear();
                    }
                    kind.clear();
                    present = [0; 8];
                    for event in file.syntax().preorder_with_tokens() {
                        match event {
                            WalkEvent::Enter(element) => {
                                let id = kind.len() as u32;
                                let k = match &element {
                                    NodeOrToken::Node(n) => n.kind(),
                                    NodeOrToken::Token(t) => t.kind(),
                                } as u16;
                                kind.push(k);
                                present[(k as usize >> 6) & 7] |= 1 << (k & 63);
                                parent.push(stack.last().copied().unwrap_or(u32::MAX));
                                end.push(0);
                                let range = element.text_range();
                                start.push(range.start().into());
                                stop.push(range.end().into());
                                stack.push(id);
                            }
                            WalkEvent::Leave(_) => {
                                if let Some(id) = stack.pop() {
                                    end[id as usize] = kind.len() as u32;
                                }
                            }
                        }
                    }
                    nodes.fetch_add(kind.len() as u64, Ordering::Relaxed);
                }
                let _ = present;
            });
        }
    });
    eprintln!(
        "{{\"mode\":\"{mode}\",\"files\":{},\"bytes\":{},\"nodes\":{}}}",
        files.len(),
        bytes_total.load(Ordering::Relaxed),
        nodes.load(Ordering::Relaxed)
    );
}
