//! THROWAWAY (ryo-fmx3.1, redo-prefix pass 2): the second half's driver, made once a run and fed one file at a time, so
//! the second half runs either on its own walk (`vec finish`) or inside the first half's visit on the same parsed file
//! (`vec funnel ... --finish`): one parse and one file view a file for both halves.

use super::{finish, flag, thread_cpu};
use std::collections::HashMap;
use ryo::languages::NodeTable;
use std::collections::HashSet;

/// The second half of a run: the plan's emits to run, the run's symbols, the executor, and what it found.
pub(super) struct Finisher<'p> {
    plan: &'p finish::Plan,
    /// The plan's emits the run asks for (`--rules`), by index into `plan.emits`.
    emits: Vec<usize>,
    /// The plan's seed rules (their rows start from the first half's survivors by node), by name.
    /// redo-union pass 2 c12: per plan emit, whether its rule is a seed rule (decided once a run, not looked up a file).
    emit_seeded: Vec<bool>,
    /// The survivors by node of each (rule, path) (`--surv`), and their rules by path.
    surv: HashMap<(String, String), Vec<u32>>,
    surv_by_path: HashMap<String, HashSet<String>>,
    check: bool,
    pub(super) query_rows: bool,
    dump: Option<String>,
    dump_text: String,
    pub(super) run: finish::Run,
    exec: finish::Exec<'p>,
    findings: Vec<String>,
    quoted_rules: Vec<std::cell::OnceCell<String>>,
    quoted_paths: Vec<std::cell::OnceCell<String>>,
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
    /// `args` as `vec finish` takes them: `<plan> <files> <root> [--rules A,B] [--surv F] [--check-bases] [--dump F]
    /// [--out F] [--stats F] [--tally F] [--check-out F]`; `plan` is `args[0]` loaded.
    pub(super) fn new(plan: &'p finish::Plan, run: finish::Run, args: &[&str], request: Option<(&str, &serde_json::Value)>) -> Self {
        let t0 = thread_cpu();
        let only: Option<HashSet<String>> = flag(args, "--rules").map(|r| r.split(',').map(str::to_owned).collect());
        let emits: Vec<usize> = (0..plan.emits.len()).filter(|e| only.as_ref().is_none_or(|o| o.contains(&plan.emits[*e].0))).collect();
        // the survivor handoff: per (rule, file) the prefix's root nodes (`vec funnel ... --surv-out`); a seeded rule runs
        // only where it has survivors, from them
        let mut surv: HashMap<(String, String), Vec<u32>> = HashMap::new();
        if let Some(p) = flag(args, "--surv") {
            for l in std::fs::read_to_string(p).expect("survivors").lines() {
                let mut it = l.split('\t');
                let (Some(r), Some(f), Some(ns)) = (it.next(), it.next(), it.next()) else { continue };
                surv.entry((r.to_owned(), f.to_owned())).or_default().extend(ns.split(' ').filter_map(|x| x.parse::<u32>().ok()));
            }
            for v in surv.values_mut() {
                v.sort_unstable();
                v.dedup();
            }
        }
        let mut surv_by_path: HashMap<String, HashSet<String>> = HashMap::new();
        for (r, f) in surv.keys() {
            surv_by_path.entry(f.clone()).or_default().insert(r.clone());
        }
        let quoted_paths = vec![std::cell::OnceCell::new(); run.paths.len()];
        let mut me = Self {
            quoted_paths,
            quoted_rules: vec![std::cell::OnceCell::new(); plan.emits.len()],
            plan,
            emits,
            emit_seeded: plan.emits.iter().map(|e| plan.seeds.contains(&e.0)).collect(),
            surv,
            surv_by_path,
            check: args.contains(&"--check-bases"),
            query_rows: false,
            dump: flag(args, "--dump"),
            dump_text: String::new(),
            run,
            exec: finish::Exec::new(plan),
            findings: Vec::new(),
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

    /// The emits to run on the file at `path`: a seed rule where it has survivors by node; any other where `seeded` says
    /// the first half kept it (`seeded` gets the emit's index into the plan's emits). Empty and no base check: the file
    /// need not be walked for the second half.
    pub(super) fn wanted(&mut self, path: &str, seeded: &dyn Fn(usize) -> bool) -> Vec<usize> {
        let mut out = Vec::new();
        self.wanted_into(path, seeded, &mut out);
        out
    }

    pub(super) fn wanted_into(&mut self, path: &str, mut seeded: impl FnMut(usize) -> bool, out: &mut Vec<usize>) {
        let t0 = thread_cpu();
        out.clear();
        if self.plan.seeds.is_empty() {
            out.extend(self.emits.iter().copied().filter(|&emit| seeded(emit)));
        } else {
            let surv_here = self.surv_by_path.get(path);
            out.extend(self.emits.iter().copied().filter(|&emit| {
                if self.emit_seeded[emit] {
                    surv_here.is_some_and(|s| s.contains(&self.plan.emits[emit].0))
                } else {
                    seeded(emit)
                }
            }));
        }
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
        !self.query_rows && self.dump.is_none() && self.surv.is_empty()
    }

    pub(super) fn emit_exact(&mut self, file: usize, table: &NodeTable, emit: usize, nodes: &[u64]) -> Option<u64> {
        if self.failed_idx[emit] || (self.accepted_only[emit] && languages::node_table::acceptance_of(table) == languages::SyntaxAcceptance::Rejected) {
            return None;
        }
        let mut count = 0;
        super::pair_checks::each(nodes, |i| {
            let rule = self.quoted_rules[emit].get_or_init(|| serde_json::to_string(&self.plan.emits[emit].0).expect("rule string"));
            let path = self.quoted_paths[file].get_or_init(|| serde_json::to_string(&self.run.paths[file]).expect("path string"));
            let span = super::span_bytes(table, i as usize);
            self.findings.push(format!(r#"{{"rule":{rule},"file":{path},"start":{},"end":{}}}"#, span.start, span.end));
            count += 1;
        });
        Some(count)
    }

    /// One file's emits run on its node table and source through the provider `pv`.
    pub(super) fn file(&mut self, file_no: usize, path: &str, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize]) {
        let a0 = super::alloc_count::counts();
        self.file_inner(file_no, path, ft, bytes, pv, wanted);
        let a1 = super::alloc_count::counts();
        for i in 0..2 {
            self.alloc[i] += a1[i] - a0[i];
        }
    }

    fn file_inner(&mut self, file_no: usize, path: &str, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize]) {
        self.files_run += 1;
        let plan = self.plan;
        let mut f = finish::File::new(ft, bytes, file_no as u32, plan.nodes.len());
        if let Some(text) = pv.syntax_text() { f.set_text(text); }
        // redo-union pass 2 c12: no survivor lists given: every set empty, without a lookup a set a file
        f.seeds = if self.surv.is_empty() { vec![Vec::new(); plan.seeds.len()] } else { plan.seeds.iter().map(|r| self.surv.get(&(r.clone(), path.to_owned())).cloned().unwrap_or_default()).collect() };
        finish::handover::expand(&mut f, &plan.seed_paths);
        let run = &mut self.run;
        if self.check {
            for rel in ["semantic-syntax", "syntax-tree", "semantic-child", "syntax-child", "source-location", "token-link"] {
                if run.reader_tables.contains(rel) { continue; }
                let mine = finish::native_table(&mut f, run, rel);
                match pv.fill(run, rel, finish::Scope::File(file_no as u32)) {
                    Ok(theirs) => self.check_lines.push(format!("{}\t{rel}\t{}", run.paths[file_no], finish::diff(run, &mine, &theirs))),
                    Err(e) => self.check_lines.push(format!("{}\t{rel}\tprovider: {e}", run.paths[file_no])),
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
                    finish::dump_rows(&f, run, &t, &e.0, &run.paths[file_no], &mut self.dump_text);
                    return Ok(());
                }
                if t.n == 0 { return Ok(()); }
                let subject = t.col(&e.2)?;
                let span = t.col("span").ok();
                let rule = self.quoted_rules[*ei].get_or_init(|| serde_json::to_string(&e.0).expect("rule string"));
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
                        self.findings.push(finish::answer::row(run, &t, r, &e.0, Some(file_no), &identity)?);
                        continue;
                    }
                    let target = if t.tys[subject] == finish::Ty::File {
                        usize::try_from(t.cols[subject][r]).ok().filter(|&id| id < run.paths.len())
                            .ok_or_else(|| format!("file subject outside the run: {}", t.cols[subject][r]))?
                    } else { file_no };
                    let (st, en) = span.map_or((0, 0), |c| (t.cols[c][r] >> 32, t.cols[c][r] & 0xffff_ffff));
                    let file = self.quoted_paths[target].get_or_init(|| serde_json::to_string(&run.paths[target]).expect("path string"));
                    self.findings.push(format!("{{\"rule\":{rule},\"file\":{file},\"start\":{st},\"end\":{en}}}"));
                }
                Ok(())
            });
            if let Err(err) = written {
                self.failed.insert(e.0.clone(), format!("{}: {err}", run.paths[file_no]));
                self.failed_idx[*ei] = true;
            }
        }
        finish::emptyprop::file_end(&mut self.exec, &mut f, run, pv);
        self.exec.cold_end(&mut f, run);
    }

    /// Writes the findings, dumps and stats, and prints the second half's lines; `walk_time` is the walk's own time when
    /// the second half walked the files itself (0 inside the first half's walk).
    pub(super) fn end(mut self, walk_time: f64, project_findings: impl FnOnce(&mut Vec<String>)) -> bool {
        let ok = self.failed.is_empty();
        self.exec.leaf_report(&mut self.run);
        self.exec.cold_report(&self.run);
        let total = self.time + walk_time;
        let file_time = self.time;
        self.findings.sort();
        project_findings(&mut self.findings);
        if let Some(d) = &self.dump {
            std::fs::write(d, &self.dump_text).expect("write dump");
        }
        if let Some(out) = &self.out {
            std::fs::write(out, self.findings.join("\n") + "\n").expect("write findings");
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
        println!("files run {}; findings {}; rules {} ({} failed)", self.files_run, self.findings.len(), self.emits.len(), self.failed.len());
        println!("thread CPU-s: total {total:.3}; walk+table {walk_time:.3}; in the visit {file_time:.3}: steps {steps:.3}, provider fills {:.3} ({} rows)", self.provider_time, self.provider_rows);
        // the per-file emptiness decision's static part: steps visited and per-file tests decided (VEC_STEP_TIME: its seconds)
        println!("known-empty static part: {} steps visited ({} with no leaf of their own), {} per-file tests; {:.3} s, of which preparing the needs once a run {:.3} s", exec.stats.static_visits, exec.stats.static_pass, exec.stats.static_file_tests, exec.stats.static_time, exec.stats.rec_time);
        println!("fact keyed lists: fill rows listed {}", finish::FACT_IX_ROWS.load(std::sync::atomic::Ordering::Relaxed));
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
fn applicability(plan: &finish::Plan, path: Option<&str>, request: Option<(&str, &serde_json::Value)>) -> Vec<bool> {
    let Some(path) = path else { return vec![false; plan.emits.len()] };
    let owned;
    let j = match request {
        Some((same, value)) if same == path && value.get("rule_applicability").is_some() => value,
        _ => {
            owned = serde_json::from_slice(&std::fs::read(path).expect("applicability file")).expect("applicability json");
            &owned
        }
    };
    let map = j.get("rule_applicability").unwrap_or(j);
    plan.emits.iter().map(|e| map.get(&e.0).and_then(serde_json::Value::as_str) == Some("accepted-syntax")).collect()
}
