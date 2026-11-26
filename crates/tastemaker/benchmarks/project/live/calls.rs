//! Typed calls from the pinned check. A refusal returns to the host as an error.
use ryo_executor::bench_support::thread_cpu;
use ryo_executor::check_session::hosted::{HostCell, Parameters};
use ryo_executor::check_session::replay::{self, Active, Authority, Batch, Hosted, Input, LiveAdmission};
mod call_cache;
#[path = "../../../../ryo/crates/engine/benches/lint_image/sdk_request.rs"]
mod sdk_request;
use call_cache::{CallCache, PathFilter};
use crate::linked::Linked;
use release_sdk::linter as sdk;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber,
};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::row_texts::types::Cell;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    rc::Rc,
    time::Instant,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};

use release_sdk::linked_sdk::{finding, report, unknown, unread};

type Reply = Result<sdk::LintReport, sdk::LintError>;

pub fn link_executor() {
    let ledger = std::env::temp_dir().join("ryo-operators.tsv");
    if ledger.is_file() {
        if let Err(error) = ryo_executor::finish::operators::configure(&ledger) {
            eprintln!("operator ledger: {error}");
        }
    }
    release_sdk::linked_sdk::link(execute, |id| {
        measure_worker(|| release_session(id));
    }, retire_session);
}

/// Every image's last act, once its outputs are written: flush, report the outcome as the pinned entry
/// does, and exit without dropping what the session still holds.
pub fn exit_process(outcome: Result<(), Box<dyn std::error::Error>>) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let code = match outcome {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Error: {error:?}");
            1
        }
    };
    let _ = std::io::stderr().flush();
    std::process::exit(code)
}

fn execute(
    id: usize,
    mut requests: Vec<sdk::LintRequest>,
    languages: &sdk::Languages,
    cache: &sdk::Cache,
    workers: std::num::NonZeroUsize,
) -> Vec<Reply> {
    let keys = crate::proof_enabled().then(|| {
        requests.iter_mut().map(|request| {
            // Different runs allocate different names for the same in-memory fixture.
            let root = matches!(request.corpus, sdk::Corpus::Given(_))
                .then(|| std::mem::take(&mut request.root));
            let key = digest(request);
            if let Some(root) = root {
                request.root = root;
            }
            key
        }).collect::<Vec<_>>()
    });
    let answers = measure_worker(|| execute_round(id, requests, languages, cache, workers));
    if let Some(keys) = keys {
        for (key, answer) in keys.into_iter().zip(&answers) {
            eprintln!("TM_ROUND_DIGEST\t{key:x}\t{:x}", digest(answer));
        }
    }
    answers
}

fn digest(value: &impl std::fmt::Debug) -> sha2::digest::Output<sha2::Sha256> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    struct Stream(Sha256);
    impl Write for Stream {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0.update(text.as_bytes());
            Ok(())
        }
    }

    let mut stream = Stream(Sha256::new());
    write!(stream, "{value:?}").expect("the digest writer accepts all text");
    stream.0.finalize()
}

mod sdk;
use sdk::*;

/// Options supplied by the caller of the pinned SDK check.
#[derive(Clone, Copy, Default)]
pub struct LiveOptions {
    pub workers: Option<std::num::NonZeroUsize>,
    pub reverse_files: bool,
}

// The pinned CLI has one check invocation. The SDK receives these values
// explicitly even though its session resides on the host's scheduler thread.
static CHECK_OPTIONS: std::sync::OnceLock<LiveOptions> = std::sync::OnceLock::new();

pub fn configure_bench_check(workers: Option<std::num::NonZeroUsize>, reverse_files: bool) {
    let _ = CHECK_OPTIONS.set(LiveOptions { workers, reverse_files });
}

/// The workers every linked image gives its pool: the request's count, else the CPUs the process may use.
pub fn configured_workers() -> usize {
    check_options()
        .workers
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, std::num::NonZeroUsize::get)
}

pub fn check_options() -> LiveOptions {
    CHECK_OPTIONS.get().copied().unwrap_or_default()
}

/// The languages the session reads: each linked language's native provider (a language linked without one is not read).
pub(crate) fn natives(languages: impl IntoIterator<Item = release_sdk::languages::Language>) -> Vec<Linked> {
    languages.into_iter().filter_map(|language| language.native_provider).map(Linked::from).collect()
}

/// Replay a closed capture with exactly the parsers linked into this release.
pub fn capture(linked: sdk::Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>) {
    capture_then(linked, manifest, output, workers, || {});
}

pub fn capture_then(linked: sdk::Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>, completed: impl FnOnce()) {
    let readers: Vec<_> = natives(linked.0);
    replay::batch_mode_with_workers(&readers, manifest, output, None, workers, completed);
}

pub fn capture_retained(linked:sdk::Languages,manifest:&str,output:&str,root:&str){
    let readers:Vec<_>=natives(linked.0);
    replay::batch_mode(&readers,manifest,output,Some(std::path::Path::new(root)));
}

#[derive(Default)]
struct Measurement {
    sessions: usize,
    rounds: Vec<usize>,
    empty_reader_epochs: usize,
    bulk_screen_files: usize,
    old_route: std::collections::BTreeMap<String, usize>,
    project_counts: [usize; 4],
    calls: usize,
    requests: usize,
    refused: usize,
    reasons: BTreeMap<String, usize>,
    worker_cpu: f64,
    file_shards: usize,
    dispatch_wall: [f64; 5],
    largest_file: String,
    parallel_rows: usize,
    delivery_dictionaries: usize,
    empty_file_epochs: usize,
    omitted_file_bindings: usize,
    admission_shards: usize,
    project_shards: usize,
    project_prepare_wall: f64,
    project_provider_wall: f64,
    project_number_wall: f64,
    project_numbered: usize,
    empty_project_epochs: usize,
    call_wall: f64,
    phase_wall: [f64; 4],
    parts: [f64; 6],
    bound_proofs: f64,
    literal_positions: usize,
    witnesses: [usize; 4],
    named_counts: [usize; 4],
    exact_inputs: [usize; 8],
    named_cpu: f64,
    certified: usize,
    empty: usize,
    requirement_programs: usize,
    requirement_bindings: usize,
    counts: [usize; 10],
    reused_binding_entries: usize,
    pattern_visits: [usize; 4],
    registration: [usize; 10],
    overlay: [usize; 2],
}
static MEASURED: std::sync::Mutex<Measurement> = std::sync::Mutex::new(Measurement {
    sessions: 0,
    rounds: Vec::new(),
    empty_reader_epochs: 0,
    bulk_screen_files: 0,
    old_route: std::collections::BTreeMap::new(),
    project_counts: [0; 4],
    calls: 0,
    requests: 0,
    refused: 0,
    reasons: BTreeMap::new(),
    worker_cpu: 0.0,
    file_shards: 0,
    dispatch_wall: [0.; 5],
    largest_file: String::new(),
    parallel_rows: 0,
    delivery_dictionaries: 0,
    empty_file_epochs: 0,
    omitted_file_bindings: 0,
    admission_shards: 0,
    project_shards: 0,
    project_prepare_wall: 0.,
    project_provider_wall: 0.,
    project_number_wall: 0.,
    project_numbered: 0,
    empty_project_epochs: 0,
    call_wall: 0.0,
    phase_wall: [0.0; 4],
    parts: [0.0; 6],
    bound_proofs: 0.,
    literal_positions: 0,
    witnesses: [0; 4],
    named_counts: [0; 4],
    exact_inputs: [0; 8],
    named_cpu: 0.,
    certified: 0,
    empty: 0,
    requirement_programs: 0,
    requirement_bindings: 0,
    counts: [0; 10],
    reused_binding_entries: 0,
    pattern_visits: [0; 4],
    registration: [0; 10],
    overlay: [0; 2],
});

#[derive(Clone, Copy)]
pub enum HostPhase { Conversion, Validation, Emission }

static HOST: std::sync::Mutex<[f64; 3]> = std::sync::Mutex::new([0.; 3]);

pub fn host_wall(phase: HostPhase, seconds: f64) {
    HOST.lock().expect("host clocks")[phase as usize] += seconds;
}

pub fn host_snapshot() -> (f64, f64) {
    let sdk = MEASURED.lock().expect("SDK clocks").call_wall;
    (sdk, HOST.lock().expect("host clocks")[HostPhase::Conversion as usize])
}

thread_local! {
    static WORK_ON_THIS_THREAD: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

static WORK_NANOS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_NANOS: AtomicU64 = AtomicU64::new(0);

fn measure_worker<T>(work: impl FnOnce() -> T) -> T {
    let start = thread_cpu();
    let result = work();
    let elapsed = thread_cpu() - start;
    WORK_ON_THIS_THREAD.set(WORK_ON_THIS_THREAD.get() + elapsed);
    WORK_NANOS.fetch_add((elapsed * 1e9) as u64, Ordering::Relaxed);
    result
}

/// Include scheduling on the caller, including fixture calls made on the same worker.
pub fn measure_dispatch<T>(lint: impl FnOnce() -> T) -> T {
    let prior = WORK_ON_THIS_THREAD.get();
    let start = thread_cpu();
    let result = lint();
    let overlapped = WORK_ON_THIS_THREAD.get() - prior;
    let dispatch = thread_cpu() - start - overlapped;
    DISPATCH_NANOS.fetch_add((dispatch * 1e9) as u64, Ordering::Relaxed);
    result
}

/// Print the one executor's cumulative counters to the benchmark log after the host renders.
pub fn print_measurement() {
    let [conversion, validation, emission] = *HOST.lock().expect("host clocks");
    eprintln!("LIVE_HOST_WALL conversion={conversion:.6} validation={validation:.6} emission={emission:.6}");
    let m = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (reason, count) in &m.reasons {
        eprintln!("UNANSWERED\t{count}\t{reason}");
    }
    eprintln!("ADMISSION_EPOCH reused_binding_entries={}", m.reused_binding_entries);
    let [before, after, selections, searches] = m.pattern_visits;
    eprintln!("ADMISSION_PATHS prior_visits={before} visits={after} indexed_selections={selections} searches={searches}");
    let [paths, epochs, calls, rules, host_uses, row_hashes, tables, rows, cells, slots] = m.registration;
    eprintln!("ADMISSION_SMALL source_lookups_removed={paths} slot_lookups_removed={slots} epochs={epochs} whole_calls_reused={calls} registrations_removed={rules} bound_host_uses={host_uses} row_hashes_removed={row_hashes} supplied_tables={tables} delivery_rows={rows} delivery_cells={cells}");
    let [files, bytes] = m.overlay;
    eprintln!("ADMISSION_OVERLAY shared_versions={files} duplicate_source_bytes_removed={bytes}");
    let unclaimed = m.old_route.get("unclaimed").copied().unwrap_or(0);
    let taken: usize = m.old_route.iter().filter(|(name, _)| name.as_str() != "unclaimed").map(|(_, count)| count).sum();
    let by_reader: Vec<String> = m.old_route.iter().filter(|(name, count)| name.as_str() != "unclaimed" && **count > 0).map(|(name, count)| format!("{name}:{count}")).collect();
    eprintln!("OLD_ROUTE taken={taken} unclaimed={unclaimed} by_reader={}", by_reader.join(","));
    if m.rounds.iter().any(|&size| size > 1) {
        eprintln!("TM_ROUND_COUNTS epochs={} max_width={} closed_only={} bulk_screen_files={}",
            m.rounds.len(), m.rounds.iter().max().unwrap(), m.empty_reader_epochs, m.bulk_screen_files);
    }
    eprintln!(
        "LIVE_PROJECT_WALL provider={:.6} numbering={:.6} numbered={} empty_epochs={}",
        m.project_provider_wall, m.project_number_wall, m.project_numbered, m.empty_project_epochs
    );
    let [prepare, read, first, second, project, assembly] = m.parts;
    let total = (WORK_NANOS.load(Ordering::Relaxed) + DISPATCH_NANOS.load(Ordering::Relaxed)) as f64 / 1e9;
    let [inventory, join, critical, busy, largest] = m.dispatch_wall;
    eprintln!("LIVE_FILE_WALL inventory={inventory:.6} join={join:.6} critical={critical:.6} busy={busy:.6} largest={largest:.6}");
    if largest > 0. {
        eprintln!("LIVE_LARGEST_FILE {:?}", m.largest_file);
    }
    let [admission_wall, file_wall, project_wall, delivery_wall] = m.phase_wall;
    eprintln!("LIVE_DELIVERY rows={} dictionary_borrows={}", m.parallel_rows, m.delivery_dictionaries);
    eprintln!(
        "LIVE_WORKERS empty_file_epochs={} omitted_file_bindings={} file_shards={} admission_shards={} project_shards={} child_cpu={:.6} project_prepare_wall={:.6} project_execute_wall={:.6}",
        m.empty_file_epochs, m.omitted_file_bindings, m.file_shards, m.admission_shards, m.project_shards, m.worker_cpu,
        m.project_prepare_wall, project_wall - m.project_prepare_wall,
    );
    eprintln!("LIVE_PHASE_WALL calls={:.6} admission={admission_wall:.6} files={file_wall:.6} project={project_wall:.6} delivery={delivery_wall:.6}",m.call_wall);
    let [plans, bindings, sources, reads, parses, pairs, skips, eval, productive, refused] = m.counts;
    let empty = m.empty;
    let [inventories, builds, relations, repeated] = m.project_counts;
    eprintln!(
        "PROJECT_POPULATIONS sessions={} inventories={inventories} builds={builds} relations={relations} repeated_relations={repeated}",
        m.sessions,
    );
    let [pairs, any, all, projected, partial, named, closed, any_partial] = m.exact_inputs;
    eprintln!("EXACT_INPUTS pairs={pairs} any_root={any} all_roots={all} projected={projected} relevant_partial={partial} named={named} closed={closed} any_partial={any_partial}");
    eprintln!("PAIR_WITNESSES unchecked={} absent={} partial={} unverified={} named_batches={} named_relations={} named_rows={} named_empty={} named_cpu={:.6}", m.witnesses[0], m.witnesses[1], m.witnesses[2], m.witnesses[3], m.named_counts[0], m.named_counts[1], m.named_counts[2], m.named_counts[3], m.named_cpu);
    eprintln!(
        "CHECK_SESSION live calls={} requests={} failed_calls={} programs={plans} bindings={bindings} sources={sources} reads={reads} parses={parses} distinct_pairs={pairs} skipped={skips} evaluated={eval} productive={productive} empty={empty} refused={refused} prep={prepare:.6} read_parse={read:.6} first={first:.6} second={second:.6} project={project:.6} assembly={assembly:.6} admission={:.6} bound_proofs={:.6} literal_positions={} certified_pairs={} requirement_programs={} requirement_bindings={}",
        m.calls,
        m.requests,
        m.refused,
        total + m.worker_cpu - m.parts.iter().sum::<f64>(),
        m.bound_proofs,
        m.literal_positions,
        m.certified,
        m.requirement_programs,
        m.requirement_bindings
    );
}

/// Write one fixture's complete rows, through the same reader and executor as the check.
pub fn capture_one(
    linked: sdk::Languages,
    pack: &str,
    input: &str,
    out: &str,
) -> Result<(), String> {
    let readers: Vec<_> = natives(linked.0);
    let bytes = std::fs::read(input).map_err(|e| e.to_string())?;
    let req = ryo_executor::request_input::decode(&bytes).map_err(|e| e.to_string())?;
    replay::one_source(&readers, pack, req, out)
}
