//! Reading the receipts a process capture wrote, and replaying them through the offline validator.
use crate::application::case_observation::service::write as record;
use crate::outbound::inventory::fixtures::registrations::document_of;
use crate::outbound::inventory::fixtures::registrations::process_failure;
use crate::outbound::inventory::fixtures::registrations::registration_of;
use crate::outbound::inventory::interface::InventoryPort as _;
use crate::outbound::inventory::types::RawArtifacts;
use crate::outbound::inventory::types::Registration;
use crate::outbound::receipts::interface::ReceiptsPort as _;
use crate::outbound::receipts::types::ReceiptPath;
use crate::outbound::suite::interface::SuitePort as _;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess;
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use serde_json::Value;
use serde_json::json;
use sha2::Digest as _;
use std::collections::BTreeMap;
use std::path::PathBuf;
use super::process_capture::Fixture;
use super::process_capture::pin;
use super::process_identity::Verdict;
use super::process_identity::verdict;

/// The rows of one receipt file, with the raw bytes and invocation records they point at.
pub(crate) struct Loaded {
    pub records: Vec<ObservationDocument>,
    pub artifacts: BTreeMap<PathBuf, CapturedBytes>,
    pub invocations: BTreeMap<PathBuf, InvocationDocument>,
}
impl Loaded {
    /// The raw artifacts as the offline audit takes them.
    pub fn raw(&self) -> RawArtifacts {
        RawArtifacts {
            bytes: self.artifacts.clone(),
            invocations: self.invocations.clone(),
        }
    }
    /// The raw bytes the first row's observed reference process wrote to stdout and stderr.
    pub fn reference_streams(&self) -> (&CapturedBytes, &CapturedBytes) {
        let ReferenceProcess::Observed(observed) = &self.records[0].process_observation else {
            panic!("the reference process is observed");
        };
        let ProcessArtifactsRecord::Attached(pointers) = &observed.artifacts else {
            panic!("the observed process points at artifacts");
        };
        (
            &self.artifacts[std::path::Path::new(&pointers.stdout.path.0)],
            &self.artifacts[std::path::Path::new(&pointers.stderr.path.0)],
        )
    }
}

/// The `file` member of the JSON document the bytes spell.
pub(crate) fn file_member(bytes: &CapturedBytes) -> String {
    serde_json::from_slice::<Value>(&bytes.0).unwrap()["file"]
        .as_str()
        .expect("the document names a file")
        .to_owned()
}

impl Fixture {
    pub fn load(&self) -> Loaded {
        self.read().expect("the fixture receipts decode")
    }
    /// The rows of another receipt file, with the raw bytes and invocation records they point at.
    pub fn load_file(&self, receipt: &std::path::Path) -> Loaded {
        self.read_at(receipt).expect("the receipt decodes")
    }
    fn read(&self) -> Option<Loaded> {
        self.read_at(&self.receipt)
    }
    fn read_at(&self, receipt: &std::path::Path) -> Option<Loaded> {
        let records = self.receipts.case_observations(ReceiptPath(receipt)).ok()?;
        let raw = crate::outbound::inventory::factory::local()
            .raw_artifacts(receipt, &records)
            .ok()?;
        Some(Loaded {
            records,
            artifacts: raw.bytes,
            invocations: raw.invocations,
        })
    }
    pub fn registration(&self) -> Registration {
        registration_of(&self.case)
    }
    /// The process failure the registered case declares.
    pub fn declared_failure(&self) -> ProcessFailure {
        process_failure(&self.case["reference_discrepancy"]["process"])
    }
    /// What the offline validator answers over `loaded`, against the registered case.
    pub fn replay(&self, loaded: &Loaded) -> Verdict {
        verdict(self.validate(&self.document, loaded))
    }
    /// The refusal of the offline validator over `loaded`.
    pub fn refusal(&self, loaded: &Loaded) -> String {
        self.validate(&self.document, loaded).unwrap_err().to_string()
    }
    fn validate(
        &self,
        document: &CaseDocument,
        loaded: &Loaded,
    ) -> Result<
        crate::domain::inventory_receipts::types::ReceiptCounts,
        crate::domain::inventory_receipts::errors::InventoryReceiptsError,
    > {
        crate::domain::inventory_receipts::service::validate_with_artifacts(
            std::slice::from_ref(document),
            &loaded.records,
            &loaded.artifacts,
            &loaded.invocations,
        )
    }
    /// What the validator answers after row `row` reports one finding of `rule` over `start..end` of `file`.
    pub fn replay_finding(&self, row: usize, rule: &str, file: &str, start: u64, end: u64) -> Verdict {
        let finding = json!([{"rule": rule, "file": file, "start": start, "end": end}]);
        self.replay_tampered(row, "/diagnostics", &finding.to_string())
    }
    /// What the validator answers when the registered case spells another expectation contract.
    pub fn replay_contract(&self, contract: &str) -> Verdict {
        let mut case = self.case.clone();
        case["expectation_contract"] = json!(contract);
        verdict(self.validate(&document_of(&case), &self.load()))
    }
    /// What the validator answers after `value` (JSON text) replaces `pointer` on one row, or a refusal when the schema refuses the row.
    pub fn replay_tampered(&self, row: usize, pointer: &str, value: &str) -> Verdict {
        let original = std::fs::read_to_string(&self.receipt).unwrap();
        let mut lines: Vec<Value> = original
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        *lines[row].pointer_mut(pointer).unwrap() = serde_json::from_str(value).unwrap();
        let edited = lines
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&self.receipt, edited + "\n").unwrap();
        let loaded = self.read();
        std::fs::write(&self.receipt, original).unwrap();
        loaded.map_or(Verdict::Refused, |loaded| self.replay(&loaded))
    }
    /// The rows with the first row's stdout bytes replaced by forged ones.
    pub fn forged_stdout(&self) -> Loaded {
        let mut loaded = self.load();
        let path = {
            let ReferenceProcess::Observed(observed) = &loaded.records[0].process_observation else {
                panic!("the reference process is observed");
            };
            let ProcessArtifactsRecord::Attached(artifacts) = &observed.artifacts else {
                panic!("the observed process points at artifacts");
            };
            PathBuf::from(&artifacts.stdout.path.0)
        };
        loaded.artifacts.insert(path, CapturedBytes(b"forged".to_vec()));
        loaded
    }
    /// The rows whose first invocation record spells `flag` as its only argument, under a digest that follows the edit.
    pub fn rearmed_invocation(&self, flag: &str) -> Loaded {
        let mut loaded = self.load();
        let ReferenceProcess::Observed(observed) = &mut loaded.records[0].process_observation else {
            panic!("the reference process is observed");
        };
        let ProcessArtifactsRecord::Attached(artifacts) = &mut observed.artifacts else {
            panic!("the observed process points at artifacts");
        };
        let pointer = &mut artifacts.invocation;
        let path = PathBuf::from(&pointer.path.0);
        let mut invocation: Value = serde_json::from_slice(&loaded.artifacts[&path].0).unwrap();
        invocation["arguments"] = json!([flag]);
        let bytes = serde_json::to_vec(&invocation).unwrap();
        pointer.sha256 = Sha256Text(pin(&bytes));
        let captured = CapturedBytes(bytes);
        loaded.invocations.insert(
            path.clone(),
            crate::outbound::inventory::factory::local()
                .decode_invocation(&captured)
                .unwrap(),
        );
        loaded.artifacts.insert(path, captured);
        loaded
    }
}
