//! THROWAWAY (ryo-fmx3.1, redo-prefix pass 2): the second half's driver, made once a run and fed one file at a time, so
//! the second half runs either on its own walk (`vec finish`) or inside the first half's visit on the same parsed file
//! (`vec funnel ... --finish`): one parse and one file view a file for both halves.

use super::{ColFacts, HashMap, NodeTable, Vocab, files, finish, flag, thread_cpu};
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
    dump: Option<String>,
    dump_text: String,
    pub(super) run: finish::Run,
    exec: finish::Exec<'p>,
    fwords: languages::FileWords,
    findings: Vec<String>,
    failed: HashMap<String, String>,
    /// redo-honest: per plan emit, whether its rule runs only on a file whose syntax the parser accepted (the request's
    /// `rule_applicability`, `--applicability F`: "accepted-syntax"); without the flag every rule runs on every file.
    accepted_only: Vec<bool>,
    /// redo-union pass 2 c16: a failed emit by its index (no name hashed an emit a file).
    failed_idx: Vec<bool>,
    check_lines: Vec<String>,
    files_run: u64,
    provider_time: f64,
    provider_rows: u64,
    by_rel: HashMap<String, (f64, u64, u64)>,
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
    pub(super) fn new(plan: &'p finish::Plan, walk: &impl languages::Walk, vocab: &Vocab, args: &[&str]) -> Self {
        let t0 = thread_cpu();
        let list = files(args[1]);
        let root = format!("{}/", args[2].trim_end_matches('/'));
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
        let mut run = finish::Run::default();
        run.empty = run.sym(b"");
        run.links();
        run.root_dir = args[2].trim_end_matches('/').to_owned();
        run.search_roots = vec![String::new(), "src".to_owned()];
        run.kind_sym = vocab.kind_names.clone().iter().map(|k| run.sym(k.as_bytes())).collect();
        run.field_sym = vocab.field_names.clone().iter().map(|k| run.sym(k.as_bytes())).collect();
        run.paths = list.iter().map(|p| p.strip_prefix(&root).unwrap_or(p).to_owned()).collect();
        run.path_sym = run.paths.clone().iter().map(|p| run.sym(p.as_bytes())).collect();
        let kind_code = |n: &str| vocab.kinds.get(n).copied().expect("kind");
        run.token_first = kind_code("name-token");
        run.token_last = kind_code("colonequal");
        run.token_error = kind_code("errortoken");
        run.pairs = [(kind_code("lpar"), kind_code("rpar")), (kind_code("lsqb"), kind_code("rsqb")), (kind_code("lbrace"), kind_code("rbrace"))];
        let mut me = Self {
            plan,
            emits,
            emit_seeded: plan.emits.iter().map(|e| plan.seeds.contains(&e.0)).collect(),
            surv,
            surv_by_path,
            check: args.contains(&"--check-bases"),
            dump: flag(args, "--dump"),
            dump_text: String::new(),
            run,
            exec: finish::Exec::new(plan),
            fwords: ryo_python::fixtures::walking::walk_words(walk).expect("the walk's fixed names"),
            findings: Vec::new(),
            failed: HashMap::new(),
            failed_idx: vec![false; plan.emits.len()],
            accepted_only: applicability(plan, flag(args, "--applicability").as_deref()),
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
        me.time += thread_cpu() - t0;
        me
    }

    /// The emits to run on the file at `path`: a seed rule where it has survivors by node; any other where `seeded` says
    /// the first half kept it (`seeded` gets the emit's index into the plan's emits). Empty and no base check: the file
    /// need not be walked for the second half.
    pub(super) fn wanted(&mut self, path: &str, seeded: &dyn Fn(usize) -> bool) -> Vec<usize> {
        let t0 = thread_cpu();
        let surv_here = self.surv_by_path.get(path);
        let w: Vec<usize> = self
            .emits
            .iter()
            .copied()
            .filter(|e| {
                let name = &self.plan.emits[*e].0;
                if self.emit_seeded[*e] { surv_here.is_some_and(|s| s.contains(name)) } else { seeded(*e) }
            })
            .collect();
        self.time += thread_cpu() - t0;
        w
    }

    /// Whether a file with these emits to run is visited at all.
    pub(super) fn visits(&self, wanted: &[usize]) -> bool {
        !wanted.is_empty() || self.check
    }

    /// The plan's emits by name with their index into `plan.emits` (for a caller that maps its own rules to them).
    pub(super) fn emit_names(&self) -> impl Iterator<Item = (usize, &str)> {
        self.plan.emits.iter().enumerate().map(|(e, x)| (e, x.0.as_str()))
    }

    /// The second half on one walked file of the direct-columns walk (its own model by parts, built on demand).
    pub(super) fn column_file(&mut self, file_no: usize, path: &str, file: &mut ryo_python::fixtures::columns::ColumnFile<'_, '_>, bytes: &[u8], wanted: &[usize]) {
        let t0 = thread_cpu();
        let ft = file.table;
        let mut fwords = std::mem::take(&mut self.fwords);
        languages::facts::clear_words(&mut fwords);
        let mut pv = ColFacts { file, words: &mut fwords, filled: languages::FilledColumns::default(), time: 0.0, rows: 0, by_rel: HashMap::new(), file_no: file_no as u32, kept: HashMap::new() };
        self.file(file_no, path, ft, bytes, &mut pv, wanted);
        self.provider_time += pv.time;
        self.provider_rows += pv.rows;
        for (k, v) in pv.by_rel {
            let e = self.by_rel.entry(k).or_default();
            e.0 += v.0;
            e.1 += v.1;
            e.2 += v.2;
        }
        self.fwords = fwords;
        self.time += thread_cpu() - t0;
    }

    /// The second half on one file through the reader's facts (`vec finish` without `VEC_COLWALK`).
    pub(super) fn reader_file(&mut self, file_no: usize, path: &str, file: &mut languages::FileFacts<'_>, bytes: &[u8], wanted: &[usize]) {
        let t0 = thread_cpu();
        let ft = file.table;
        let mut fwords = std::mem::take(&mut self.fwords);
        languages::facts::clear_words(&mut fwords);
        let mut pv = super::Facts { facts: file, words: &mut fwords, filled: languages::FilledColumns::default(), time: 0.0, rows: 0, by_rel: HashMap::new(), file: file_no as u32 };
        self.file(file_no, path, ft, bytes, &mut pv, wanted);
        self.provider_time += pv.time;
        self.provider_rows += pv.rows;
        for (k, v) in pv.by_rel {
            let e = self.by_rel.entry(k).or_default();
            e.0 += v.0;
            e.1 += v.1;
            e.2 += v.2;
        }
        self.fwords = fwords;
        self.time += thread_cpu() - t0;
    }

    /// One file's emits run on its node table and source through the provider `pv`.
    fn file(&mut self, file_no: usize, path: &str, ft: &NodeTable, bytes: &[u8], pv: &mut dyn finish::Provider, wanted: &[usize]) {
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
        // redo-union pass 2 c12: no survivor lists given: every set empty, without a lookup a set a file
        f.seeds = if self.surv.is_empty() { vec![Vec::new(); plan.seeds.len()] } else { plan.seeds.iter().map(|r| self.surv.get(&(r.clone(), path.to_owned())).cloned().unwrap_or_default()).collect() };
        finish::handover::expand(&mut f, &plan.seed_paths);
        self.exec.reset_file(f.n);
        let run = &mut self.run;
        if self.check {
            for rel in ["semantic-syntax", "syntax-tree", "semantic-child", "syntax-child", "source-location", "token-link"] {
                let mine = finish::native_table(&mut f, run, rel);
                match pv.fill(run, rel) {
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
            let got = self.exec.eval(&mut f, run, pv, e.1);
            let got = if run.missing.is_empty() { got } else { Err(format!("functions {:?} not in the step set", run.missing)) };
            match got {
                Ok(t) => {
                    if self.dump.is_some() {
                        finish::dump_rows(&f, run, &t, &e.0, &run.paths[file_no], &mut self.dump_text);
                        continue;
                    }
                    let sc = t.col("span").ok();
                    for r in 0..t.n {
                        let (st, en) = sc.map_or((0, 0), |c| (t.cols[c][r] >> 32, t.cols[c][r] & 0xffff_ffff));
                        self.findings.push(format!("{{\"rule\":\"{}\",\"file\":\"{}\",\"start\":{st},\"end\":{en}}}", e.0, run.paths[file_no]));
                    }
                }
                Err(err) => {
                    self.failed.insert(e.0.clone(), format!("{}: {err}", run.paths[file_no]));
                    self.failed_idx[*ei] = true;
                }
            }
        }
    }

    /// Writes the findings, dumps and stats, and prints the second half's lines; `walk_time` is the walk's own time when
    /// the second half walked the files itself (0 inside the first half's walk).
    pub(super) fn end(mut self, walk_time: f64) {
        let total = self.time + walk_time;
        let file_time = self.time;
        self.findings.sort();
        self.findings.dedup();
        if let Some(d) = &self.dump {
            std::fs::write(d, &self.dump_text).expect("write dump");
        }
        if let Some(out) = &self.out {
            std::fs::write(out, self.findings.join("\n") + "\n").expect("write findings");
        }
        if self.check {
            std::fs::write(self.check_out.clone().unwrap_or_else(|| "check-bases.tsv".to_owned()), self.check_lines.join("\n") + "\n").expect("write check");
        }
        let exec = &self.exec;
        if let Some(out) = &self.stats {
            let mut lines = vec!["node\top\tcalls\trows\ttouched\tseconds".to_owned()];
            for (i, pn) in self.plan.nodes.iter().enumerate() {
                if exec.stats.calls[i] > 0 || exec.stats.probes[i] > 0 || exec.stats.touched[i] > 0 || exec.stats.rows[i] > 0 || exec.stats.time[i] > 0.0 || exec.stats.known[i] > 0 {
                    lines.push(format!("{i}\t{}\t{}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{}", pn.op, exec.stats.calls[i], exec.stats.rows[i], exec.stats.touched[i], exec.stats.time[i], exec.stats.empty[i], exec.stats.probes[i], exec.stats.made_by[i] as i64, exec.stats.why[i], exec.stats.known[i], exec.stats.empty_time[i], exec.stats.probe_empty[i]));
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
        println!("per-key memo: rows in {} rows out {} keys asked {} keys missing {} probes answered whole {}", exec.stats.memo_in, exec.stats.memo_out, exec.stats.memo_keys, exec.stats.memo_missing, exec.stats.memo_whole_hits);
        println!("memo key lists answered whole: {} ({} keys)", exec.stats.memo_reused, exec.stats.memo_reused_keys);
        if exec.stats.known_timed > 0 {
            // redo-union pass 2: the clock's pair cost, calibrated here (a million back-to-back pairs)
            let n = 1_000_000;
            let t0 = thread_cpu();
            let mut x = 0.0;
            for _ in 0..n {
                let a = thread_cpu();
                x += thread_cpu() - a;
            }
            let pair = (thread_cpu() - t0) / f64::from(n);
            std::hint::black_box(x);
            let per = exec.stats.known_time / exec.stats.known_timed as f64;
            println!("known-empty whole runs (decided before): {} timed, {:.0} ns a run measured, clock pair {:.0} ns, {:.0} ns a run net; deciding the static part a file: {:.3} s over {} steps ({} per-file tests), of which preparing the needs once a run {:.3} s", exec.stats.known_timed, per * 1e9, pair * 1e9 / 2.0, (per - pair / 2.0) * 1e9, exec.stats.static_time, exec.stats.static_visits, exec.stats.static_file_tests, exec.stats.rec_time);
        }
        println!("fact keyed lists: fill rows listed {}", finish::FACT_IX_ROWS.load(std::sync::atomic::Ordering::Relaxed));
        finish::empty_why::print();
        finish::empty_sel::print();
        if std::env::var_os("VEC_MEMO_DUMP").is_some() {
            // redo-union pass 2 c19
            for l in exec.memo_dump() {
                println!("{l}");
            }
        }
        {
            // redo-union pass 2: per step kind: runs (whole, by key, known empty), runs that returned nothing (known before
            // the bracket, bracketed whole, bracketed by key) and the seconds in the bracketed ones (`VEC_STEP_TIME`)
            let mut by: std::collections::BTreeMap<&str, [f64; 8]> = std::collections::BTreeMap::new();
            for (i, pn) in self.plan.nodes.iter().enumerate() {
                let e = by.entry(pn.op.as_str()).or_default();
                let st = &exec.stats;
                for (k, v) in [st.calls[i] as f64, st.probes[i] as f64, st.known[i] as f64, st.empty[i] as f64, st.probe_empty[i] as f64, st.empty_time[i], st.time[i], 0.0].into_iter().enumerate() {
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
    }
}

/// redo-honest: per plan emit, whether its rule's applicability is "accepted-syntax" in `path` (a request's
/// `rule_applicability`, or that map alone); none given: false for every emit (every rule runs on every file).
fn applicability(plan: &finish::Plan, path: Option<&str>) -> Vec<bool> {
    let Some(path) = path else { return vec![false; plan.emits.len()] };
    let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).expect("applicability file")).expect("applicability json");
    let map = j.get("rule_applicability").unwrap_or(&j);
    plan.emits.iter().map(|e| map.get(&e.0).and_then(serde_json::Value::as_str) == Some("accepted-syntax")).collect()
}
