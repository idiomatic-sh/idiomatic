//! Real source staging preserves metadata, declared absences and Given scope admission.
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::{factory::local, interface::SuitePort as _, types::ArtifactPath};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

fn fixture(root: &std::path::Path, mode: &str, declare: bool) -> std::path::PathBuf {
    let rules = root.join("rules");
    std::fs::create_dir_all(rules.join("A001")).unwrap();
    std::fs::write(root.join("benchmark.toml"), format!("tool='fixture'\nversion='1'\nsha256=['{}']\ndefaults=['A001']\nproject-context-mode='{mode}'\nfile-observation='posix-executable'\n", "a".repeat(64))).unwrap();
    std::fs::write(rules.join("A001/query.ryoql"), "let files = select path from \"filesystem.executable\"; emit files subject path key(path);").unwrap();
    let context = if mode == "listed-disk" {
        "context={logical-root='repo',auxiliary-files={}}\n"
    } else {
        ""
    };
    let metadata = if declare {
        "file-metadata={'case.txt'=true}\n"
    } else {
        ""
    };
    let cases = format!(
        "[[cases]]\nname='positive'\ncomparison='full'\n{context}files={{'case.txt'='data'}}\n{metadata}expected=[{{rule='A001',file='case.txt',start=0,end=0}}]\n[[cases]]\nname='negative'\ncomparison='full'\n{context}files={{'case.txt'='data'}}\n{metadata}expected=[]\n"
    );
    std::fs::write(rules.join("A001/cases.toml"), cases).unwrap();
    rules
}

#[cfg(unix)]
fn both_materialization_routes_stage_and_reverify_declared_mode_before_each_attempt() {
    for mode in ["isolated", "listed-disk"] {
        let root = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let rules = fixture(root.path(), mode, true);
        let cases = service::cases(&local(), &rules, scratch.path()).unwrap();
        for case in cases {
            local().validate_file_metadata(&case.request).unwrap();
            let path = case.request.root.join("case.txt");
            let before = std::fs::metadata(&path).unwrap();
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(before.permissions().mode() & !0o111),
            )
            .unwrap();
            assert!(local().validate_file_metadata(&case.request).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), b"data");
            assert_eq!(
                std::fs::metadata(&path).unwrap().modified().unwrap(),
                before.modified().unwrap()
            );
            std::fs::set_permissions(&path, before.permissions()).unwrap();
            local().validate_file_metadata(&case.request).unwrap();
            std::fs::remove_file(&path).unwrap();
            assert!(local().validate_file_metadata(&case.request).is_err());
        }
    }
}

fn required_profile_cannot_admit_omitted_observations_even_for_negative_cases() {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), "isolated", false);
    assert!(service::cases(&local(), &rules, scratch.path()).is_err());
}

fn sealed_missing_request_needs_metadata_only_for_observed_sources() {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), "listed-disk", true);
    std::fs::write(
        rules.join("A001/cases.toml"),
        "[[cases]]\nname='observed-plus-missing'\ncomparison='full'\nrequested-files=['case.txt','missing.txt']\nfiles={'case.txt'='data'}\nfile-metadata={'case.txt'=true}\ncontext={logical-root='repo',auxiliary-files={},missing=['repo/missing.txt']}\nexpected=[{rule='A001',file='case.txt',start=0,end=0}]\n[[cases]]\nname='negative-with-missing'\ncomparison='full'\nrequested-files=['case.txt','missing.txt']\nfiles={'case.txt'='data'}\nfile-metadata={'case.txt'=true}\ncontext={logical-root='repo',auxiliary-files={},missing=['repo/missing.txt']}\nexpected=[]\n",
    )
    .unwrap();
    let cases = service::cases(&local(), &rules, scratch.path())
        .expect("the missing requested path has no fabricated metadata");
    assert_eq!(cases.len(), 2);
    for case in cases {
        assert_eq!(case.request.file_metadata.as_ref().unwrap().len(), 1);
        assert!(!case.request.root.join("missing.txt").exists());
        local()
            .validate_file_metadata(&case.request)
            .expect("only the readable source is observed");
    }
    let path = rules.join("A001/cases.toml");
    let observed = std::fs::read_to_string(&path).unwrap();
    let fabricated = observed.replace(
        "file-metadata={'case.txt'=true}",
        "file-metadata={'case.txt'=true,'missing.txt'=true}",
    );
    assert_ne!(fabricated, observed);
    std::fs::write(path, fabricated).unwrap();
    assert!(
        service::cases(&local(), &rules, scratch.path()).is_err(),
        "a missing path cannot acquire fabricated file metadata"
    );
}
fn empty_cases_are_refused() {
    for empty_rule in [false, true] {
        let rules = tempfile::tempdir().expect("rules");
        let scratch = tempfile::tempdir().expect("scratch");
        if empty_rule {
            let directory = rules.path().join("fixture");
            std::fs::create_dir_all(&directory).expect("rule");
            std::fs::write(
                directory.join("query.ryoql"),
                "let rows = select * from comment; emit rows subject file key(span);",
            )
            .expect("query");
            std::fs::write(directory.join("cases.toml"), "cases = []").expect("cases");
        }
        let result = service::cases(&local(), rules.path(), scratch.path());
        let actual = result
            .expect_err("no cases cannot certify correctness")
            .to_string();
        let expected = if empty_rule {
            "case evidence must be nonempty"
        } else {
            "no executable rule cases"
        };
        assert_eq!(actual, expected);
    }
}

fn decoded_source_admission_preserves_report_and_given_scope() {
    let root = tempfile::tempdir().unwrap();
    let rule = "fixture".try_into().unwrap();
    std::fs::create_dir_all(root.path().join("rules/fixture")).unwrap();
    let path = root.path().join("rules/fixture/cases.toml");
    std::fs::write(&path, "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{rule='invalid-syntax',file='case.py',start=0,end=1}]").unwrap();
    let suite = local();
    let cases = suite
        .rule_cases(ArtifactPath(&root.path().join("rules")), &rule)
        .unwrap();
    crate::domain::suite::service::validate_case_sources(&cases).unwrap();
    assert_eq!(
        cases.cases[0].comparison,
        ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full
    );
    // The same formerly-schema refusal belongs to mandatory case-source admission.
    std::fs::write(&path, "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{file='case.py',start=0,end=1}]").unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let refusal = service::cases(&suite, &root.path().join("rules"), scratch.path())
        .expect_err("full report cannot inherit identity");
    assert!(
        matches!(refusal, ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message) if message == "full-report expectations require explicit rule identities")
    );
    assert_eq!(
        std::fs::read_dir(scratch.path()).unwrap().count(),
        0,
        "admission refuses before source materialization"
    );
    let case = "[[cases]]\nname='sibling'\ncomparison='full'\nrequested-files=['main.py']\nproject-search-roots=['']\nfiles={'main.py'='import localmod', 'localmod.py'='value=1'}\ngiven-project-inventory={completeness='partial', entries=[{path='localmod.py', state='file'}]}\nexpected=[]\n";
    std::fs::write(&path, case).unwrap();
    let cases = suite
        .rule_cases(ArtifactPath(&root.path().join("rules")), &rule)
        .unwrap();
    crate::domain::suite::service::validate_case_sources(&cases)
        .expect("unselected sibling is staged under explicit Given authority");
    assert_eq!(
        cases.cases[0]
            .given_project_inventory
            .as_ref()
            .unwrap()
            .entries
            .len(),
        1
    );
    assert_eq!(
        cases.cases[0]
            .given_project_inventory
            .as_ref()
            .unwrap()
            .entries[0]
            .state,
        ryo::linter::ProjectEntryState::File
    );
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    #[cfg(unix)]
    Case {
        name: CaseName(
            "both_materialization_routes_stage_and_reverify_declared_mode_before_each_attempt",
        ),
        word: Word::Succeeds,
        input: || {
            both_materialization_routes_stage_and_reverify_declared_mode_before_each_attempt
                as fn()
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "required_profile_cannot_admit_omitted_observations_even_for_negative_cases",
        ),
        word: Word::Succeeds,
        input: || {
            required_profile_cannot_admit_omitted_observations_even_for_negative_cases as fn()
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("sealed_missing_request_needs_metadata_only_for_observed_sources"),
        word: Word::Succeeds,
        input: || sealed_missing_request_needs_metadata_only_for_observed_sources as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_cases_are_refused"),
        word: Word::Succeeds,
        input: || empty_cases_are_refused as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("decoded_source_admission_preserves_report_and_given_scope"),
        word: Word::Succeeds,
        input: || decoded_source_admission_preserves_report_and_given_scope as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
