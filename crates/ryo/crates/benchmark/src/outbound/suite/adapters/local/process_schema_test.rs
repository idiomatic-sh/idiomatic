//! Exercise the actual TOML case boundary before any authored process certificates are admitted.
use super::schemas::case_reference_wires;
use crate::outbound::suite::types::process::{
    OutputPartWire, PathEncodingWire, ProcessFailureWire,
};

fn source() -> String {
    r#"
[[cases]]
name = "encoded-path"
comparison = "full"
files = { "case.py" = "x = 1\n" }
expected = []
[cases.reference-discrepancy]
reason = "independent semantic authority"
[cases.reference-discrepancy.identity]
tool = "toy"
version = "1"
observed-executable-sha256 = "PIN"
profile-sha256 = "PIN"
settings-sha256 = "PIN"
source-sha256 = "PIN"
reference-output-sha256 = "PIN"
intended-output-sha256 = "PIN"
[[cases.reference-discrepancy.evidence]]
path = "semantics.txt"
sha256 = "PIN"
kind = "language-spec"
language-version = "1"
target-configuration = "target-v1"
version-rationale = "same supported semantics"
[cases.reference-discrepancy.process]
kind = "process-failure"
exit = 2
stdout = [{kind="literal", value='{"file":"'}, {kind="input-path", file="case.py", encoding="json-string-content"}, {kind="literal", value='"}'}]
stderr = [{kind="literal", value="failure\n"}]
observations = [
 {executable-sha256="PIN", invocation={path="one/invocation.json",sha256="PIN"},stdout={path="one/stdout.bin",sha256="PIN"},stderr={path="one/stderr.bin",sha256="PIN"}},
 {executable-sha256="PIN", invocation={path="two/invocation.json",sha256="PIN"},stdout={path="two/stdout.bin",sha256="PIN"},stderr={path="two/stderr.bin",sha256="PIN"}}
]
"#.replace("PIN", &"a".repeat(64))
}
#[test]
fn tagged_process_certificate_and_declared_json_path_survive_real_case_toml_decoding() {
    let mut values = case_reference_wires(&source()).unwrap();
    assert_eq!(values.len(), 1);
    let (name, wire, basis) = values.pop().unwrap();
    assert!(matches!(
        basis,
        crate::outbound::suite::types::discrepancy::DiscrepancyBasisWire::LanguageDefect
    ));
    assert_eq!(name, "encoded-path");
    assert!(wire.expected.is_empty());
    assert_eq!(wire.evidence.len(), 1);
    let ProcessFailureWire::ProcessFailure {
        exit,
        stdout,
        observations,
        ..
    } = wire.process.unwrap();
    assert_eq!(exit, 2);
    assert_eq!(observations.len(), 2);
    assert!(
        matches!(&stdout[1], OutputPartWire::InputPath { file, encoding:PathEncodingWire::JsonStringContent } if file == std::path::Path::new("case.py"))
    );
}
#[test]
fn unknown_process_variants_and_unknown_byte_normalizations_are_not_decoded() {
    assert!(
        case_reference_wires(
            &source().replace("kind = \"process-failure\"", "kind = \"normal-report\"")
        )
        .is_err()
    );
    assert!(
        case_reference_wires(&source().replace("json-string-content", "strip-all-paths")).is_err()
    );
    assert!(
        case_reference_wires(&source().replace("exit = 2", "exit = 2\nignore-backtrace = true"))
            .is_err()
    );
}

#[test]
fn one_case_cannot_declare_two_discrepancy_authorities() {
    let duplicate = source().replace(
        "expected = []\n[cases.reference-discrepancy]",
        "expected = []\ntool-contract-difference = {reason=\"other policy\", expected=[], identity={tool=\"toy\",version=\"1\",observed-executable-sha256=\"PIN\",profile-sha256=\"PIN\",settings-sha256=\"PIN\",source-sha256=\"PIN\",reference-output-sha256=\"PIN\",intended-output-sha256=\"PIN\"},evidence=[]}\n[cases.reference-discrepancy]",
    ).replace("PIN", &"a".repeat(64));
    assert!(case_reference_wires(&duplicate).is_err());
}

#[test]
fn authored_toml_decodes_each_evidence_kind_with_its_documented_field_names() {
    use crate::outbound::suite::types::discrepancy::{DiscrepancyBasisWire, EvidenceWire};
    let base = source()
        .split("[cases.reference-discrepancy.process]")
        .next()
        .unwrap()
        .to_owned();
    let language_fields = "kind = \"language-spec\"\nlanguage-version = \"1\"\ntarget-configuration = \"target-v1\"\nversion-rationale = \"same supported semantics\"";
    assert!(base.contains(language_fields));
    let independent = base.replace(
        "kind = \"language-spec\"",
        "kind = \"independent-language-run\"",
    );
    let contract = base
        .replace("cases.reference-discrepancy", "cases.tool-contract-difference")
        .replace(language_fields, &format!("kind = \"public-tool-contract\"\nsource-revision = \"{}\"\nsource-path = \"crates/tool/src/api.rs\"\nrationale = \"public contract\"", "b".repeat(40)));
    let prior = base
        .replace("cases.reference-discrepancy", "cases.tool-contract-difference")
        .replace(language_fields, &format!("kind = \"prior-worker-run\"\nworker-sha256 = \"{0}\"\ncontrol-source-sha256 = \"{0}\"\ncontrol-query-sha256 = \"{0}\"\ncontrol-profile-sha256 = \"{0}\"\nwitness-case = \"positive\"\nrationale = \"prior control\"", "c".repeat(64)));
    for (text, kind) in [
        (base, "language"),
        (independent, "independent"),
        (contract, "contract"),
        (prior, "prior"),
    ] {
        let mut cases = case_reference_wires(&text).expect("authored TOML spelling");
        assert_eq!(cases.len(), 1);
        let (_name, wire, basis) = cases.pop().unwrap();
        assert_eq!(wire.evidence.len(), 1);
        assert!(match kind {
            "language" => matches!(
                (&wire.evidence[0], basis),
                (
                    EvidenceWire::LanguageSpec { .. },
                    DiscrepancyBasisWire::LanguageDefect
                )
            ),
            "independent" => matches!(
                (&wire.evidence[0], basis),
                (
                    EvidenceWire::IndependentLanguageRun { .. },
                    DiscrepancyBasisWire::LanguageDefect
                )
            ),
            "contract" => matches!(
                (&wire.evidence[0], basis),
                (
                    EvidenceWire::PublicToolContract { .. },
                    DiscrepancyBasisWire::ToolContract
                )
            ),
            _ => matches!(
                (&wire.evidence[0], basis),
                (
                    EvidenceWire::PriorWorkerRun { .. },
                    DiscrepancyBasisWire::ToolContract
                )
            ),
        });
    }
}
