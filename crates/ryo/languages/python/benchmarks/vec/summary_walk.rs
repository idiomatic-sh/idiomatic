//! Feed each accepted file's summary decisions directly to the executor while
//! the provider still lends its parse, table and on-demand semantic state.
use super::{
    Columns, Skip, fields_present, finisher::Finisher, funnel::Words, pair_summary::Program,
    source_batch::SourceBatch, span_bytes, thread_cpu,
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
    mut make_walk: impl FnMut() -> ColumnWalk,
    mut fin: Option<&mut Finisher<'_>>,
    root: Option<&str>,
    seed_out: Option<&str>,
    exact_out: Option<&str>,
    codes: Option<Vec<Option<usize>>>,
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
    let width = match std::env::var("VEC_LOAN_BATCH").as_deref() {
        Err(_) | Ok("2") => 2,
        Ok("1") => 1,
        Ok("4") => 4,
        Ok("8") => 8,
        _ => panic!("expected 1, 2, 4, or 8 parse loans"),
    };
    let mut extra: Vec<_> = (2..width).map(|_| make_walk()).collect();
    let mut walks = vec![first, second];
    walks.extend(extra.iter_mut());
    let verify = std::env::var_os("VEC_VERIFY_CHILDREN").is_some();
    for walk in &mut walks {
        walk.defer_lexical();
        if std::env::var_os("VEC_NATIVE_SPANS").is_some() {
            walk.use_native_spans();
        }
        if std::env::var_os("VEC_LAZY_CHILDREN").is_some() {
            walk.defer_child_ids();
        }
        if verify || std::env::var_os("VEC_TREE_RANGES").is_some() {
            walk.use_native_navigation();
        }
    }
    let (mut first_children, mut second_children) = (0usize, 0usize);
    let prep = thread_cpu();
    let mut order = super::summary_schedule::Schedule::new(program);
    order.symbols = codes;
    order.literals = std::env::var_os("VEC_LITERAL_BUCKETS")
        .is_some()
        .then(|| super::summary_literals::Layout::new(program, words, word_kind));
    order.keys = program
        .facts
        .iter()
        .map(|s| match s.source {
            super::pair_summary::Source::Catalog(r) => {
                ryo_python::fixtures::walking::declared_keys(r)
                    .into_iter()
                    .find(|(_, i, k)| Some(*i) == s.key && *k == languages::CellKind::Node)
                    .map(|v| v.0)
            }
            _ => None,
        })
        .collect();
    println!(
        "summary evaluation preparation {:.6} s",
        thread_cpu() - prep
    );
    let wide_keys = std::env::var_os("VEC_NATIVE_KEYS").is_some();
    let diagnostic = std::env::var_os("VEC_SUMMARY_METRICS").is_some();
    let mut scratch = super::pair_checks::Scratch::default();
    if std::env::var_os("VEC_MASK_CENSUS").is_some() {
        scratch.measure_masks();
    }
    let mut metrics =
        rustc_hash::FxHashMap::<String, (usize, usize, usize, usize, usize, f64, usize)>::default();
    let certify = fin.as_ref().is_none_or(|f| f.can_finish_exact());
    let mut exact_counts = vec![(0u64, 0u64, 0u64); program.roots.len()];
    let mut exact_log =
        exact_out.map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("exact pairs")));
    let mut input = SourceBatch::default();
    let mut tables: Vec<_> = (0..width).map(|_| NodeTable::default()).collect();
    let mut alive = vec![(vec![false; program.roots.len()], Vec::new()); width];
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
            let count = (stop - i).min(width);
            super::column_batch::walked(
                &mut walks[..count],
                &mut tables[..count],
                &mut alive[..count],
                &list[i..i + count],
                &input,
                i,
                &mut |slot, file, (alive, exact)| {
                    let begin = thread_cpu();
                    let id = i + slot;
                    let bytes = input.get(id);
                    let t = file.table;
                    if verify {
                        for id in 0..node_table::element_count(t).0 as u32 {
                            let p = languages::Position(id);
                            assert!(
                                node_table::direct_children(t, p)
                                    .unwrap()
                                    .eq(node_table::children_of(t, p).iter().copied()),
                                "children at {id} in {}",
                                list[i + slot]
                            );
                        }
                    }
                    elements += node_table::element_count(t).0 as u64;
                    fields_present(t, &mut cols, fields);
                    kinds.clear();
                    kinds.extend(node_table::kinds_present(t).0.iter().map(|w| w.0));
                    fields_kinds += thread_cpu() - begin;
                    present.fill(0);
                    if let Some(layout) = &order.literals {
                        layout.names(t, bytes, &mut present, &mut scratch.literals);
                    } else {
                        for p in node_table::ids_of_kind(t, KindBits(word_kind)) {
                            if let Some(&w) = words.ids.get(&bytes[span_bytes(t, p.0 as usize)]) {
                                present[usize::from(w) / 64] |= 1 << (w % 64);
                            }
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
                    exact.clear();
                    steps += program.eval(
                        &mut scratch,
                        Some(&order),
                        t,
                        bytes,
                        path,
                        fwords,
                        |source, query, words, out| {
                            use super::{pair_checks::Query, pair_summary::Source};
                            let now = diagnostic.then(thread_cpu);
                            let mut rows = 0;
                            let whole = matches!(query, Query::Rows);
                            let keyed = matches!(query, Query::Node(..) | Query::HasAt(..));
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
                                    let answered = host.unwrap_or_else(|| {
                                        super::pair_provider::read(file, id, source, words, out)
                                    });
                                    rows = out.0.first().map_or(0, |c| c.0.len());
                                    Some(answered)
                                }
                                Query::HasAt(at) => {
                                    if let Source::Catalog(r) = source {
                                        file.projected_key_exists(r, Some(at))
                                    } else {
                                        None
                                    }
                                }
                                Query::Node(key, at) => {
                                    if let Source::Catalog(r) = source {
                                        if file.can_select_summary_nodes(r, wide_keys) {
                                            let yes = file.fill(
                                                r,
                                                languages::Wanted::Key(
                                                    key,
                                                    languages::CellWord(u64::from(at.0)),
                                                ),
                                                words,
                                                out,
                                            );
                                            rows = out.0.first().map_or(0, |c| c.0.len());
                                            Some(matches!(yes, languages::FactAnswer::Answered(_)))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                }
                                Query::Exists => {
                                    if let Source::Catalog(r) = source {
                                        file.summary_presence(r, None)
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
                                                rows += 1;
                                            }
                                            true
                                        })
                                    } else {
                                        None
                                    }
                                }
                            };
                            if let Some(begin) = now
                                && result.is_some()
                            {
                                let elapsed = thread_cpu() - begin;
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
                                } else if keyed {
                                    m.6 += 1;
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
                &mut |slot, file, (alive, exact)| {
                    let begin = thread_cpu();
                    if let Some(f) = fin.as_deref_mut() {
                        let id = i + slot;
                        let mut wanted =
                            f.wanted(&list[id], &|e| emit_rules[e].iter().any(|&r| alive[r]));
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
            i += count;
        }
    }
    println!(
        "domain bitset words {} peak {}",
        scratch.words, scratch.peak
    );
    scratch.report_masks();
    for (ri, _) in &program.exact {
        let (pairs, rows, nonempty) = exact_counts[*ri];
        println!(
            "EXACT\t{}\t{pairs}\t{rows}\t{nonempty}",
            program.roots[*ri].0
        );
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
            walks[0]
                .walked(&list[id], &src, &mut tables[0], &mut |file| {
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
    for (relation, (_, files, whole, projected, rows, cpu, keyed)) in metrics {
        println!(
            "summary fill {relation} files {files} whole {whole} keyed {keyed} projected {projected} rows {rows} CPU {cpu:.6}"
        );
    }
    if let Some(path) = seed_out {
        std::fs::write(path, dump).expect("write file seeds");
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
    println!(
        "literal bucket visits names {} other {}",
        scratch.literals.name_visits, scratch.literals.other_visits
    );
    println!("distinct existential tests {}", scratch.witness_calls);
    println!(
        "node-class files {} masks {} rejections {}",
        scratch.classes.built, scratch.classes.tested, scratch.classes.ruled_out
    );
    println!(
        "candidate nodes {} local tests {}",
        scratch.visits, scratch.points
    );
    println!("pair summary checks {steps} CPU {checks:.4} s");
    println!("file seeds: {elements} elements");
    println!(
        "candidates {candidates} survivors {survivors} steps {steps} rule-files-with-survivors {survivors}"
    );
}
