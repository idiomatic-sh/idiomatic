//! Only a production source statement changes; the sparse public API case stays identical.
use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

#[derive(Clone, Copy)]
pub(super) enum Mutation {
    DenseReads,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Rejected {
    DormantReads,
}
pub(super) struct ActualSource {
    overlay: tempfile::TempDir,
    setup: String,
}
const SPARSE: &str = "same_131_rule_sparse_intent_reads_only_its_required_metadata";
const ALLOCATION: &str =
    "one_selector_has_identical_actual_work_and_allocations_with_128_dormant_roots";
impl ActualSource {
    pub(super) fn new() -> Self {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"));
        let overlay = tempfile::tempdir().unwrap();
        super::super::fixtures::copy_sources(&source.join("src"), &overlay.path().join("src")).unwrap();
        super::super::fixtures::copy_sources(
            &source.join("tests/activation"),
            &overlay.path().join("tests/activation"),
        )
        .unwrap();
        fs::copy(source.join("README.md"), overlay.path().join("README.md")).unwrap();
        fs::copy(
            source.join("../../Cargo.lock"),
            overlay.path().join("Cargo.lock"),
        )
        .unwrap();
        let mut manifest = super::super::fixtures::manifest(source);
        let mut target = toml::Table::new();
        target.insert("name".into(), "activation".into());
        target.insert("path".into(), "tests/activation/test.rs".into());
        target.insert("harness".into(), false.into());
        manifest.insert("test".into(), toml::Value::Array(vec![target.into()]));
        fs::write(
            overlay.path().join("Cargo.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        Self {
            setup: fs::read_to_string(source.join(super::SETUP)).unwrap(),
            overlay,
        }
    }
    fn run(&self, case: &str) -> Output {
        let output = Command::new(env!("CARGO"))
            .args([
                "test",
                "--offline",
                "--test",
                "activation",
                // The real target gets `fixtures` from the crate's self dev-dependency, which the
                // overlay manifest leaves out; the harness calls `open_file`, sealed to that feature.
                "--features",
                "fixtures",
                "--jobs",
                "2",
                "--message-format=json",
            ])
            .arg("--manifest-path")
            .arg(self.overlay.path().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(self.overlay.path().join("target"))
            .arg("--")
            .arg(case)
            .output()
            .unwrap();
        assert!(
            output.status.code().is_some(),
            "the activation process must end by itself, not by a signal"
        );
        let mut built = false;
        for message in Message::parse_stream(output.stdout.as_slice()) {
            match message.unwrap() {
                Message::CompilerMessage(message) => assert_ne!(
                    message.message.level,
                    DiagnosticLevel::Error,
                    "a compiler failure cannot certify a runtime mutant: {message:#?}"
                ),
                Message::CompilerArtifact(artifact) if artifact.target.name == "activation" => {
                    built = true
                }
                _ => {}
            }
        }
        assert!(
            built,
            "the actual activation target must compile before a behavioral rejection"
        );
        output
    }
    pub(super) fn baseline(&self) {
        for case in [SPARSE, ALLOCATION] {
            let output = self.run(case);
            assert!(
                output.status.success(),
                "unmodified actual source must pass {case}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("activation: 1 actual cases passed")
            );
        }
    }
    pub(super) fn reject(&self, mutation: Mutation) -> Result<(), Rejected> {
        // Every mutant starts from exactly the same production source. Neither
        // mutation changes the public131-rule request or its sparse intent[0].
        fs::write(self.overlay.path().join(super::SETUP), &self.setup).unwrap();
        let (case, message, rejected) = match mutation {
            Mutation::DenseReads => {
                let anchor = super::ANCHOR;
                assert_eq!(self.setup.matches(anchor).count(), 1);
                let mutant = self.setup.replacen(anchor, &format!("for index in 0..131 {{ let _ = self.relations_for(RuleIndex(index))?; }}\n{anchor}"), 1);
                fs::write(self.overlay.path().join(super::SETUP), mutant).unwrap();
                (
                    SPARSE,
                    "sparse intent must not read dormant rule metadata",
                    Rejected::DormantReads,
                )
            }
        };
        let output = self.run(case);
        assert!(
            !output.status.success(),
            "the same sparse input must reject this source mutant"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(message) && stderr.contains("tests/activation/test.rs"),
            "the expected actual sparse assertion must reject the mutant: {stderr}"
        );
        Err(rejected)
    }
}
