//! Feed each accepted file's summary decisions directly to the executor while
//! the provider still lends its parse, table and on-demand semantic state.
use super::{
    Columns, Skip, fields_present, visit::Finisher, funnel::Words, pair_summary::Program,
    source_batch::SourceBatch, thread_cpu,
};
use ryo::languages::{self, FileWords, NodeTable, node_table};
use ryo_python::fixtures::columns::ColumnWalk;
use std::fmt::Write;

pub(super) fn run(
    list: &[String],
    program: &Program,
    skip: &mut Skip,
    words: &Words,
    fields: usize,
    fwords: &mut FileWords,
    first: &mut ColumnWalk,
    second: &mut ColumnWalk,
    mut fin: Option<&mut Finisher<'_>>,
    root: Option<&str>,
    seed_out: Option<&str>,
    exact_out: Option<&str>,
    order: &super::summary_schedule::Schedule,
    scratch: &mut super::pair_checks::Scratch,
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
    for walk in [&mut *first, &mut *second] {
        walk.defer_lexical();
        walk.use_native_navigation();
    }

    #[cfg(feature = "summary-diagnostics")]
    struct ProviderClock<'a>(f64, &'a mut f64);
    #[cfg(feature = "summary-diagnostics")]
    impl Drop for ProviderClock<'_> {
        fn drop(&mut self) {
            *self.1 += thread_cpu() - self.0;
        }
    }
    #[cfg(feature = "summary-diagnostics")]
    let mut provider_cpu = 0.0;
    #[cfg(feature = "summary-diagnostics")]
    let mut metrics =
        rustc_hash::FxHashMap::<String, (usize, usize, usize, usize, usize, f64)>::default();
    let certify = fin.as_ref().is_none_or(|f| f.can_finish_exact());
    let mut exact_counts = vec![(0u64, 0u64, 0u64); program.roots.len()];
    let mut exact_log =
        exact_out.map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("exact pairs")));
    let mut input = SourceBatch::default();
    let mut table = NodeTable::default();
    let mut other_table = NodeTable::default();
    let mut alive = std::array::from_fn(|_| (vec![false; program.roots.len()], Vec::new(), std::collections::HashMap::new()));
    let mut present = vec![0u64; words.names.len().div_ceil(64)];
    let mut kinds = Vec::new();
    let mut fields_kinds = 0.;
    let mut cols = Columns::default();
    let mut wanted = Vec::new();
    let mut dump = String::new();
    let (mut elements, mut steps, mut candidates, mut survivors) = (0u64, 0, 0u64, 0u64);
    let (mut read, mut contents, mut skip_cpu, mut checks, mut visit, mut finish) =
        (0., 0., 0., 0., 0., 0.);
    let fin0 = fin.as_ref().map_or(0., |f| f.time);
    let executor = std::cell::RefCell::new(fin);
    println!("start-up to first file {:.4} s", thread_cpu());
    let start = thread_cpu();
    let mut i = 0;
    while i < list.len() {
        let begin = thread_cpu();
        let stop = input.load(list, i).expect("read sources");
        read += thread_cpu() - begin;
        while i < stop {
            let count = (stop - i).min(2);
            super::column_pair::walked_pair(
                first,
                &mut table,
                second,
                &mut other_table,
                (&list[i], input.get(i)),
                (count == 2).then(|| (list[i + 1].as_str(), input.get(i + 1))),
                &mut alive,
                &mut |slot, file, (alive, exact, kept)| {
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
                    order
                        .literals
                        .names(t, bytes, &mut present, &mut scratch.literals);
                    let c = thread_cpu();
                    contents += c - begin;
                    skip.file(&kinds, &cols.field_present);
                    for (r, a) in alive.iter_mut().enumerate() {
                        *a = skip.alive(r) && skip.words_alive(r, &present);
                    }
                    candidates += alive.iter().filter(|a| **a).count() as u64;
                    let b = thread_cpu();
                    skip_cpu += b - c;
                    let mut fin = executor.borrow_mut();
                    let run_path = fin.as_deref().map(|f| f.run.paths[id].clone());
                    let path = run_path.as_deref().map(str::as_bytes).or_else(|| {
                        root.as_ref().map(|r| list[id].strip_prefix(r).unwrap_or(&list[id]).as_bytes())
                    });
                    exact.clear();
                    kept.clear();
                    steps += program.eval(
                        scratch,
                        order,
                        t,
                        bytes,
                        path,
                        None,
                        fwords,
                        |source, query, words, out| {
                            #[cfg(feature = "summary-diagnostics")]
                            let _provider_clock = ProviderClock(thread_cpu(), &mut provider_cpu);
                            use super::{pair_checks::Query, pair_summary::Source};
                            if matches!(source, Source::Catalog(languages::Relation::ModuleIdentity)) {
                                return match query {
                                    Query::Rows => Some(fin.as_deref_mut().is_some_and(|f| f.identity(id, words, out))),
                                    _ => None,
                                };
                            }
                            #[cfg(feature = "summary-diagnostics")]
                            let now = thread_cpu();
                            #[cfg(feature = "summary-diagnostics")]
                            let mut rows = 0;
                            #[cfg(feature = "summary-diagnostics")]
                            let whole = matches!(query, Query::Rows);
                            let result = match query {
                                Query::Rows => {
                                    let host = match source {
                                        Source::Catalog(r) => super::host_columns::fill_path(
                                            r,
                                            std::path::Path::new(&list[id]),
                                            id,
                                            out,
                                        ),
                                        _ => None,
                                    };
                                    let answered = host.unwrap_or_else(|| match source {
                                        Source::Walk(name) if super::pair_provider::visit_table(name) => fin
                                            .as_deref_mut()
                                            .is_some_and(|f| f.walk_table(id, file, bytes, name, words, out, kept)),
                                        _ => super::pair_provider::read(file, id, source, words, out),
                                    });
                                    #[cfg(feature = "summary-diagnostics")]
                                    {
                                        rows = out.0.first().map_or(0, |c| c.0.len());
                                    }
                                    Some(answered)
                                }
                                Query::Exists => {
                                    if let Source::Catalog(r) = source {
                                        file.summary_presence(r)
                                    } else {
                                        None
                                    }
                                }
                                Query::Keys(bits) => {
                                    if let Source::Catalog(r) = source {
                                        file.projected_keys(r).map(|keys| {
                                            for k in keys {
                                                let w = (k.0 / 64) as usize;
                                                if bits.len() <= w {
                                                    bits.resize(w + 1, 0);
                                                }
                                                bits[w] |= 1 << (k.0 % 64);
                                                #[cfg(feature = "summary-diagnostics")]
                                                {
                                                    rows += 1;
                                                }
                                            }
                                            true
                                        })
                                    } else {
                                        None
                                    }
                                }
                            };
                            #[cfg(feature = "summary-diagnostics")]
                            if result.is_some() {
                                let elapsed = thread_cpu() - now;
                                let name = match source {
                                    Source::Catalog(r) => format!("{r:?}"),
                                    Source::Walk(n) => n.to_owned(),
                                };
                                let m = metrics.entry(name).or_default();
                                if m.0 != id + 1 {
                                    m.0 = id + 1;
                                    m.1 += 1;
                                }
                                if whole {
                                    m.2 += 1
                                } else {
                                    m.3 += 1
                                }
                                m.4 += rows;
                                m.5 += elapsed;
                            }
                            result
                        },
                        alive,
                        certify.then_some(exact),
                    );
                    checks += thread_cpu() - b;
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
                &mut |slot, file, (alive, exact, kept)| {
                    let begin = thread_cpu();
                    if let Some(f) = executor.borrow_mut().as_deref_mut() {
                        let id = i + slot;
                        f.wanted_into(|e| emit_rules[e].iter().any(|&r| alive[r]), &mut wanted);
                        if !exact.is_empty() {
                            wanted.retain(|&e| {
                                let [ri] = emit_rules[e].as_slice() else {
                                    return true;
                                };
                                let Ok(at) = exact.binary_search_by_key(ri, |(r, _)| *r) else {
                                    return true;
                                };
                                if let Some(n) = f.emit_exact(id, file.table, e, &exact[at].1) {
                                    if let Some(log) = &mut exact_log {
                                        use std::io::Write;
                                        writeln!(
                                            log,
                                            "{}\t{}\t{n}",
                                            program.roots[*ri].0, f.run.paths[id]
                                        )
                                        .expect("exact pair");
                                    }
                                    exact_counts[*ri].0 += 1;
                                    exact_counts[*ri].1 += n;
                                    exact_counts[*ri].2 += u64::from(n > 0);
                                }
                                false
                            });
                        }
                        if f.visits(&wanted) {
                            f.column_file(id, file, input.get(id), &wanted, std::mem::take(kept));
                        }
                    }
                    finish += thread_cpu() - begin;
                },
            );
            i += count;
        }
    }
    #[cfg(feature = "summary-diagnostics")]
    println!(
        "domain bitset words {} peak {}",
        scratch.words, scratch.peak
    );
    for (ri, _) in &program.exact {
        let (pairs, rows, nonempty) = exact_counts[*ri];
        println!(
            "EXACT\t{}\t{pairs}\t{rows}\t{nonempty}",
            program.roots[*ri].0
        );
    }
    #[cfg(feature = "summary-diagnostics")]
    println!("SUMMARY_PROVIDER {provider_cpu:.9}");
    let elapsed = thread_cpu() - start;
    let mut fin = executor.into_inner();
    let second = fin.as_ref().map_or(0., |f| f.time) - fin0;
    #[cfg(feature = "summary-diagnostics")]
    for (relation, (_, files, whole, projected, rows, cpu)) in metrics {
        println!(
            "summary fill {relation} files {files} whole {whole} keyed 0 projected {projected} rows {rows} CPU {cpu:.6}"
        );
    }
    if let Some(path) = seed_out {
        std::fs::write(path, dump).expect("write file seeds");
    }
    println!("one walk: the second half inside the first half's visit {second:.4} s");
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
    println!("pair summary checks {steps} CPU {checks:.4} s");
    println!("file seeds: {elements} elements");
    println!(
        "candidates {candidates} survivors {survivors} steps {steps} rule-files-with-survivors {survivors}"
    );
}
