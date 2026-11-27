//! What a session counts of its runs for the images' tools (`print_measurement`): collected only in a build with `bench-hooks`.
//! Every line and every key here is read by a harness (the reader list is in the bead's flag).
use std::collections::BTreeMap;

pub(super) struct Measurement {
    pub(super) rounds: Vec<usize>,
    pub(super) bulk_screen_files: usize,
    pub(super) calls: usize,
    pub(super) requests: usize,
    pub(super) refused: usize,
    pub(super) reasons: BTreeMap<String, usize>,
    pub(super) worker_cpu: f64,
    pub(super) parts: [f64; 6],
    pub(super) bound_proofs: f64,
    pub(super) literal_positions: usize,
    pub(super) distinct_pairs: usize,
    pub(super) certified: usize,
    pub(super) empty: usize,
    pub(super) visit_projects: usize,
    pub(super) counts: [usize; 10],
    pub(super) reused_binding_entries: usize,
}

pub(super) static MEASURED: std::sync::Mutex<Measurement> = std::sync::Mutex::new(Measurement {
    rounds: Vec::new(),
    bulk_screen_files: 0,
    calls: 0,
    requests: 0,
    refused: 0,
    reasons: BTreeMap::new(),
    worker_cpu: 0.0,
    parts: [0.0; 6],
    bound_proofs: 0.,
    literal_positions: 0,
    distinct_pairs: 0,
    certified: 0,
    empty: 0,
    visit_projects: 0,
    counts: [0; 10],
    reused_binding_entries: 0,
});

pub(crate) fn print_measurement() {
    let m = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (reason, count) in &m.reasons {
        eprintln!("UNANSWERED\t{count}\t{reason}");
    }
    eprintln!("ADMISSION_EPOCH reused_binding_entries={}", m.reused_binding_entries);
    if m.rounds.iter().any(|&size| size > 1) {
        eprintln!("TM_ROUND_COUNTS epochs={} bulk_screen_files={}", m.rounds.len(), m.bulk_screen_files);
    }
    let [prepare, read, first, second, project, assembly] = m.parts;
    let [plans, bindings, sources, reads, parses, _pairs, skips, eval, productive, refused] = m.counts;
    eprintln!(
        "CHECK_SESSION live calls={} requests={} failed_calls={} programs={plans} bindings={bindings} sources={sources} reads={reads} parses={parses} distinct_pairs={} skipped={skips} evaluated={eval} productive={productive} empty={} refused={refused} prep={prepare:.6} read_parse={read:.6} first={first:.6} second={second:.6} project={project:.6} assembly={assembly:.6} admission={:.6} bound_proofs={:.6} literal_positions={} certified_pairs={} visit_projects={}",
        m.calls,
        m.requests,
        m.refused,
        m.distinct_pairs,
        m.empty,
        m.worker_cpu - m.parts.iter().sum::<f64>(),
        m.bound_proofs,
        m.literal_positions,
        m.certified,
        m.visit_projects
    );
}
