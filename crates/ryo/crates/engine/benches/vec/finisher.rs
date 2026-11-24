//! File-local execution over the first walk's node table. One driver serves the run.

use super::{finish, flag, thread_cpu};
use std::collections::HashMap;
use ryo::languages::NodeTable;
use std::collections::HashSet;

/// The second half of a run: the plan's emits to run, the run's symbols, the executor, and what it found.
pub(super) struct Finisher<'p> {
    plan: &'p finish::Plan,
    /// The plan's emits the run asks for (`--rules`), by index into `plan.emits`.
    emits: Vec<usize>,
    check: bool,
    pub(super) query_rows: bool,
    dump: Option<String>,
    dump_chunks: Vec<(usize, usize, String)>,
    pub(super) run: finish::Run,
    exec: finish::Exec<'p>,
    findings: finish::json_output::Findings,
    failed: HashMap<String, (usize, String)>,
    /// redo-honest: per plan emit, whether its rule runs only on a file whose syntax the parser accepted (the request's
    /// `rule_applicability`, `--applicability F`: "accepted-syntax"); without the flag every rule runs on every file.
    accepted_only: Vec<bool>,
    /// redo-union pass 2 c16: a failed emit by its index (no name hashed an emit a file).
    failed_idx: Vec<bool>,
    rule_stats: Vec<[u64; 3]>,
    check_lines: Vec<(usize, String)>,
    files_run: u64,
    pub(super) provider_time: f64,
    pub(super) provider_rows: u64,
    pub(super) by_rel: HashMap<String, (f64, u64, u64)>,
    /// Thread CPU of everything the finisher did (the second half's own time when it runs inside another walk).
    pub(super) time: f64,
    out: Option<String>,
    stats: Option<String>,
    tally: Option<String>,
    check_out: Option<String>,
    /// redo-alloc: blocks and bytes allocated inside `file` (the second half's own), summed.
    alloc: [u64; 2],

}

#[path = "finisher/worker.rs"]
pub(crate) mod worker;
#[path = "workers.rs"]
pub(crate) mod workers;

use ryo::languages;

impl<'p> Finisher<'p> {
    /// Bind selected emits, applicability and reporting options once for this typed plan.
    pub(super) fn new(plan: &'p finish::Plan, run: finish::Run, args: &[&str], request: Option<(&str, &finish::json_input::Accepted)>) -> Self {
        let t0 = thread_cpu();
        let only: Option<HashSet<String>> = flag(args, "--rules").map(|r| r.split(',').map(str::to_owned).collect());
        let emits: Vec<usize> = (0..plan.emits.len()).filter(|e| only.as_ref().is_none_or(|o| o.contains(&plan.emits[*e].0))).collect();
        let mut me = Self {
            plan,
            emits,
            check: args.contains(&"--check-bases"),
            query_rows: false,
            dump: flag(args, "--dump"),
            dump_chunks: Vec::new(),
            run,
            exec: finish::Exec::new(plan),
            findings: Default::default(),
            failed: HashMap::new(),
            failed_idx: vec![false; plan.emits.len()],
            accepted_only: applicability(plan, flag(args, "--applicability").as_deref(), request),
            rule_stats: vec![[0; 3]; plan.emits.len()],
            check_lines: Vec::new(),
            files_run: 0,
            provider_time: 0.0,
            provider_rows: 0,
            by_rel: HashMap::new(),
            time: 0.0,
            out: flag(args, "--out"),
            stats: flag(args, "--stats"),
            tally: flag(args, "--tally"),
            check_out: flag(args, "--check-out"),
            alloc: [0; 2],
        };
        me.exec.cold_test_load(&mut me.run);
        me.exec.cold_load(&mut me.run);
        me.exec.col_load(&mut me.run);
        me.time += thread_cpu() - t0;
        me
    }

    pub(super) fn selected_emits(&self) -> &[usize] { &self.emits }

    /// The emits retained by the first half for this file.
    pub(super) fn wanted_into(&mut self, mut seeded: impl FnMut(usize) -> bool, out: &mut Vec<usize>) {
        let t0 = thread_cpu();
        out.clear();
        out.extend(self.selected_emits().iter().copied().filter(|&emit| seeded(emit)));
        self.time += thread_cpu() - t0;
    }

    /// redo-general: the plan the second half runs (for a driver that derives its own inputs from it).
    pub(super) fn plan(&self) -> &'p finish::Plan {
        self.plan
    }

    /// Whether a file with these emits to run is visited at all.
    pub(super) fn visits(&self, wanted: &[usize]) -> bool {
        !wanted.is_empty() || self.check
    }

    /// The plan's emits by name with their index into `plan.emits` (for a caller that maps its own rules to them).
    pub(super) fn emit_names(&self) -> impl Iterator<Item = (usize, &str)> {
        self.plan.emits.iter().enumerate().map(|(e, x)| (e, x.0.as_str()))
    }

    pub(super) fn can_finish_exact(&self) -> bool {
        !self.query_rows && self.dump.is_none()
    }

    pub(super) fn emit_exact(&mut self, file: usize, table: &NodeTable, emit: usize, nodes: &[u64]) -> Option<u64> {
        if self.failed_idx[emit] || (self.accepted_only[emit] && languages::node_table::acceptance_of(table) == languages::SyntaxAcceptance::Rejected) {
            return None;
        }
        let mut count = 0;
        super::pair_checks::each(nodes, |i| {
            let span = super::span_bytes(table, i as usize);
            self.findings.location(file, emit, file, span.start as u64, span.end as u64);
            count += 1;
        });
        Some(count)
    }

    /// One file's emits run on its node table and source through the provider `pv`.
    pub(super) fn file(&mut self, file_no: usize, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize]) {
        let a0 = super::alloc_count::counts();
        self.file_inner(file_no, ft, bytes, pv, wanted);
        let a1 = super::alloc_count::counts();
        for i in 0..2 {
            self.alloc[i] += a1[i] - a0[i];
        }
    }

    fn file_inner(&mut self, file_no: usize, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize]) {
        self.files_run += 1;
        let plan = self.plan;
        let mut f = finish::File::new(ft, bytes, file_no as u32, plan.nodes.len());
        if let Some(text) = pv.syntax_text() { f.set_text(text); }
        let run = &mut self.run;
        if self.check {
            for rel in ["semantic-syntax", "syntax-tree", "semantic-child", "syntax-child", "source-location", "token-link"] {
                if run.reader_tables.contains(rel) { continue; }
                let mine = finish::native_table(&mut f, run, rel);
                match pv.fill(run, rel, finish::Scope::File(file_no as u32)) {
                    Ok(theirs) => self.check_lines.push((file_no, format!("{}\t{rel}\t{}", run.paths[file_no], finish::diff(run, &mine, &theirs)))),
                    Err(e) => self.check_lines.push((file_no, format!("{}\t{rel}\tprovider: {e}", run.paths[file_no]))),
                }
            }
        }
        let rejected = languages::node_table::acceptance_of(ft) == languages::SyntaxAcceptance::Rejected;
        for ei in wanted {
            if self.failed_idx[*ei] || (rejected && self.accepted_only[*ei]) {
                continue;
            }
            let e = &plan.emits[*ei];
            run.missing.clear();
            let before = (self.exec.stats.total_runs, self.exec.stats.total_rows);
            let got = self.exec.eval(&mut f, run, pv, e.1);
            let stats = &mut self.rule_stats[*ei];
            stats[0] += 1;
            stats[1] += self.exec.stats.total_runs - before.0;
            stats[2] += self.exec.stats.total_rows - before.1;
            self.exec.emit_done(&f, e.1);
            let got = if run.missing.is_empty() { got } else { Err(format!("functions {:?} not in the step set", run.missing)) };
            let written = got.and_then(|t| {
                if self.dump.is_some() {
                    let mut chunk = String::new();
                    finish::dump_rows(&f, run, &t, &e.0, &run.paths[file_no], &mut chunk);
                    if !chunk.is_empty() {
                        self.dump_chunks.push((file_no, *ei, chunk));
                    }
                    return Ok(());
                }
                if t.n == 0 { return Ok(()); }
                let subject = t.col(&e.2)?;
                let span = t.col("span").ok();
                // The declared subject and key identify a finding. Independent keys
                // may still project to the same diagnostic range.
                let mut identity = vec![subject];
                for name in &e.3 { identity.push(t.col(name)?); }
                let mut seen = rustc_hash::FxHashSet::default();
                for r in 0..t.n {
                    if t.n > 1 && !seen.insert(identity.iter().map(|&c| t.cols[c][r]).collect::<Vec<_>>()) {
                        continue;
                    }
                    if self.query_rows {
                        self.findings.query(run, &t, r, *ei, &e.0, file_no, &identity)?;
                        continue;
                    }
                    let target = if t.tys[subject] == finish::Ty::File {
                        usize::try_from(t.cols[subject][r]).ok().filter(|&id| id < run.paths.len())
                            .ok_or_else(|| format!("file subject outside the run: {}", t.cols[subject][r]))?
                    } else { file_no };
                    let (st, en) = span.map_or((0, 0), |c| (t.cols[c][r] >> 32, t.cols[c][r] & 0xffff_ffff));
                    self.findings.location(file_no, *ei, target, st, en);
                }
                Ok(())
            });
            if let Err(err) = written {
                self.failed.insert(e.0.clone(), (file_no, format!("{}: {err}", run.paths[file_no])));
                self.failed_idx[*ei] = true;
            }
        }
        finish::emptyprop::file_end(&mut self.exec, &mut f, run, pv);
        self.exec.cold_end(&mut f, run);
        self.exec.stats.fact_index_rows += f.fact_index_rows.get();
    }

    /// Write final findings, dumps and statistics after the last file.
    pub(super) fn end(mut self, projection: finish::json_output::Projection<'_>) -> bool {
        let ok = self.failed.is_empty();
        self.exec.leaf_report(&mut self.run);
        self.exec.cold_report(&self.run);
        let total = self.time;
        let file_time = self.time;
        let start = thread_cpu();
        let cutoff: Vec<_> = self.plan.emits.iter().map(|e| self.failed.get(&e.0).map_or(usize::MAX, |r| r.0)).collect();
        let count = self.findings.finish(self.plan, &self.run, self.out.as_deref(), projection, &cutoff);
        println!("ordered findings and write CPU {:.6} s", thread_cpu() - start);
        if let Some(d) = &self.dump {
            use std::io::Write;
            self.dump_chunks.sort_by_key(|(file, _, _)| *file);
            let mut out = std::io::BufWriter::new(std::fs::File::create(d).expect("open dump"));
            for (file, emit, text) in self.dump_chunks {
                if file <= cutoff[emit] {
                    out.write_all(text.as_bytes()).expect("write dump");
                }
            }
            out.flush().expect("close dump");
        }
        if self.check {
            use std::io::Write;
            self.check_lines.sort_by_key(|(file, _)| *file);
            let path = self.check_out.as_deref().unwrap_or("check-bases.tsv");
            let mut out = std::io::BufWriter::new(std::fs::File::create(path).expect("open checks"));
            if self.check_lines.is_empty() {
                writeln!(out).expect("write empty check");
            }
            for (_, line) in self.check_lines {
                writeln!(out, "{line}").expect("write check");
            }
            out.flush().expect("close checks");
        }
        if let Ok(path) = std::env::var("VEC_RULE_STATS") {
            use std::fmt::Write as _;
            let mut data = String::from("rule\tfiles\tstep_runs\trows\n");
            for (emit, [files, runs, rows]) in self.plan.emits.iter().zip(&self.rule_stats) {
                writeln!(data, "{}\t{files}\t{runs}\t{rows}", emit.0).unwrap();
            }
            std::fs::write(path, data).expect("write rule counters");
        }
        let exec = &self.exec;
        exec.col_report();
        if let Some(out) = &self.stats {
            // columns 8 and 13: runs by key, runs by key that returned nothing (finish/keyed.rs); 9 and 10 (made by, why)
            // -1 and empty; 14: rows of tables the step built (a table handed on from an input counts 0)
            let mut lines = vec!["node\top\tcalls\trows\ttouched\tseconds".to_owned()];
            for (i, pn) in self.plan.nodes.iter().enumerate() {
                if exec.stats.calls[i] > 0 || exec.stats.touched[i] > 0 || exec.stats.rows[i] > 0 || exec.stats.time[i] > 0.0 || exec.stats.known[i] > 0 || exec.stats.keyed[i] > 0 {
                    lines.push(format!("{i}\t{}\t{}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{}\t{}", pn.op, exec.stats.calls[i], exec.stats.rows[i], exec.stats.touched[i], exec.stats.time[i], exec.stats.empty[i], exec.stats.keyed[i], -1, "", exec.stats.known[i], exec.stats.empty_time[i], exec.stats.keyed_empty[i], exec.stats.built[i]));
                }
            }
            for (k, v) in &self.by_rel {
                lines.push(format!("fill:{k}\tfill\t{}\t{}\t0\t{:.6}", v.2, v.1, v.0));
            }
            std::fs::write(out, lines.join("\n") + "\n").expect("write stats");
        }
        let steps: f64 = exec.stats.time.iter().sum();
        if let Some(out) = &self.tally {
            let mut lines: Vec<String> = exec.tally.ran.iter().map(|(n, c)| format!("{n}\tran\t{c}")).collect();
            lines.extend(exec.tally.back.iter().map(|((n, why), c)| format!("{n}\tback\t{c}\t{why}")));
            lines.sort();
            std::fs::write(out, lines.join("\n") + "\n").expect("write tally");
        }
        let a = super::alloc_count::counts();
        println!("ALLOC {} process: blocks {} bytes {} peak-live {}; second half (inside the file calls): blocks {} bytes {}", super::alloc_count::NAME, a[0], a[1], a[2], self.alloc[0], self.alloc[1]);
        println!("files run {}; findings {}; rules {} ({} failed)", self.files_run, count, self.emits.len(), self.failed.len());
        println!("thread CPU-s: total {total:.3}; walk+table 0.000; in the visit {file_time:.3}: steps {steps:.3}, provider fills {:.3} ({} rows)", self.provider_time, self.provider_rows);
        // the per-file emptiness decision's static part: steps visited and per-file tests decided (VEC_STEP_TIME: its seconds)
        println!("known-empty static part: {} steps visited ({} with no leaf of their own), {} per-file tests; {:.3} s, of which preparing the needs once a run {:.3} s", exec.stats.static_visits, exec.stats.static_pass, exec.stats.static_file_tests, exec.stats.static_time, exec.stats.rec_time);
        println!("fact keyed lists: fill rows listed {}", self.exec.stats.fact_index_rows);
        println!("{}", finish::fill_prof::reads_line());
        finish::fill_prof::flush();
        for l in finish::fill_prof::unread_lines(&self.run) {
            println!("{l}");
        }
        for l in finish::project::report(&self.run) {
            println!("{l}");
        }
        finish::empty_why::print();
        finish::empty_sel::print();
        finish::emptyprop::print();
        let plan = self.plan;
        exec.print_skipped(&|n| plan.nodes[n].op.clone());
        {
            // redo-union pass 2: per step kind: runs (whole, by key, known empty), runs that returned nothing (known before
            // the bracket, bracketed whole, bracketed by key) and the seconds in the bracketed ones (`VEC_STEP_TIME`)
            let mut by: std::collections::BTreeMap<&str, [f64; 8]> = std::collections::BTreeMap::new();
            for (i, pn) in self.plan.nodes.iter().enumerate() {
                let e = by.entry(pn.op.as_str()).or_default();
                let st = &exec.stats;
                for (k, v) in [st.calls[i] as f64, st.keyed[i] as f64, st.known[i] as f64, st.empty[i] as f64, st.keyed_empty[i] as f64, st.empty_time[i], st.time[i], 0.0].into_iter().enumerate() {
                    e[k] += v;
                }
            }
            println!("KIND\top\twhole\tprobes\tknown-empty\tempty-whole\tempty-probes\tempty-s\tempty-ns-a-run\tseconds");
            for (op, e) in &by {
                if e[0] + e[1] + e[2] == 0.0 {
                    continue;
                }
                let ne = e[3] + e[4];
                println!("KIND\t{op}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.0}\t{:.3}", e[0], e[1], e[2], e[3], e[4], e[5], if ne > 0.0 { e[5] * 1e9 / ne } else { 0.0 }, e[6]);
            }
        }
        let mut f: Vec<_> = self.failed.into_iter().collect();
        f.sort();
        for (r, (_, e)) in f {
            println!("FAILED {r}: {e}");
        }
        // redo-union pass 2 c18: the process ends here: the executor is not dropped one by one
        std::mem::forget(self.exec);
        ok
    }
}

/// redo-honest: per plan emit, whether its rule's applicability is "accepted-syntax" in `path` (a request's
/// `rule_applicability`, or that map alone); none given: false for every emit (every rule runs on every file).
fn applicability(plan: &finish::Plan, path: Option<&str>, request: Option<(&str, &finish::json_input::Accepted)>) -> Vec<bool> {
    let Some(path) = path else { return vec![false; plan.emits.len()] };
    let owned;
    let accepted = match request {
        Some((same, accepted)) if same == path => accepted,
        _ => { owned = finish::json_input::applicability(path); &owned }
    };
    plan.emits.iter().map(|e| accepted.contains(&e.0)).collect()
}
