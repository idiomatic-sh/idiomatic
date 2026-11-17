//! The disk store's lifecycle and the shared port contract: what is kept reads back whole through a new adapter,
//! another build reads it as empty, a failed write keeps the previous store, damaged bytes are a cold miss, and the
//! store has the mode any written file has.
use super::constants::{DIAGNOSTICS, FILE};
use super::errors::CheckStorePostcardError;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::fixtures::contract::check;
use crate::outbound::check_store::fixtures::fake::checked;
use crate::outbound::check_store::interface::CheckStorePort as _;
use crate::outbound::check_store::{
    factory,
    types::{Build, Store, Stored},
};

#[derive(Clone, Copy, Debug)]
enum Question {
    Contract,
    Reopened,
    /// The current artifact replaces stale failed output after a successful check.
    Diagnostics,
    OtherBuild,
    FailedWrite,
    Corrupt,
    Blocked,
    /// The mode of the store, beside a file `std::fs::write` makes there.
    Mode,
    /// The store file kept for one build, read as another's.
    ReadForAnother,
}
type Input = Question;
type Value = bool;
/// What a row fails with: the port's failure to keep the store, or the reader's refusal of the kept bytes.
#[derive(Debug)]
enum Refusal {
    /// The store could not be kept.
    Keep(CheckStoreError),
    /// The kept bytes are no store of the reading build.
    Read(CheckStorePostcardError),
}

impl From<CheckStoreError> for Refusal {
    fn from(error: CheckStoreError) -> Self {
        Self::Keep(error)
    }
}

impl From<CheckStorePostcardError> for Refusal {
    fn from(error: CheckStorePostcardError) -> Self {
        Self::Read(error)
    }
}

type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("current_failure_artifact_replaces_stale_output"),
        word: Word::Succeeds,
        input: || Question::Diagnostics,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_adapter_passes_the_port_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_checked_store_reads_back_whole_through_a_new_adapter"),
        word: Word::Succeeds,
        input: || Question::Reopened,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("another_build_reads_the_store_as_empty"),
        word: Word::Succeeds,
        input: || Question::OtherBuild,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_failed_write_preserves_the_previous_store"),
        word: Word::Succeeds,
        input: || Question::FailedWrite,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_damaged_store_is_a_cold_miss"),
        word: Word::Succeeds,
        input: || Question::Corrupt,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_store_has_the_mode_any_written_file_has"),
        word: Word::Succeeds,
        input: || Question::Mode,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_blocked_write_preserves_external_bytes"),
        word: Word::Fails,
        input: || Question::Blocked,
        expect: Expect::Refusal(|error| {
            if matches!(error, Refusal::Keep(CheckStoreError::Unwritable { .. })) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_kept_file_read_for_another_build_is_that_builds"),
        word: Word::Refuses,
        input: || Question::ReadForAnother,
        expect: Expect::Refusal(|error| {
            if matches!(error, Refusal::Read(CheckStorePostcardError::OtherBuild)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// The permission bits of `path`; on a system with no modes, whether it is read-only.
fn mode(path: &std::path::Path) -> u32 {
    let permissions = std::fs::metadata(path).expect("its metadata").permissions();
    #[cfg(unix)]
    let bits = std::os::unix::fs::PermissionsExt::mode(&permissions) & 0o777;
    #[cfg(not(unix))]
    let bits = u32::from(permissions.readonly());
    bits
}

/// A newly persisted check atomically replaces the prior diagnostic artifact.
fn diagnostics(
    adapter: &impl crate::outbound::check_store::interface::CheckStorePort,
    cache: &std::path::Path,
) -> Result<Value, Failure> {
    let mut stored = checked();
    let crate::outbound::check_store::types::KeptReport::Kept(report) = &mut stored.report else {
        panic!("checked report");
    };
    let report = std::sync::Arc::make_mut(report);
    let scan = report.scanned.first_mut().expect("tool scan");
    scan.run.status = asymptote_values::domain::judgement::types::Status::Failed(
        asymptote_values::domain::judgement::types::Failure::Exited {
            code: asymptote_values::domain::judgement::types::Code(7),
            stderr: asymptote_values::domain::judgement::types::Tail::default(),
        },
    );
    scan.failure_output = asymptote_values::domain::reports::types::FailureOutput::Captured {
        stdout: asymptote_values::domain::reports::types::DiagnosticText(
            "stdout-only failure".into(),
        ),
        stderr: asymptote_values::domain::reports::types::DiagnosticText::default(),
        inputs: scan.stamp.clone(),
    };
    adapter.keep(&stored)?;
    let artifact: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cache.join(DIAGNOSTICS)).expect("artifact"))
            .expect("JSON artifact");
    let store = std::fs::read(cache.join(FILE)).expect("store bytes");
    assert_eq!(
        artifact["store_digest"],
        blake3::hash(&store).to_hex().to_string(),
        "artifact names exact store bytes"
    );
    assert_eq!(
        artifact["failures"][0]["output"]["Captured"]["stdout"], "stdout-only failure",
        "full stdout retained"
    );
    adapter.keep(&checked())?;
    let fresh: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cache.join(DIAGNOSTICS)).expect("replacement"))
            .expect("JSON artifact");
    Ok(fresh["failures"].as_array().is_some_and(Vec::is_empty))
}

fn subject(question: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("temporary product");
    let cache = directory.path().join("cache");
    let open = |build: &str| {
        factory::postcard(Store {
            directory: cache.clone(),
            build: Build(build.into()),
        })
    };
    let adapter = open("build-1");
    match question {
        Question::Diagnostics => diagnostics(&adapter, &cache),
        Question::Contract => {
            check(&adapter);
            Ok(open("build-1").load() != Stored::default())
        }
        Question::Blocked => {
            std::fs::write(&cache, "unchanged").expect("obstruction");
            let result = adapter.keep(&Stored::default());
            assert_eq!(
                std::fs::read_to_string(&cache).expect("preserved bytes"),
                "unchanged",
                "a blocked write leaves the bytes in its way alone"
            );
            result.map(|()| false).map_err(Refusal::Keep)
        }
        Question::Reopened => {
            let stored = checked();
            adapter.keep(&stored)?;
            Ok(open("build-1").load() == stored)
        }
        Question::OtherBuild => {
            adapter.keep(&checked())?;
            Ok(open("build-2").load() == Stored::default())
        }
        Question::FailedWrite => {
            let stored = checked();
            adapter.keep(&stored)?;
            let before = std::fs::read(cache.join(FILE)).expect("the store");
            // What cannot be written: the directory on unix, where a file is replaced through it; the store itself on
            // Windows, where a read-only file cannot be replaced and a directory's read-only flag holds nothing back.
            let locked = if cfg!(windows) {
                cache.join(FILE)
            } else {
                cache.clone()
            };
            let mut permissions = std::fs::metadata(&locked)
                .expect("the store's place")
                .permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&locked, permissions.clone()).expect("a read-only place");
            let refused = adapter.keep(&Stored::default());
            #[expect(
                clippy::permissions_set_readonly_false,
                reason = "the temporary directory is made writable again so it can be removed"
            )]
            permissions.set_readonly(false);
            std::fs::set_permissions(&locked, permissions).expect("a writable place");
            assert!(
                matches!(refused, Err(CheckStoreError::Unwritable { .. })),
                "a store that cannot be written fails the write"
            );
            Ok(open("build-1").load() == stored
                && std::fs::read(cache.join(FILE)).expect("the preserved store") == before)
        }
        Question::Mode => {
            adapter.keep(&checked())?;
            let written = cache.join("written");
            std::fs::write(&written, "").expect("a file written as any is");
            Ok(mode(&cache.join(FILE)) == mode(&written))
        }
        Question::ReadForAnother => {
            adapter.keep(&checked())?;
            let bytes = std::fs::read(cache.join(FILE)).expect("the store's bytes");
            Ok(super::schemas::read(&bytes, &Build("build-2".into()))? == checked())
        }
        Question::Corrupt => {
            adapter.keep(&checked())?;
            std::fs::write(cache.join(FILE), "corrupt").expect("damage the store");
            Ok(open("build-1").load() == Stored::default())
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
