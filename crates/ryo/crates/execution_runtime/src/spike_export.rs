//! Spike (ryo-fmx3, bead ryo-r6ux): the facts export of the engine bake-off. When `RYO_EXPORT` names a folder, every file's
//! rows of every relation the program reads are appended, as the engine's lookup lends them, to `<folder>/<relation>.tsv`
//! (a leading file ordinal, then the relation's columns: nodes and spans as the integers the engine holds, names as escaped
//! text), the file to `files.tsv`, and the plans to `plans-<n>.json`. Throwaway export code; no engine behaviour reads it.

use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::table_texts::service::{name_text, symbol_text};
use ryo_operators::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::service::{all, column_name, columns as catalog_columns, from_bits, is_key, kind, name};
use ryo_relations::domain::relations::types::ColumnKeyness;
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, FileId, Relation, Value};
use ryo_values::domain::files::types::SourceView;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

/// Each relation's open file, and the file list.
struct Sinks {
    relations: HashMap<Relation, std::io::BufWriter<std::fs::File>>,
    files: Option<std::io::BufWriter<std::fs::File>>,
}

fn sinks() -> &'static Mutex<Sinks> {
    static SINKS: OnceLock<Mutex<Sinks>> = OnceLock::new();
    SINKS.get_or_init(|| {
        Mutex::new(Sinks {
            relations: HashMap::new(),
            files: None,
        })
    })
}

/// Nanoseconds spent opening files' facts (`fact_lookup::opened`), and spent writing the export, over all workers.
pub(crate) static OPEN_NANOS: AtomicU64 = AtomicU64::new(0);
static WRITE_NANOS: AtomicU64 = AtomicU64::new(0);
static FILES: AtomicU64 = AtomicU64::new(0);
static PLANS: AtomicU64 = AtomicU64::new(0);

fn escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
}

fn cell(out: &mut String, kind: CellKind, bits: u64, texts: &Texts<'_>) {
    match from_bits(kind, Bits(bits)) {
        Value::Symbol(symbol) => match symbol_text(texts, symbol) {
            Some(text) => escaped(out, text.0),
            None => out.push_str("\\N"),
        },
        Value::Name(found) => match name_text(texts, found) {
            Some(text) => escaped(out, text.0),
            None => out.push_str("\\N"),
        },
        Value::Number(number) => {
            let _ignored = write!(out, "{}", number.0);
        }
        Value::Node(_) | Value::File(_) | Value::Span(_) | Value::Flag(_) => {
            let _ignored = write!(out, "{bits}");
        }
    }
}

/// Appends one file's rows of every relation in `relations` that `columns` lends.
pub(crate) fn file<'c>(
    file: FileId,
    path: &str,
    source: SourceView<'_>,
    symbols: &Symbols,
    relations: &[Relation],
    elements: &ryo_language::domain::elements::types::Elements,
    columns: &dyn Fn(Relation) -> Option<ColumnsView<'c>>,
) {
    let Some(dir) = ryo_operators::spike::export_dir() else {
        return;
    };
    let started = std::time::Instant::now();
    let texts = Texts {
        symbols,
        source,
        paths: &[],
        arenas: &[],
    };
    let mut chunks: Vec<(Relation, String)> = Vec::with_capacity(relations.len());
    for relation in relations {
        // The tree relations the engine reads in place from the file's elements, never from the facts: their rows as the
        // engine's tree scan makes them, every element (every one but the root for the edges).
        if let Some(form) = ryo_operators::domain::tree_cells::service::native_form(*relation) {
            let nodes: Vec<ryo_relations::domain::relations::types::NodeId> =
                ryo_language::domain::elements::service::positions(elements)
                    .map(|at| ryo_relations::domain::relations::service::node_id(file, at))
                    .collect();
            let table = ryo_operators::domain::tree_cells::service::tree_table(
                form,
                ryo_relations::domain::relations::service::kinds(*relation),
                elements,
                ryo_operators::domain::tree_cells::service::with_rows(form, &nodes),
            );
            let width = table.kinds.len().max(1);
            let mut out = String::with_capacity(table.cells.len() * 12);
            for row in table.cells.chunks(width) {
                let _ignored = write!(out, "{}", file.0);
                for (kind, value) in table.kinds.iter().zip(row) {
                    out.push('\t');
                    let bits = ryo_relations::domain::relations::service::to_bits(*value).0;
                    cell(&mut out, *kind, bits, &texts);
                }
                out.push('\n');
            }
            chunks.push((*relation, out));
            continue;
        }
        let Some(view) = columns(*relation) else {
            continue;
        };
        let rows = view.columns.first().map_or(0, |column| column.0.len());
        let mut out = String::with_capacity(rows * 24);
        for row in 0..rows {
            let _ignored = write!(out, "{}", file.0);
            for (kind, column) in view.kinds.iter().zip(view.columns) {
                out.push('\t');
                cell(&mut out, *kind, column.0.get(row).copied().unwrap_or(0), &texts);
            }
            out.push('\n');
        }
        chunks.push((*relation, out));
    }
    let mut sinks = sinks().lock().unwrap_or_else(PoisonError::into_inner);
    for (relation, chunk) in chunks {
        let sink = sinks.relations.entry(relation).or_insert_with(|| {
            let target = dir.join(format!("{}.tsv", name(relation).0));
            std::io::BufWriter::with_capacity(
                1 << 20,
                std::fs::File::create(target).unwrap_or_else(|_| std::process::abort()),
            )
        });
        drop(sink.write_all(chunk.as_bytes()));
    }
    let files = sinks.files.get_or_insert_with(|| {
        std::io::BufWriter::new(
            std::fs::File::create(dir.join("files.tsv")).unwrap_or_else(|_| std::process::abort()),
        )
    });
    let mut line = String::new();
    escaped(&mut line, path);
    drop(writeln!(files, "{}\t{line}\t{}", file.0, source.0.len()));
    drop(sinks);
    FILES.fetch_add(1, Ordering::Relaxed);
    WRITE_NANOS.fetch_add(
        u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

/// Writes one program's plans as `plans-<n>.json`.
pub(crate) fn plans(text: &str) {
    let Some(dir) = ryo_operators::spike::export_dir() else {
        return;
    };
    let at = PLANS.fetch_add(1, Ordering::Relaxed);
    drop(std::fs::write(dir.join(format!("plans-{at}.json")), text));
}

/// Flushes every relation's file and writes `timings.json`.
pub(crate) fn flush() {
    let Some(dir) = ryo_operators::spike::export_dir() else {
        return;
    };
    let mut sinks = sinks().lock().unwrap_or_else(PoisonError::into_inner);
    for sink in sinks.relations.values_mut() {
        drop(sink.flush());
    }
    if let Some(files) = sinks.files.as_mut() {
        drop(files.flush());
    }
    // The catalog: every relation's columns in declared order, their kinds and their declared keyness.
    let mut catalog = String::from("{");
    for (at, relation) in all().iter().enumerate() {
        if at > 0 {
            catalog.push_str(",\n");
        }
        let _ignored = write!(catalog, "\"{}\":[", name(*relation).0);
        for (place, column) in catalog_columns(*relation).into_iter().enumerate() {
            if place > 0 {
                catalog.push(',');
            }
            let _ignored = write!(
                catalog,
                "{{\"name\":\"{}\",\"kind\":\"{}\",\"key\":{}}}",
                column_name(column).0,
                ryo_execution_domain::spike_export::kind(kind(column)),
                is_key(column) == ColumnKeyness::Key
            );
        }
        catalog.push(']');
    }
    catalog.push_str("}\n");
    drop(std::fs::write(dir.join("catalog.json"), catalog));
    drop(std::fs::write(
        dir.join("timings.json"),
        format!(
            "{{\"files\":{},\"open_facts_s\":{:.3},\"export_write_s\":{:.3}}}\n",
            FILES.load(Ordering::Relaxed),
            f64::from(u32::try_from(OPEN_NANOS.load(Ordering::Relaxed) / 1_000_000).unwrap_or(u32::MAX)) / 1000.0,
            f64::from(u32::try_from(WRITE_NANOS.load(Ordering::Relaxed) / 1_000_000).unwrap_or(u32::MAX)) / 1000.0,
        ),
    ));
}
