//! File-local execution over the first walk's node table. One driver serves the run.

#[path = "retained.rs"]
pub(crate) mod retained;
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
    dump_text: String,
    pub(super) run: finish::Run,
    exec: finish::Exec<'p>,
    findings: finish::json_output::Findings,
    pub(crate) capture: Option<retained::Capture>,
    failed: HashMap<String, String>,
    /// redo-honest: per plan emit, whether its rule runs only on a file whose syntax the parser accepted (the request's
    /// `rule_applicability`, `--applicability F`: "accepted-syntax"); without the flag every rule runs on every file.
    accepted_only: Vec<bool>,
    /// redo-union pass 2 c16: a failed emit by its index (no name hashed an emit a file).
    failed_idx: Vec<bool>,
    rule_stats: Vec<[u64; 3]>,
    check_lines: Vec<String>,
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
            dump_text: String::new(),
            run,
            exec: finish::Exec::new(plan),
            findings: Default::default(),
            capture: None,
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

    pub(crate) fn retained_file(&mut self, origin: &str) -> bool {
        self.capture.as_mut().is_some_and(|c| c.replay(origin, &mut self.findings.restored, &mut self.failed))
    }

    /// The emits retained by the first half for this file.
    pub(super) fn wanted_into(&mut self, mut seeded: impl FnMut(usize) -> bool, out: &mut Vec<usize>) {
        let t0 = thread_cpu();
        out.clear();
        out.extend(self.emits.iter().copied().filter(|&emit| seeded(emit)));
        self.time += thread_cpu() - t0;
    }

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
        if self.failed_idx[emit] {
            if let Some(capture) = &mut self.capture { capture.partial(&self.run.paths[file]); }
            return None;
        }
        if self.accepted_only[emit] && languages::node_table::acceptance_of(table) == languages::SyntaxAcceptance::Rejected {
            return None;
        }
        let mut count = 0;
        super::pair_checks::each(nodes, |i| {
            let span = super::span_bytes(table, i as usize);
            if let Some(c) = &mut self.capture {
                let origin = &self.run.paths[file];
                c.finding(origin, retained::Finding { rule: self.plan.emits[emit].0.clone(), file: origin.clone(), start: span.start as u64, end: span.end as u64 });
            }
            if self.capture.is_some() && self.failed.contains_key(&self.plan.emits[emit].0) { return; }
            self.findings.location(emit, file, span.start as u64, span.end as u64);
            count += 1;
        });
        Some(count)
    }

    /// One file's emits run on its node table and source through the provider `pv`.
    pub(super) fn file(&mut self, file_no: usize, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize], indexed: Option<(&dyn finish::request::Indexed, &[languages::Relation])>, empty: &[&str]) -> u64 {
        let a0 = super::alloc_count::counts();
        let widened = self.file_inner(file_no, ft, bytes, pv, wanted, indexed, empty);
        let a1 = super::alloc_count::counts();
        for i in 0..2 {
            self.alloc[i] += a1[i] - a0[i];
        }
        widened
    }

    fn file_inner(&mut self, file_no: usize, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize], indexed: Option<(&dyn finish::request::Indexed, &[languages::Relation])>, empty: &[&str]) -> u64 {
        self.files_run += 1;
        let plan = self.plan;
        let mut version = finish::FileVersion::new(ft, bytes, file_no as u32, pv.syntax_text());
        version.bind_packet(indexed.map(|(packet, _)| packet));
        let mut f = finish::File::new(&version, plan.nodes.len());
        let run = &mut self.run;
        let checks: Vec<_> = if self.check {
            ["semantic-syntax", "syntax-tree", "semantic-child", "syntax-child", "source-location", "token-link"]
                .into_iter().filter(|rel| !run.reader_tables.contains(*rel)).collect()
        } else {
            Vec::new()
        };
        let rejected = languages::node_table::acceptance_of(ft) == languages::SyntaxAcceptance::Rejected;
        let roots = wanted.iter().copied().filter(|&i| !self.failed_idx[i] && !(rejected && self.accepted_only[i]))
            .map(|i| plan.emits[i].1);
        let replies = match indexed {
            Some((_, facts)) => self.exec.open_indexed(&version, run, pv, facts, &checks),
            None => self.exec.open_file(&mut f, run, pv, roots, &checks, Some(empty)),
        }.expect("file provider packet");
        for (rel, reply) in checks.into_iter().zip(replies) {
            let mine = finish::native_table(&mut f, run, rel);
            match reply.and_then(finish::request::Answer::columns) {
                Ok(theirs) => self.check_lines.push(format!("{}\t{rel}\t{}", run.paths[file_no], finish::diff(run, &mine, &theirs))),
                Err(e) => self.check_lines.push(format!("{}\t{rel}\tprovider: {e}", run.paths[file_no])),
            }
        }
        for ei in wanted {
            if self.failed_idx[*ei] {
                if let Some(capture) = &mut self.capture { capture.partial(&run.paths[file_no]); }
                continue;
            }
            if rejected && self.accepted_only[*ei] {
                continue;
            }
            let e = &plan.emits[*ei];
            run.provider_refusal = None;
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
                    finish::dump_rows(&f, run, &t, &e.0, &run.paths[file_no], &mut self.dump_text);
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
                        self.findings.rows.query(run, &t, r, &e.0, Some(file_no), &identity)?;
                        continue;
                    }
                    let target = if t.tys[subject] == finish::Ty::File {
                        usize::try_from(t.cols[subject][r]).ok().filter(|&id| id < run.paths.len())
                            .ok_or_else(|| format!("file subject outside the run: {}", t.cols[subject][r]))?
                    } else { file_no };
                    let (st, en) = span.map_or((0, 0), |c| (t.cols[c][r] >> 32, t.cols[c][r] & 0xffff_ffff));
                    if let Some(c) = &mut self.capture {
                        c.finding(&run.paths[file_no], retained::Finding { rule: e.0.clone(), file: run.paths[target].clone(), start: st, end: en });
                    }
                    if self.capture.is_some() && self.failed.contains_key(&e.0) { continue; }
                    self.findings.location(*ei, target, st, en);
                }
                Ok(())
            });
            if let Err(err) = written {
                if let Some(c) = &mut self.capture && run.provider_refusal.as_ref() == Some(&err) {
                    c.refusal(&run.paths[file_no], &e.0, &err);
                } else {
                    if let Some(c) = &mut self.capture { c.failed(&run.paths[file_no]); }
                    self.failed_idx[*ei] = true;
                }
                self.failed.entry(e.0.clone()).or_insert_with(|| format!("{}: {err}", run.paths[file_no]));
            }
        }
        finish::emptyprop::file_end(&mut self.exec, &mut f, run, pv);
        self.exec.cold_end(&mut f, run);
        self.exec.close_file(&version)
    }

    /// Write final findings, dumps and statistics after the last file.
    pub(super) fn end(mut self, projection: finish::json_output::Projection<'_>) -> bool {
        let ok = self.failed.is_empty();
        self.exec.leaf_report(&mut self.run);
        self.exec.cold_report(&self.run);
        let total = self.time;
        let file_time = self.time;
        let count = self.findings.finish(self.plan, &self.run, self.out.as_deref(), projection);
        if let Some(d) = &self.dump {
            std::fs::write(d, &self.dump_text).expect("write dump");
        }
        if self.check {
            std::fs::write(self.check_out.clone().unwrap_or_else(|| "check-bases.tsv".to_owned()), self.check_lines.join("\n") + "\n").expect("write check");
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
        println!("fact keyed lists: fill rows listed {}", finish::FACT_IX_ROWS.load(std::sync::atomic::Ordering::Relaxed));
        println!("{}", finish::fill_prof::reads_line());
        if finish::fill_prof::counting() { println!("{}", self.exec.request_line()); }
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
        for (r, e) in f {
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
