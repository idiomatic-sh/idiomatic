//! Values that may cross a file-worker boundary. Tables, providers and executors stay local.
use super::{Finisher, HashMap, finish};

pub(crate) struct Settings<'p> {
    plan: &'p finish::Plan,
    layout: std::sync::Arc<finish::Layout>,
    emits: Vec<usize>,
    accepted_only: Vec<bool>,
    disabled: Vec<bool>,
    check: bool,
    query_rows: bool,
    dump: Option<String>,
    project: std::sync::Arc<finish::project::Shared>,
    constants: std::sync::Arc<finish::constants::Constants>,
    static_tests: std::sync::Arc<finish::known::Shared>,
    ids: rustc_hash::FxHashMap<Box<[u8]>, u32>,
    patterns: std::sync::Arc<finish::worker_patterns::Patterns>,
    root: String,
    paths: std::sync::Arc<[String]>,
    text: Vec<Box<[u8]>>,
    kind_sym: Vec<u32>,
    field_sym: Vec<u32>,
    link_sym: [u32; 3],
    empty: u32,
    absent: Option<u32>,
    file_fields: bool,
    reader_tables: rustc_hash::FxHashSet<String>,
    declarations: Vec<(String, Vec<String>, Vec<finish::Ty>, finish::Supplier)>,
}

impl<'p> Finisher<'p> {
    pub(crate) fn worker_settings(&mut self) -> Settings<'p> {
        let prep = super::thread_cpu();
        let r = &mut self.run;
        let declarations = r
            .declared
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|name| {
                let (_, cols, types) = r.catalog(&name).expect("declared relation");
                let supplier = r.supplier(&name);
                (name, (*cols).clone(), (*types).clone(), supplier)
            })
            .collect();
        let settings = Settings {
            project: finish::project::share(r),
            constants: self.exec.share_constants(r),
            static_tests: self.exec.share_static_tests(),
            ids: r.ids.clone(),
            patterns: finish::worker_patterns::Patterns::for_workers(self.plan, r),
            plan: self.plan,
            layout: self.exec.layout.clone(),
            emits: self.emits.clone(),
            accepted_only: self.accepted_only.clone(),
            disabled: self.failed_idx.clone(),
            check: self.check,
            query_rows: self.query_rows,
            dump: self.dump.clone(),
            root: r.root_dir.clone(),
            paths: r.paths.clone(),
            text: r.texts.clone(),
            kind_sym: r.kind_sym.clone(),
            field_sym: r.field_sym.clone(),
            link_sym: r.link_sym,
            empty: r.empty,
            absent: r.absent_word,
            file_fields: r.file_field_codes,
            reader_tables: r.reader_tables.clone(),
            declarations,
        };
        println!(
            "worker shared preparation CPU {:.6} s",
            super::thread_cpu() - prep
        );
        settings
    }
}

impl<'p> Settings<'p> {
    pub(crate) fn open(&self, tokens: Option<std::rc::Rc<dyn finish::TokenOrder>>) -> Finisher<'p> {
        let start = super::thread_cpu();
        let mut run = finish::Run::default();
        run.shared_projects = Some(self.project.clone());
        run.root_dir.clone_from(&self.root);
        run.paths.clone_from(&self.paths);
        run.texts.clone_from(&self.text);
        run.ids.clone_from(&self.ids);
        run.worker_patterns = Some(self.patterns.clone());
        run.empty = self.empty;
        run.absent_word = self.absent;
        run.file_field_codes = self.file_fields;
        run.link_sym = self.link_sym;
        run.kind_sym.clone_from(&self.kind_sym);
        run.field_sym.clone_from(&self.field_sym);
        run.reader_tables.clone_from(&self.reader_tables);
        if let Some(tokens) = tokens {
            run.set_tokens(tokens, self.kind_sym.len());
        }
        for (name, cols, types, supplier) in &self.declarations {
            let columns = cols
                .iter()
                .zip(types)
                .map(|(n, t)| (n.as_str(), *t))
                .collect::<Vec<_>>();
            run.declare(name, &columns, *supplier);
        }
        let mut worker = Finisher {
            plan: self.plan,
            emits: self.emits.clone(),
            check: self.check,
            query_rows: self.query_rows,
            dump: self.dump.clone(),
            dump_chunks: Vec::new(),
            run,
            exec: finish::Exec::with_layout(self.plan, self.layout.clone()),
            findings: Default::default(),
            failed: Default::default(),
            accepted_only: self.accepted_only.clone(),
            failed_idx: self.disabled.clone(),
            rule_stats: vec![[0; 3]; self.plan.emits.len()],
            check_lines: Vec::new(),
            files_run: 0,
            provider_time: 0.,
            provider_rows: 0,
            by_rel: Default::default(),
            time: 0.,
            out: None,
            stats: None,
            tally: None,
            check_out: None,
            alloc: [0; 2],
        };
        worker.exec.shared_constants = Some(self.constants.clone());
        worker.exec.shared_static = Some(self.static_tests.clone());
        worker.exec.cold_test_load(&mut worker.run);
        worker.exec.cold_load(&mut worker.run);
        worker.exec.col_load(&mut worker.run);
        worker.exec.cache_counts(&worker.run, "open");
        worker.time += super::thread_cpu() - start;
        println!("worker executor and dictionary CPU {:.6} s", worker.time);
        worker
    }
}

pub(crate) struct Report {
    findings: finish::json_output::Findings,
    failures: HashMap<String, (usize, String)>,
    rule_stats: Vec<[u64; 3]>,
    checks: Vec<(usize, String)>,
    dump: Vec<(usize, usize, String)>,
    files: u64,
    seconds: f64,
    rows: u64,
    provider_time: f64,
    by_rel: HashMap<String, (f64, u64, u64)>,
    alloc: [u64; 2],
    stats: finish::Stats,
    ran: rustc_hash::FxHashMap<usize, u64>,
    back: rustc_hash::FxHashMap<(usize, &'static str), u64>,
}

impl Finisher<'_> {
    pub(crate) fn report(self) -> Report {
        self.exec.cache_counts(&self.run, "close");
        let start = super::thread_cpu();
        let report = self.into_report();
        println!("worker executor release CPU {:.6} s", super::thread_cpu() - start);
        report
    }

    fn into_report(self) -> Report {
        Report {
            findings: self.findings,
            failures: self.failed,
            rule_stats: self.rule_stats,
            checks: self.check_lines,
            dump: self.dump_chunks,
            files: self.files_run,
            seconds: self.time,
            rows: self.provider_rows,
            provider_time: self.provider_time,
            by_rel: self.by_rel,
            alloc: self.alloc,
            stats: self.exec.stats,
            ran: self.exec.tally.ran,
            back: self.exec.tally.back,
        }
    }
    pub(crate) fn merge(&mut self, r: Report) {
        self.findings.merge(r.findings);
        for (rule, failure) in r.failures {
            let slot = self.failed.entry(rule).or_insert_with(|| failure.clone());
            if failure.0 < slot.0 {
                *slot = failure;
            }
        }
        self.check_lines.extend(r.checks);
        self.dump_chunks.extend(r.dump);
        for (total, part) in self.rule_stats.iter_mut().zip(r.rule_stats) {
            sum(total, &part);
        }
        self.files_run += r.files;
        self.time += r.seconds;
        self.provider_rows += r.rows;
        self.provider_time += r.provider_time;
        for (rel, (time, rows, calls)) in r.by_rel {
            let entry = self.by_rel.entry(rel).or_default();
            entry.0 += time;
            entry.1 += rows;
            entry.2 += calls;
        }
        sum(&mut self.alloc, &r.alloc);
        let total = &mut self.exec.stats;
        let s = r.stats;
        total.fact_index_rows += s.fact_index_rows;
        total.total_runs += s.total_runs;
        total.total_rows += s.total_rows;
        total.static_time += s.static_time;
        total.static_visits += s.static_visits;
        total.static_pass += s.static_pass;
        total.static_file_tests += s.static_file_tests;
        total.rec_time += s.rec_time;
        sum(&mut total.calls, &s.calls);
        sum(&mut total.rows, &s.rows);
        sum(&mut total.touched, &s.touched);
        sum(&mut total.empty, &s.empty);
        sum(&mut total.known, &s.known);
        sum(&mut total.time, &s.time);
        sum(&mut total.empty_time, &s.empty_time);
        sum(&mut total.keyed, &s.keyed);
        sum(&mut total.keyed_empty, &s.keyed_empty);
        sum(&mut total.built, &s.built);
        for (k, v) in r.ran {
            *self.exec.tally.ran.entry(k).or_default() += v;
        }
        for (k, v) in r.back {
            *self.exec.tally.back.entry(k).or_default() += v;
        }
    }
}

fn sum<T: Copy + std::ops::AddAssign>(target: &mut [T], part: &[T]) {
    assert_eq!(target.len(), part.len());
    for (x, &y) in target.iter_mut().zip(part) {
        *x += y;
    }
}
