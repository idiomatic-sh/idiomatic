//! Doubles that observe how the inventory is verified: how many worker runs overlap, and what every published summary
//! holds while another publication may be under way.
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::inventory::{
    interface::InventoryPort,
    types::{
        CaseDocuments, ContextFiles, ProfileProjection, RawArtifacts, ReceiptFile, Registration,
        Row, RowReason, RuleInstanceRecord, RunState, Summary,
    },
};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessOutput,
};
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;
use std::sync::{Barrier, Mutex};
use std::time::Duration;

/// The runs under way now, the most ever under way at once, and how many have started.
#[derive(Debug, Default)]
struct Overlap {
    active: usize,
    peak: usize,
    started: usize,
}

/// An execution whose first `expected` runs each wait at a barrier until all of them are under way, so they overlap
/// whatever the scheduling; a verifier that runs them one by one never passes the barrier, and the test runner's timeout
/// reports it.
#[derive(Debug)]
pub(super) struct OverlappingExecution<E> {
    /// The execution that answers each run.
    pub inner: E,
    /// How many runs must be under way at once.
    pub expected: usize,
    overlap: Mutex<Overlap>,
    together: Barrier,
}

impl<E> OverlappingExecution<E> {
    /// Wrap `inner`, holding its first `expected` runs until all of them are under way.
    pub(super) fn new(inner: E, expected: usize) -> Self {
        Self {
            inner,
            expected,
            overlap: Mutex::new(Overlap::default()),
            together: Barrier::new(expected),
        }
    }
    /// The most runs that were ever under way at once.
    pub(super) fn peak(&self) -> usize {
        self.overlap.lock().expect("overlap").peak
    }
}

impl<E: ExecutionPort> ExecutionPort for OverlappingExecution<E> {
    fn profile(&self) -> ExecutionProfile {
        self.inner.profile()
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let held = {
            let mut overlap = self.overlap.lock().expect("overlap");
            overlap.active += 1;
            overlap.peak = overlap.peak.max(overlap.active);
            overlap.started += 1;
            overlap.started <= self.expected
        };
        if held {
            let _arrival = self.together.wait();
        }
        let answer = self.inner.run(invocation);
        self.overlap.lock().expect("overlap").active -= 1;
        answer
    }
}

/// What one publication carried, and how many publications were under way while it was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Publication {
    /// The run state the summary declared.
    pub running: bool,
    /// The reason of each row, in inventory order.
    pub reasons: Vec<RowReason>,
    /// How many publications were under way at once, this one included.
    pub writers: usize,
}

/// An inventory store that answers through `inner` and records every publication.
#[derive(Debug)]
pub(super) struct ObservedInventory<I> {
    /// The store that answers.
    pub inner: I,
    writers: Mutex<usize>,
    publications: Mutex<Vec<Publication>>,
}

impl<I> ObservedInventory<I> {
    /// Observe the publications `inner` receives.
    pub(super) fn new(inner: I) -> Self {
        Self {
            inner,
            writers: Mutex::new(0),
            publications: Mutex::new(Vec::new()),
        }
    }
    /// Every publication, in the order it was made.
    pub(super) fn publications(&self) -> Vec<Publication> {
        self.publications.lock().expect("publications").clone()
    }
}

impl<I: InventoryPort> InventoryPort for ObservedInventory<I> {
    fn load(&self, path: &Path) -> Result<Registration, Error> {
        self.inner.load(path)
    }
    fn begin(&self, output: &Path) -> Result<(), Error> {
        self.inner.begin(output)
    }
    fn publish(&self, output: &Path, summary: &Summary) -> Result<(), Error> {
        let writers = {
            let mut writers = self.writers.lock().expect("writers");
            *writers += 1;
            *writers
        };
        // Leave room for a second writer to arrive while this one is under way.
        std::thread::sleep(Duration::from_millis(20));
        let answer = self.inner.publish(output, summary);
        self.publications
            .lock()
            .expect("publications")
            .push(Publication {
                running: summary.run_state == RunState::Running,
                reasons: summary.rules.iter().map(|row| row.reason.clone()).collect(),
                writers,
            });
        *self.writers.lock().expect("writers") -= 1;
        answer
    }
    fn prepare(&self, observations: &Path) -> Result<(), Error> {
        self.inner.prepare(observations)
    }
    fn capture(
        &self,
        observations: &Path,
        stdout: &CapturedBytes,
        stderr: &CapturedBytes,
    ) -> Result<ReceiptFile, Error> {
        self.inner.capture(observations, stdout, stderr)
    }
    fn result(&self, observations: &Path, row: &Row) -> Result<(), Error> {
        self.inner.result(observations, row)
    }
    fn decode_invocation(&self, bytes: &CapturedBytes) -> Result<InvocationDocument, Error> {
        self.inner.decode_invocation(bytes)
    }
    fn raw_artifacts(
        &self,
        receipt: &Path,
        records: &[ObservationDocument],
    ) -> Result<RawArtifacts, Error> {
        self.inner.raw_artifacts(receipt, records)
    }
    fn context_files(&self, context: &Path) -> Result<ContextFiles, Error> {
        self.inner.context_files(context)
    }
    fn audit_report(&self, audit: &ReceiptAudit) -> Result<CapturedBytes, Error> {
        self.inner.audit_report(audit)
    }
    fn store(&self, path: &Path, registration: &Registration) -> Result<(), Error> {
        self.inner.store(path, registration)
    }
    fn stage_profile(
        &self,
        source: &Path,
        destination: &Path,
        projection: &ProfileProjection,
    ) -> Result<(), Error> {
        self.inner.stage_profile(source, destination, projection)
    }
    fn case_inventory(&self, stage: &Path) -> Result<CaseDocuments, Error> {
        self.inner.case_inventory(stage)
    }
    fn instance(&self, record: &RuleInstanceRecord) -> Result<CanonicalJson, Error> {
        self.inner.instance(record)
    }
}
