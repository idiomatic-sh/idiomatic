//! A registration lists every default; a missing implementation fails its row, a default the canonical profile defers is
//! DEFERRED and runs nothing, and an empty list or an undeclared deferral is refused.
use super::{interface::VerifyInventoryUseCase as _, use_case::VerifyInventoryUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use crate::outbound::{execution::fixtures::fake::ExecutionFake, inventory::factory::local};
use serde_json::json;
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

/// The registration a row verifies.
#[derive(Clone, Copy, Debug)]
enum Registered {
    /// No implementation, under unavailable authority, for these defaults.
    Missing(&'static [&'static str]),
    /// `A1` registered as deferred under `control_flow`, beside the unregistered defaults listed, under execution
    /// authority whose canonical profile declares the deferral when `declared`.
    Deferred {
        others: &'static [&'static str],
        declared: bool,
    },
    /// `A1` registered as deferred under unavailable authority, which pins no profile to account it.
    DeferredWithoutAuthority,
}

type Input = Registered;
/// The verdict, the launches and the summary's counts.
type Value = (InventoryVerdict, usize, Cow<'static, str>);
/// The refusal and the launches before it.
type Failure = (Cow<'static, str>, usize);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a default without an implementation fails its row and launches nothing"),
        word: Word::Succeeds,
        input: || Registered::Missing(&["missing"]),
        expect: Expect::Value((
            InventoryVerdict::Failed,
            0,
            Cow::Borrowed(r#"{"DEFERRED":0,"FAIL":1,"PASS":0}"#),
        )),
    },
    Case {
        name: CaseName("a default the profile defers is DEFERRED, launches nothing and passes"),
        word: Word::Succeeds,
        input: || Registered::Deferred {
            others: &[],
            declared: true,
        },
        expect: Expect::Value((
            InventoryVerdict::Passed,
            0,
            Cow::Borrowed(r#"{"DEFERRED":1,"FAIL":0,"PASS":0}"#),
        )),
    },
    Case {
        name: CaseName("a deferred default beside a missing one leaves the inventory failed"),
        word: Word::Succeeds,
        input: || Registered::Deferred {
            others: &["B1"],
            declared: true,
        },
        expect: Expect::Value((
            InventoryVerdict::Failed,
            0,
            Cow::Borrowed(r#"{"DEFERRED":1,"FAIL":1,"PASS":0}"#),
        )),
    },
    Case {
        name: CaseName("empty defaults are refused before any launch"),
        word: Word::Refuses,
        input: || Registered::Missing(&[]),
        expect: Expect::Error((
            Cow::Borrowed("inventory must contain unique nonempty rule IDs"),
            0,
        )),
    },
    Case {
        name: CaseName("a deferral the canonical profile does not declare is refused"),
        word: Word::Refuses,
        input: || Registered::Deferred {
            others: &[],
            declared: false,
        },
        expect: Expect::Error((
            Cow::Borrowed("registered deferral differs from the canonical profile's: A1"),
            0,
        )),
    },
    Case {
        name: CaseName("a deferral under unavailable authority is refused"),
        word: Word::Refuses,
        input: || Registered::DeferredWithoutAuthority,
        expect: Expect::Error((
            Cow::Borrowed("receipt-only inventory cannot authorize execution"),
            0,
        )),
    },
];

/// A deferred rule as the registration document writes it.
fn deferred_rule() -> serde_json::Value {
    json!({"definition": null, "modules": {}, "artifacts": {}, "rule_id": "A1", "command": {"kind": "deferred", "capability": "control_flow"}, "reason": null, "cases": [], "instance": null})
}

/// Write `bytes` at `path` and answer their digest.
fn pinned(path: &Path, bytes: &[u8]) -> String {
    use sha2::Digest as _;
    std::fs::write(path, bytes).expect("pinned fixture");
    format!("{:x}", sha2::Sha256::digest(bytes))
}

/// The registration document of `registered`, its pinned files written under `root`.
fn registration(root: &Path, registered: Registered) -> serde_json::Value {
    let unavailable =
        json!({"kind": "unavailable", "reason": "fixture registration has no implementation"});
    match registered {
        Registered::Missing(defaults) => {
            json!({"authority": unavailable, "provider": "fixture", "defaults": defaults, "rules": []})
        }
        Registered::DeferredWithoutAuthority => {
            json!({"authority": unavailable, "provider": "fixture", "defaults": ["A1"], "rules": [deferred_rule()]})
        }
        Registered::Deferred { others, declared } => {
            let reference = root.join("reference");
            let worker = root.join("worker");
            let reference_sha256 = pinned(&reference, b"pinned reference executable");
            let worker_sha256 = pinned(&worker, b"distinct pinned worker executable");
            let defaults = [&["A1"], others].concat();
            let deferral = if declared {
                "\n[deferred]\ncontrol_flow = [\"A1\"]\n"
            } else {
                ""
            };
            let profile = root.join("profile");
            std::fs::create_dir_all(&profile).expect("profile directory");
            let profile_sha256 = pinned(
                &profile.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
                format!("tool = \"fixture\"\nversion = \"1\"\nsha256 = [{reference_sha256:?}]\ndefaults = {defaults:?}\n{deferral}").as_bytes(),
            );
            json!({
                "authority": {"kind": "execution", "profile": profile, "profile_sha256": profile_sha256,
                    "reference": {"path": reference, "sha256": reference_sha256},
                    "worker": {"path": worker, "sha256": worker_sha256}},
                "provider": "fixture", "defaults": defaults, "rules": [deferred_rule()],
            })
        }
    }
}

fn subject(registered: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("inventory fixture");
    let registration_path = root.path().join("registration.json");
    let output = root.path().join("result");
    std::fs::write(
        &registration_path,
        registration(root.path(), registered).to_string(),
    )
    .expect("registration fixture");
    let execution = Arc::new(ExecutionFake::default());
    let operation = VerifyInventoryUseCaseImpl::new(
        Arc::new(crate::outbound::suite::factory::local()),
        Arc::new(crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        )),
        execution.clone(),
        Arc::new(local()),
    );
    let result = operation.verify_inventory(&registration_path, &output);
    let calls = execution.history.lock().expect("history").len();
    let counts = std::fs::read(output.join("summary.json")).map_or_else(
        |_absent| Cow::Borrowed(""),
        |bytes| {
            Cow::Owned(
                serde_json::from_slice::<serde_json::Value>(&bytes).expect("summary JSON")["counts"]
                    .to_string(),
            )
        },
    );
    result
        .map(|verdict| (verdict, calls, counts))
        .map_err(|error| (Cow::Owned(error.to_string()), calls))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
