//! What every `context` adapter must pass.
use super::data::{context, missing, present, sources};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::context::interface::ContextPort;
use crate::outbound::context::types::CorpusFiles;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// The independently specified observations a context adapter answers.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    /// A materialized context validates under the root it was materialized for.
    MaterializedValidates,
    /// A receipt of declared dependencies verifies against its manifest.
    ReceiptVerifies,
    /// The receipt text is stable for one receipt and separates receipts with different observations.
    ReceiptTextSeparatesReceipts,
    /// The seal binds the declared absences.
    FingerprintBindsAbsences,
    /// The seal never depends on the physical boundary.
    FingerprintIgnoresBoundary,
    /// An existing corpus seals under its root.
    ExistingCorpusSeals,
    /// An auxiliary file outside the snapshot is refused.
    UnsafeAuxiliaryPath,
    /// A dependency that escapes the boundary is refused.
    EscapingDependency,
    /// A receipt is assembled from the manifest alone, after its snapshot is gone.
    ReceiptReadsNoFile,
}

/// What a contract row is refused with.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The adapter refused the input as invalid.
    Invalid,
    /// The adapter failed for another reason.
    Other,
}

/// The operation a row exercises.
type Input = Operation;
/// What a row expects when it succeeds.
type Value = ();
/// What a row expects when it is refused.
type Failure = Refusal;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a materialized context validates"),
        word: Word::Succeeds,
        input: || Operation::MaterializedValidates,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a receipt of declared dependencies verifies"),
        word: Word::Succeeds,
        input: || Operation::ReceiptVerifies,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the receipt text is stable and separates different observations"),
        word: Word::Succeeds,
        input: || Operation::ReceiptTextSeparatesReceipts,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the seal binds declared absences"),
        word: Word::Succeeds,
        input: || Operation::FingerprintBindsAbsences,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the seal ignores the physical boundary"),
        word: Word::Succeeds,
        input: || Operation::FingerprintIgnoresBoundary,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an existing corpus seals under its root"),
        word: Word::Succeeds,
        input: || Operation::ExistingCorpusSeals,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a receipt reads no file of its snapshot"),
        word: Word::Succeeds,
        input: || Operation::ReceiptReadsNoFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an auxiliary path outside the snapshot is refused"),
        word: Word::Refuses,
        input: || Operation::UnsafeAuxiliaryPath,
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a dependency escaping the boundary is refused"),
        word: Word::Refuses,
        input: || Operation::EscapingDependency,
        expect: Expect::Error(Refusal::Invalid),
    },
];

/// Run the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
/// Panics if any contract observation differs.
pub fn check(adapter: &impl ContextPort) {
    let subject = |operation| {
        observe(adapter, operation).map_err(|error| match error {
            Error::Invalid(_) => Refusal::Invalid,
            Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Other,
        })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Exercise one operation against independently constructed inputs.
fn observe(adapter: &impl ContextPort, operation: Operation) -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let boundary = scratch.path().join("boundary");
    let materialized =
        |auxiliary: &str| adapter.materialize(&boundary, &context(auxiliary), &sources());
    match operation {
        Operation::MaterializedValidates => {
            let manifest = materialized("pkg/__init__.py")?;
            adapter.validate(&manifest, &manifest.boundary.join("pkg"))
        }
        Operation::ReceiptVerifies => {
            let manifest = materialized("pkg/__init__.py")?;
            let root = manifest.boundary.join("pkg");
            let observed = adapter.receipt(
                &manifest,
                &root,
                vec![present("__init__.py"), missing("../__init__.py")],
            )?;
            adapter.verify_receipt(&manifest, &root, &observed)
        }
        Operation::ReceiptTextSeparatesReceipts => {
            let manifest = materialized("pkg/__init__.py")?;
            let root = manifest.boundary.join("pkg");
            let both = adapter.receipt(
                &manifest,
                &root,
                vec![present("__init__.py"), missing("../__init__.py")],
            )?;
            let present_only = adapter.receipt(&manifest, &root, vec![present("__init__.py")])?;
            let text = adapter.receipt_text(&both)?;
            if text != adapter.receipt_text(&both)? {
                Err(Error::Invalid(
                    "the receipt text of one receipt changes".into(),
                ))
            } else if text == adapter.receipt_text(&present_only)? {
                Err(Error::Invalid(
                    "the receipt text ignores an observation".into(),
                ))
            } else {
                Ok(())
            }
        }
        Operation::FingerprintBindsAbsences => {
            let manifest = materialized("pkg/__init__.py")?;
            let mut absent = manifest.clone();
            absent.missing.push("pkg/marker.py".into());
            check_differs(adapter, &manifest, &absent)
        }
        Operation::FingerprintIgnoresBoundary => {
            let first = materialized("pkg/__init__.py")?;
            let other = scratch.path().join("elsewhere");
            let second = adapter.materialize(&other, &context("pkg/__init__.py"), &sources())?;
            if adapter.fingerprint(&first) == adapter.fingerprint(&second) {
                Ok(())
            } else {
                Err(Error::Invalid("the seal depends on the boundary".into()))
            }
        }
        Operation::ExistingCorpusSeals => {
            std::fs::write(scratch.path().join("source.py"), "import pkg\n")?;
            let tracked = vec![std::path::PathBuf::from("source.py")];
            let manifest = adapter.seal_existing(
                scratch.path(),
                &[ryo::linter::RelativePath("source.py".into())],
                CorpusFiles(&tracked),
            )?;
            adapter.validate(&manifest, scratch.path())
        }
        Operation::UnsafeAuxiliaryPath => materialized("../escape").map(|_| ()),
        Operation::EscapingDependency => {
            let manifest = materialized("pkg/__init__.py")?;
            let root = manifest.boundary.join("pkg");
            adapter
                .receipt(&manifest, &root, vec![missing("../../escape")])
                .map(|_| ())
        }
        Operation::ReceiptReadsNoFile => {
            let manifest = materialized("pkg/__init__.py")?;
            let root = manifest.boundary.join("pkg");
            match std::fs::remove_dir_all(&manifest.boundary) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            adapter
                .receipt(
                    &manifest,
                    &root,
                    vec![
                        present("__init__.py"),
                        missing("../__init__.py"),
                        missing("absent.py"),
                    ],
                )
                .map(|_| ())
        }
    }
}

/// Two manifests with different declared content must not share a seal.
fn check_differs(
    adapter: &impl ContextPort,
    first: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
    second: &ryo_benchmark_contracts::domain::context::types::ContextManifest,
) -> Result<(), Error> {
    if adapter.fingerprint(first) == adapter.fingerprint(second) {
        Err(Error::Invalid("the seal ignores a declared absence".into()))
    } else {
        Ok(())
    }
}
