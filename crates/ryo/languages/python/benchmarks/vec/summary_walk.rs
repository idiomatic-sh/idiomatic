//! Feed each accepted file's summary decisions directly to the executor while
//! the provider still lends its parse, table and on-demand semantic state.
use super::{
    Columns, Skip, column_pair, fields_present, finisher::Finisher, funnel::Words,
    pair_summary::Program, source_batch::SourceBatch, span_bytes, thread_cpu,
};
use ryo::languages::{self, FileWords, KindBits, NodeTable, node_table};
use ryo_python::fixtures::columns::ColumnWalk;
use std::fmt::Write;

pub(super) fn run(
    list: &[String],
    program: &Program,
    skip: &mut Skip,
    words: &Words,
    word_kind: u16,
    fields: usize,
    fwords: &mut FileWords,
    first: &mut ColumnWalk,
    second: &mut ColumnWalk,
    mut fin: Option<&mut Finisher<'_>>,
    root: Option<&str>,
    seed_out: Option<&str>,
) {
    let root = root.map(|s| format!("{}/", s.trim_end_matches('/')));
    let emit_rules: Vec<Vec<usize>> = fin
        .as_ref()
        .map(|f| {
            f.emit_names()
                .map(|(_, name)| {
                    program
                        .roots
                        .iter()
                        .enumerate()
                        .filter(|(_, (r, _))| r == name)
                        .map(|(i, _)| i)
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    if std::env::var_os("VEC_NATIVE_SPANS").is_some() {
        first.use_native_spans();
        second.use_native_spans();
    }
    if std::env::var_os("VEC_LAZY_CHILDREN").is_some() {
        first.defer_child_ids();
        second.defer_child_ids();
    }
    let (mut first_children, mut second_children) = (0usize, 0usize);
    let prep = thread_cpu();
    let keys = program
        .facts
        .iter()
        .map(|f| {
            ryo_python::fixtures::walking::declared_keys(f.relation)
                .iter()
                .find(|(_, i, kind)| f.key == Some(*i) && *kind == languages::CellKind::Node)
                .map(|(column, _, _)| *column)
        })
        .collect();
    let order = (std::env::var_os("VEC_REFERENCE_CHECKS").is_none())
        .then(|| super::pair_order::Order::new(program, keys));
    println!(
        "summary evaluation preparation {:.6} s",
        thread_cpu() - prep
    );
    let diagnostic = std::env::var_os("VEC_SUMMARY_METRICS").is_some();
    let mut metrics = rustc_hash::FxHashMap::<
        languages::Relation,
        (usize, usize, usize, usize, usize, usize, f64),
    >::default();
    let (mut bucket_requests, mut distinct_buckets) = (0u64, 0usize);
    let (mut probes, mut node_tests) = (0u64, 0u64);
    let mut input = SourceBatch::default();
    let mut table = NodeTable::default();
    let mut next_table = NodeTable::default();
    let mut alive = [
        vec![false; program.roots.len()],
        vec![false; program.roots.len()],
    ];
    let mut present = vec![0u64; words.names.len().div_ceil(64)];
    let mut kinds = Vec::new();
    let mut fields_kinds = 0.;
    let mut cols = Columns::default();
    let mut dump = String::new();
    let replay = fin.is_some() && std::env::var_os("VEC_REPLAY_ASKED").is_some();
    let mut queued = Vec::new();
    let (mut elements, mut steps, mut candidates, mut survivors) = (0u64, 0, 0u64, 0u64);
    let (mut read, mut contents, mut skip_cpu, mut checks, mut visit, mut finish) =
        (0., 0., 0., 0., 0., 0.);
    let fin0 = fin.as_ref().map_or(0., |f| f.time);
    println!("start-up to first file {:.4} s", thread_cpu());
    let start = thread_cpu();
    let mut i = 0;
    while i < list.len() {
        let begin = thread_cpu();
        let stop = input.load(list, i).expect("read sources");
        read += thread_cpu() - begin;
        while i < stop {
            let next = (i + 1 < stop).then(|| (list[i + 1].as_str(), input.get(i + 1)));
            column_pair::walked_pair(
                first,
                &mut table,
                second,
                &mut next_table,
                (&list[i], input.get(i)),
                next,
                &mut alive,
                &mut |slot, file, alive| {
                    let begin = thread_cpu();
                    let id = i + slot;
                    let bytes = input.get(id);
                    let t = file.table;
                    elements += node_table::element_count(t).0 as u64;
                    fields_present(t, &mut cols, fields);
                    kinds.clear();
                    kinds.extend(node_table::kinds_present(t).0.iter().map(|w| w.0));
                    fields_kinds += thread_cpu() - begin;
                    present.fill(0);
                    for p in node_table::ids_of_kind(t, KindBits(word_kind)) {
                        if let Some(&w) = words.ids.get(&bytes[span_bytes(t, p.0 as usize)]) {
                            present[usize::from(w) / 64] |= 1 << (w % 64);
                        }
                    }
                    let c = thread_cpu();
                    contents += c - begin;
                    skip.file(&kinds, &cols.field_present);
                    for (r, a) in alive.iter_mut().enumerate() {
                        *a = skip.alive(r) && skip.words_alive(r, &present);
                    }
                    candidates += alive.iter().filter(|a| **a).count() as u64;
                    let b = thread_cpu();
                    skip_cpu += b - c;
                    let path = root
                        .as_ref()
                        .map(|r| list[id].strip_prefix(r).unwrap_or(&list[id]).as_bytes());
                    let work = program.ordered_file(
                        order.as_ref(),
                        t,
                        bytes,
                        path,
                        fwords,
                        |rel, request, w, out| {
                            use super::pair_checks::Request;
                            let now = diagnostic.then(thread_cpu);
                            let mut row_count = 0;
                            let mode = match request {
                                Request::Rows(languages::Wanted::Whole) => 0,
                                Request::Rows(_) => 1,
                                _ => 2,
                            };
                            let result = match request {
                                Request::Rows(wanted) => {
                                    let result = file.fill(rel, wanted, w, out);
                                    row_count = out.0.first().map_or(0, |c| c.0.len());
                                    Some(matches!(result, languages::FactAnswer::Answered(_)))
                                }
                                Request::Has(at) => file.projected_key_exists(rel, at),
                                Request::Keys(bits) => file.projected_keys(rel).map(|keys| {
                                    for p in keys {
                                        let word = (p.0 / 64) as usize;
                                        if bits.len() <= word {
                                            bits.resize(word + 1, 0);
                                        }
                                        bits[word] |= 1 << (p.0 % 64);
                                        row_count += 1;
                                    }
                                    true
                                }),
                            };
                            if let Some(now) = now
                                && result.is_some()
                            {
                                let m = metrics.entry(rel).or_default();
                                if m.0 != id + 1 {
                                    m.0 = id + 1;
                                    m.1 += 1;
                                }
                                match mode {
                                    0 => m.2 += 1,
                                    1 => m.3 += 1,
                                    _ => m.4 += 1,
                                }
                                m.5 += row_count;
                                m.6 += thread_cpu() - now;
                            }
                            result
                        },
                        alive,
                    );
                    steps += work.steps;
                    bucket_requests += work.bucket_requests;
                    distinct_buckets += work.distinct_buckets;
                    probes += work.candidates;
                    node_tests += work.local_tests;
                    checks += thread_cpu() - b;
                    first_children += usize::from(node_table::child_ids_requested(t));
                    survivors += alive.iter().filter(|a| **a).count() as u64;
                    if seed_out.is_some() {
                        for (a, (rule, _)) in alive.iter().zip(&program.roots) {
                            if *a {
                                writeln!(dump, "{id}\t{rule}").expect("seed receipt");
                            }
                        }
                    }
                    visit += thread_cpu() - begin;
                },
                &mut |slot, file, alive| {
                    let begin = thread_cpu();
                    if let Some(f) = fin.as_deref_mut() {
                        let id = i + slot;
                        let wanted =
                            f.wanted(&list[id], &|e| emit_rules[e].iter().any(|&r| alive[r]));
                        if f.visits(&wanted) {
                            if replay {
                                queued.push((
                                    id,
                                    Box::<[u8]>::from(input.get(id)),
                                    wanted.into_boxed_slice(),
                                ));
                            } else {
                                f.column_file(id, &list[id], file, input.get(id), &wanted);
                            }
                        }
                    }
                    second_children += usize::from(node_table::child_ids_requested(file.table));
                    finish += thread_cpu() - begin;
                },
            );
            i += if next.is_some() { 2 } else { 1 };
        }
    }
    let elapsed = thread_cpu() - start;
    let second = fin.as_ref().map_or(0., |f| f.time) - fin0;
    if replay {
        let count = queued.len();
        let source_bytes: usize = queued.iter().map(|(_, src, _)| src.len()).sum();
        let emit_bytes: usize = queued
            .iter()
            .map(|(_, _, emits)| std::mem::size_of_val(emits.as_ref()))
            .sum();
        let f = fin.as_deref_mut().expect("replay executor");
        let begin = thread_cpu();
        let before = f.time;
        for (id, src, wanted) in queued {
            first
                .walked(&list[id], &src, &mut table, &mut |file| {
                    f.column_file(id, &list[id], file, &src, &wanted);
                })
                .expect("the first pass accepted this source");
        }
        let duration = thread_cpu() - begin;
        println!(
            "replay files {count} retained source {source_bytes} emit bytes {emit_bytes} second pass CPU {duration:.4} reparse and walk {:.4}",
            duration - (f.time - before)
        );
    }
    for (relation, (_, files, whole, keyed, projected, rows, cpu)) in metrics {
        println!(
            "summary fill {relation:?} files {files} whole {whole} keyed {keyed} projected {projected} rows {rows} CPU {cpu:.6}"
        );
    }
    if let Some(p) = seed_out {
        std::fs::write(p, dump).expect("write seeds");
    }
    if replay {
        println!("first pass: executor selection {second:.4} s");
    } else {
        println!("one walk: the second half inside the first half's visit {second:.4} s");
    }
    println!("thread CPU total {:.4} s", elapsed - second);
    println!(
        "phase read: {read:.4} s\nphase walk+table: {:.4} s\nphase contents: {contents:.4} s\nphase skip tests: {skip_cpu:.4} s\nphase seed handoff: {:.4} s",
        elapsed - visit - finish - read,
        visit - contents - skip_cpu - checks + finish - second
    );
    println!(
        "contents kinds and fields {fields_kinds:.4} s words {:.4} s",
        contents - fields_kinds
    );
    if diagnostic {
        println!(
            "bucket requests {bucket_requests} distinct non-coalesced buckets summed over files {distinct_buckets}"
        );
    }
    println!(
        "files with materialized child slices after prefix {first_children} after executor {second_children}"
    );
    println!("candidate nodes {probes} local tests {node_tests}");
    println!("pair summary checks {steps} CPU {checks:.4} s");
    println!("file seeds: {elements} elements");
    println!(
        "candidates {candidates} survivors {survivors} steps {steps} rule-files-with-survivors {survivors}"
    );
}
