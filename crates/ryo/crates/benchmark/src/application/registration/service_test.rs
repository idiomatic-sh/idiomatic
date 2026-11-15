//! The registrar stages each default with its own settings, supporting rules, modules and evidence, and registers a
//! rule it cannot stage with its reason; the deleted Python registration tests' assertions, at the registrar's seam.
use super::service::register;
use crate::cases::{Case, CaseName, Expect, Word, run, slashed};
use crate::outbound::executables::types::Sealed;
use crate::outbound::inventory::interface::InventoryPort as _;
use crate::outbound::inventory::types::{RuleCommand, RuleReason};
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use std::borrow::Cow;
use std::path::Path;

/// What a registration scenario authors.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A default without a definition.
    MissingImplementation,
    /// A default without a definition that the profile defers.
    DeferredDefault,
    /// A default with a definition that the profile still defers: the definition wins.
    AuthoredDeferredDefault,
    /// `profiles/<id>/benchmark.toml` names another parser configuration than the bench profile.
    DedicatedProfile,
    /// A case declares an observation produced by another rule.
    SupportingProducer,
    /// The definition imports a module no profile declares.
    UndeclaredModule,
    /// A singleton profile the projection leaves unchanged.
    FixedPoint,
    /// A listed-disk profile whose case declares no context.
    ListedDiskWithoutContext,
    /// The checked-in F821 discrepancy fixture, its evidence beside its profile.
    DiscrepancyEvidence,
    /// That fixture with its first evidence path escaping the profile.
    EscapingEvidence,
    /// That fixture with its first evidence path missing.
    MissingEvidence,
    /// That fixture with its first evidence digest wrong.
    StaleEvidence,
}

/// The bench profile every authored scenario starts from.
const PROFILE: &str = "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ndefaults = [\"A1\"]\nsupporting-rules = [\"B1\"]\nparser-configuration = \"target=1\"\n\n[rule-applicability]\nA1 = \"always\"\nB1 = \"always\"\n";

/// One case of `A1` with no finding.
const CLEAN_CASES: &str =
    "[[cases]]\nname = \"clean\"\nfiles = { \"a.txt\" = \"x\\n\" }\nexpected = []\n";

/// Write one file, making its directory.
fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("file");
}

/// Copy a directory tree.
fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).expect("fixture directory") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::create_dir_all(to).expect("directory");
            let _bytes = std::fs::copy(entry.path(), target).expect("copy");
        }
    }
}

/// What a missing evidence file is refused with, when the reason is the registrar's text around the platform's own words for a missing file.
const MISSING_EVIDENCE: &str =
    "rule registration: discrepancy evidence refused: <the platform's words for a missing file>";

/// The refusal of the registrar with the platform's words for a missing file spelled as `MISSING_EVIDENCE` spells them.
fn spelled(reason: String, absent: &Path) -> Cow<'static, str> {
    let missing = std::fs::read(absent)
        .expect_err("the path is absent")
        .to_string();
    if reason == format!("rule registration: discrepancy evidence refused: {missing}") {
        Cow::Borrowed(MISSING_EVIDENCE)
    } else {
        Cow::Owned(reason)
    }
}

/// Author the scenario's bench under `bench`.
fn author(bench: &Path, scenario: Scenario) {
    let f821 = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/reference-discrepancy-python-f821");
    let cases = bench.join("rules/F821/cases.toml");
    let edit_evidence = |from: &str, to: &str| {
        let text = std::fs::read_to_string(&cases).expect("cases");
        assert!(text.contains(from), "the fixture names {from}");
        std::fs::write(&cases, text.replacen(from, to, 1)).expect("edited cases");
    };
    match scenario {
        Scenario::DiscrepancyEvidence
        | Scenario::EscapingEvidence
        | Scenario::MissingEvidence
        | Scenario::StaleEvidence => copy_tree(&f821, bench),
        Scenario::MissingImplementation
        | Scenario::DeferredDefault
        | Scenario::AuthoredDeferredDefault
        | Scenario::DedicatedProfile
        | Scenario::SupportingProducer
        | Scenario::UndeclaredModule
        | Scenario::FixedPoint
        | Scenario::ListedDiskWithoutContext => {
            write(&bench.join("benchmark.toml"), PROFILE);
            write(&bench.join("rules/A1/cases.toml"), CLEAN_CASES);
            write(&bench.join("rules/A1/query.ryoql"), "emit nothing;\n");
        }
    }
    match scenario {
        Scenario::MissingImplementation => {
            std::fs::remove_file(bench.join("rules/A1/query.ryoql")).expect("no definition");
        }
        Scenario::DeferredDefault => {
            std::fs::remove_file(bench.join("rules/A1/query.ryoql")).expect("no definition");
            write(
                &bench.join("benchmark.toml"),
                &format!("{PROFILE}\n[deferred]\ncontrol_flow = [\"A1\"]\n"),
            );
        }
        Scenario::AuthoredDeferredDefault => write(
            &bench.join("benchmark.toml"),
            &format!("{PROFILE}\n[deferred]\ncontrol_flow = [\"A1\"]\n"),
        ),
        Scenario::DedicatedProfile => write(
            &bench.join("profiles/A1/benchmark.toml"),
            &PROFILE.replace("target=1", "target=2"),
        ),
        Scenario::SupportingProducer => {
            write(&bench.join("rules/B1/query.ryoql"), "emit produced;\n");
            write(
                &bench.join("rules/A1/cases.toml"),
                &format!(
                    "{CLEAN_CASES}rule-observations = {{ \"A1\" = [{{ \"name\" = \"seen\", \"columns\" = [{{ \"name\" = \"file\", \"kind\" = \"file\" }}], \"producers\" = [{{ \"kind\" = \"output\", \"rule\" = \"B1\", \"output\" = \"seen\" }}] }}] }}\n"
                ),
            );
        }
        Scenario::UndeclaredModule => write(
            &bench.join("rules/A1/query.ryoql"),
            "import helper from \"lint/helper\";\nemit nothing;\n",
        ),
        Scenario::FixedPoint => write(
            &bench.join("profiles/A1/benchmark.toml"),
            "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ndefaults = [\"A1\"]\nparser-configuration = \"target=1\"\n\n[rule-applicability]\nA1 = \"always\"\n",
        ),
        Scenario::ListedDiskWithoutContext => write(
            &bench.join("benchmark.toml"),
            &format!("project-context-mode = \"listed-disk\"\n{PROFILE}"),
        ),
        Scenario::EscapingEvidence => edit_evidence("\"evidence/", "\"../escape/"),
        Scenario::MissingEvidence => edit_evidence("\"evidence/", "\"evidence/missing-"),
        Scenario::StaleEvidence => {
            let text = std::fs::read_to_string(&cases).expect("cases");
            let start =
                text.find("\"sha256\" = \"").expect("an evidence digest") + "\"sha256\" = \"".len();
            let digest = text
                .get(start..start + 64)
                .expect("a whole digest")
                .to_owned();
            std::fs::write(&cases, text.replace(&digest, &"0".repeat(64))).expect("edited cases");
        }
        Scenario::DiscrepancyEvidence => {}
    }
}

/// What a registered rule shows: its reason, or its staged profile and files.
type Input = Scenario;
/// What a registered rule shows, or why it was not staged.
type Value = Cow<'static, str>;
/// Why a rule was registered without a command.
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a default without a definition is registered as a missing implementation"),
        word: Word::Succeeds,
        input: || Scenario::MissingImplementation,
        expect: Expect::Value(Cow::Borrowed("missing implementation")),
    },
    Case {
        name: CaseName("a deferred default without a definition is registered as deferred"),
        word: Word::Succeeds,
        input: || Scenario::DeferredDefault,
        expect: Expect::Value(Cow::Borrowed("deferred: control_flow")),
    },
    Case {
        name: CaseName("a deferred default with a definition is registered as executable"),
        word: Word::Succeeds,
        input: || Scenario::AuthoredDeferredDefault,
        expect: Expect::Value(Cow::Borrowed(
            "defaults = [\"A1\"]\nparser-configuration = \"target=1\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ntool = \"lint\"\nversion = \"lint 1.0.0\"\n\n[rule-applicability]\nA1 = \"always\"\n; cases: 1; files: benchmark.toml, rules/A1/cases.toml, rules/A1/query.ryoql",
        )),
    },
    Case {
        name: CaseName("profiles/<id>/benchmark.toml wins over the bench profile"),
        word: Word::Succeeds,
        input: || Scenario::DedicatedProfile,
        expect: Expect::Value(Cow::Borrowed(
            "defaults = [\"A1\"]\nparser-configuration = \"target=2\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ntool = \"lint\"\nversion = \"lint 1.0.0\"\n\n[rule-applicability]\nA1 = \"always\"\n; cases: 1; files: benchmark.toml, rules/A1/cases.toml, rules/A1/query.ryoql",
        )),
    },
    Case {
        name: CaseName(
            "a rule a case's observation names as its producer is staged as a supporting rule",
        ),
        word: Word::Succeeds,
        input: || Scenario::SupportingProducer,
        expect: Expect::Value(Cow::Borrowed(
            "staged as its source; cases: 1; files: benchmark.toml, rules/A1/cases.toml, rules/A1/query.ryoql, rules/B1/query.ryoql",
        )),
    },
    Case {
        name: CaseName(
            "a singleton profile the projection leaves unchanged is staged byte for byte",
        ),
        word: Word::Succeeds,
        input: || Scenario::FixedPoint,
        expect: Expect::Value(Cow::Borrowed(
            "staged as its source; cases: 1; files: benchmark.toml, rules/A1/cases.toml, rules/A1/query.ryoql",
        )),
    },
    Case {
        name: CaseName("discrepancy evidence is staged from the settings profile's directory"),
        word: Word::Succeeds,
        input: || Scenario::DiscrepancyEvidence,
        expect: Expect::Value(Cow::Borrowed(
            "staged as its source; cases: 4; files: benchmark.toml, evidence/cpython310-exact.json, evidence/cpython313-exact.json, evidence/cpython313.json, evidence/language-contract.json, rules/F821/cases.toml, rules/F821/query.ryoql",
        )),
    },
    Case {
        name: CaseName("an import no profile declares is refused"),
        word: Word::Refuses,
        input: || Scenario::UndeclaredModule,
        expect: Expect::Error(Cow::Borrowed(
            "rule registration: undeclared canonical helper: lint/helper",
        )),
    },
    Case {
        name: CaseName("a listed-disk case without its explicit context is refused"),
        word: Word::Refuses,
        input: || Scenario::ListedDiskWithoutContext,
        expect: Expect::Error(Cow::Borrowed(
            "rule registration: listed-disk profile requires an explicit context for every case",
        )),
    },
    Case {
        name: CaseName("evidence escaping the profile is refused"),
        word: Word::Refuses,
        input: || Scenario::EscapingEvidence,
        expect: Expect::Error(Cow::Borrowed(
            "rule registration: discrepancy evidence refused: unsafe artifact-relative path",
        )),
    },
    Case {
        name: CaseName("missing evidence is refused"),
        word: Word::Refuses,
        input: || Scenario::MissingEvidence,
        expect: Expect::Error(Cow::Borrowed(MISSING_EVIDENCE)),
    },
    Case {
        name: CaseName("evidence with a wrong digest is refused"),
        word: Word::Refuses,
        input: || Scenario::StaleEvidence,
        expect: Expect::Error(Cow::Borrowed(
            "rule registration: discrepancy evidence refused: raw artifact byte identity differs",
        )),
    },
];

/// Register the scenario's bench and describe its one rule: a staged rule's settings, supporting rules and staged
/// files (relative to its stage), or the reason of a rule registered without a command.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("registration fixture");
    let bench = root.path().join("bench");
    author(&bench, scenario);
    let output = root.path().join("out");
    let sealed = |name: &str| Sealed {
        path: root.path().join(name),
        sha256: Sha256Text("1".repeat(64)),
    };
    let suite = crate::outbound::suite::factory::local();
    let inventory = crate::outbound::inventory::factory::local();
    let path = register(
        &suite,
        &inventory,
        &bench,
        &sealed("reference"),
        &sealed("worker"),
        &output,
    )
    .expect("a registration is written");
    let registration = inventory.load(&path).expect("the registration reads back");
    let [rule] = registration.rules.as_slice() else {
        panic!("one rule is registered");
    };
    match (&rule.command, &rule.reason) {
        (
            RuleCommand::Registered(crate::outbound::inventory::types::Command::CaseVerification {
                context,
            }),
            RuleReason::Unstated,
        ) => {
            let profile =
                std::fs::read_to_string(context.join("benchmark.toml")).expect("staged profile");
            let mut files = rule
                .artifacts
                .keys()
                .map(|path| slashed(path.strip_prefix(context).expect("inside the stage")).0)
                .filter(|file| !file.starts_with("fragments/"))
                .collect::<Vec<_>>();
            files.sort();
            let source = std::fs::read_to_string(bench.join("profiles/A1/benchmark.toml"))
                .or_else(|_absent| std::fs::read_to_string(bench.join("benchmark.toml")))
                .expect("the settings profile");
            let staged = if profile == source {
                "staged as its source".to_owned()
            } else {
                profile
            };
            Ok(Cow::Owned(format!(
                "{staged}; cases: {}; files: {}",
                rule.cases.len(),
                files.join(", ")
            )))
        }
        (RuleCommand::Deferred(capability), RuleReason::Unstated) => {
            Ok(Cow::Owned(format!("deferred: {capability}")))
        }
        (_, RuleReason::Stated(reason)) if reason.0 == "missing implementation" => {
            Ok(Cow::Owned(reason.0.clone()))
        }
        (_, RuleReason::Stated(reason)) => Err(spelled(
            reason
                .0
                .replace(&root.path().display().to_string(), "<root>"),
            &root.path().join("absent"),
        )),
        (RuleCommand::Unregistered, RuleReason::Unstated) => {
            Err(Cow::Borrowed("unregistered without a reason"))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
