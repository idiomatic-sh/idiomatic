//! What a session counts of its runs, which the images' tools print (`print_measurement`): the counters the executor reports per request.
#![allow(dead_code, reason = "a counter an image does not print is still kept")]
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// The CPU nanoseconds the workers spent in the session, and the dispatching thread's own: added by the images' wrappers.
static WORK_NANOS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_NANOS: AtomicU64 = AtomicU64::new(0);

/// Adds worker CPU nanoseconds to the measurement.
#[doc(hidden)]
pub fn add_work_nanos(nanos: u64) {
    let _ = WORK_NANOS.fetch_add(nanos, Ordering::Relaxed);
}

/// Adds dispatch CPU nanoseconds to the measurement.
#[doc(hidden)]
pub fn add_dispatch_nanos(nanos: u64) {
    let _ = DISPATCH_NANOS.fetch_add(nanos, Ordering::Relaxed);
}

#[derive(Default)]
pub(super) struct Measurement {
    pub(super) sessions: usize,
    pub(super) rounds: Vec<usize>,
    pub(super) empty_reader_epochs: usize,
    pub(super) bulk_screen_files: usize,
    pub(super) old_route: BTreeMap<String, usize>,
    pub(super) project_counts: [usize; 4],
    pub(super) calls: usize,
    pub(super) requests: usize,
    pub(super) refused: usize,
    pub(super) reasons: BTreeMap<String, usize>,
    pub(super) worker_cpu: f64,
    pub(super) file_shards: usize,
    pub(super) dispatch_wall: [f64; 5],
    pub(super) largest_file: String,
    pub(super) parallel_rows: usize,
    pub(super) delivery_dictionaries: usize,
    pub(super) empty_file_epochs: usize,
    pub(super) omitted_file_bindings: usize,
    pub(super) admission_shards: usize,
    pub(super) project_shards: usize,
    pub(super) project_prepare_wall: f64,
    pub(super) project_provider_wall: f64,
    pub(super) project_number_wall: f64,
    pub(super) project_numbered: usize,
    pub(super) empty_project_epochs: usize,
    pub(super) call_wall: f64,
    pub(super) phase_wall: [f64; 4],
    pub(super) parts: [f64; 6],
    pub(super) bound_proofs: f64,
    pub(super) literal_positions: usize,
    pub(super) witnesses: [usize; 4],
    pub(super) named_counts: [usize; 4],
    pub(super) exact_inputs: [usize; 8],
    pub(super) named_cpu: f64,
    pub(super) certified: usize,
    pub(super) empty: usize,
    pub(super) requirement_programs: usize,
    pub(super) requirement_bindings: usize,
    pub(super) counts: [usize; 10],
    pub(super) reused_binding_entries: usize,
    pub(super) pattern_visits: [usize; 4],
    pub(super) registration: [usize; 10],
    pub(super) overlay: [usize; 2],
}
pub(super) static MEASURED: std::sync::Mutex<Measurement> = std::sync::Mutex::new(Measurement {
    sessions: 0,
    rounds: Vec::new(),
    empty_reader_epochs: 0,
    bulk_screen_files: 0,
    old_route: BTreeMap::new(),
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


#[doc(hidden)]
pub fn print_measurement() {
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
    let [plans, bindings, sources, reads, parses, _pairs, skips, eval, productive, refused] = m.counts;
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
