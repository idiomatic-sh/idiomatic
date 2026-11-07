//! Real source staging preserves metadata, declared absences and Given scope admission.
use super::constants;
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::{
    factory::local,
    interface::SuitePort as _,
    types::{ArtifactPath, CaseReceipts},
};
use std::borrow::Cow;

/// Which authored sources the preparation is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// A declared mode, staged by the named materialization route and reverified before each attempt.
    DeclaredMode(&'static str),
    /// A profile requiring file observations whose cases omit them.
    OmittedObservations,
    /// A requested path declared absent by its sealed context, with metadata for the readable source only.
    SealedMissing,
    /// The same sealed request with metadata fabricated for the missing path.
    FabricatedMetadata,
    /// A rule directory with no cases, or with an empty case list.
    EmptyCases { empty_rule: bool },
    /// A full-report case whose expectation inherits its rule identity.
    InheritedIdentity,
    /// A full-report case decoded with an explicit rule identity.
    DecodedParserCase,
    /// An unselected sibling staged under an explicit Given inventory.
    GivenSibling,
}

type Input = Scenario;
/// The number of cases prepared or decoded.
type Value = usize;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    #[cfg(unix)]
    Case {
        name: CaseName("the isolated route stages and reverifies the declared mode before each attempt"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredMode("isolated"),
        expect: Expect::Value(2),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("the listed disk route stages and reverifies the declared mode before each attempt"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredMode("listed-disk"),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a sealed missing request needs metadata only for observed sources"),
        word: Word::Succeeds,
        input: || Scenario::SealedMissing,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a decoded full-report case keeps its comparison"),
        word: Word::Succeeds,
        input: || Scenario::DecodedParserCase,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an unselected sibling is staged under explicit Given authority"),
        word: Word::Succeeds,
        input: || Scenario::GivenSibling,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a required profile cannot admit omitted observations even for negative cases"),
        word: Word::Refuses,
        input: || Scenario::OmittedObservations,
        expect: Expect::Error(Cow::Borrowed(constants::REQUIRED_METADATA)),
    },
    Case {
        name: CaseName("a missing path cannot acquire fabricated file metadata"),
        word: Word::Refuses,
        input: || Scenario::FabricatedMetadata,
        expect: Expect::Error(Cow::Borrowed("file metadata names an undeclared source")),
    },
    Case {
        name: CaseName("a rule directory with no rules has no cases"),
        word: Word::Refuses,
        input: || Scenario::EmptyCases { empty_rule: false },
        expect: Expect::Error(Cow::Borrowed("no executable rule cases")),
    },
    Case {
        name: CaseName("a rule with an empty case list has no evidence"),
        word: Word::Refuses,
        input: || Scenario::EmptyCases { empty_rule: true },
        expect: Expect::Error(Cow::Borrowed("case evidence must be nonempty")),
    },
    Case {
        name: CaseName("a full-report expectation cannot inherit its rule identity"),
        word: Word::Refuses,
        input: || Scenario::InheritedIdentity,
        expect: Expect::Error(Cow::Borrowed(
            "full-report expectations require explicit rule identities",
        )),
    },
];

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

/// Clears the execute bits of `path` and answers the permissions it had before.
fn clear_execute(path: &std::path::Path) -> std::fs::Permissions {
    let before = std::fs::metadata(path).unwrap().permissions();
    #[cfg(unix)]
    let cleared = <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(
        std::os::unix::fs::PermissionsExt::mode(&before) & !0o111,
    );
    #[cfg(not(unix))]
    let cleared = before.clone();
    std::fs::set_permissions(path, cleared).unwrap();
    before
}

fn declared_mode(mode: &str) -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), mode, true);
    let cases = service::cases(&local(CaseReceipts::Unrecorded), &rules, scratch.path())?;
    let count = cases.len();
    for case in cases {
        local(CaseReceipts::Unrecorded).validate_file_metadata(&case.request).unwrap();
        let path = case.request.root.join("case.txt");
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let before = clear_execute(&path);
        local(CaseReceipts::Unrecorded)
            .validate_file_metadata(&case.request)
            .map(drop)
            .expect_err("a cleared mode is refused");
        assert_eq!(std::fs::read(&path).unwrap(), b"data");
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), modified);
        std::fs::set_permissions(&path, before).unwrap();
        local(CaseReceipts::Unrecorded).validate_file_metadata(&case.request).unwrap();
        std::fs::remove_file(&path).unwrap();
        local(CaseReceipts::Unrecorded)
            .validate_file_metadata(&case.request)
            .map(drop)
            .expect_err("a removed source is refused");
    }
    Ok(count)
}

/// Two cases requesting `case.txt` and a missing path sealed absent, declaring `metadata`.
fn sealed_missing(rules: &std::path::Path, metadata: &str) {
    let case = |name: &str, expected: &str| {
        format!(
            "[[cases]]\nname='{name}'\ncomparison='full'\nrequested-files=['case.txt','missing.txt']\nfiles={{'case.txt'='data'}}\nfile-metadata={{{metadata}}}\ncontext={{logical-root='repo',auxiliary-files={{}},missing=['repo/missing.txt']}}\nexpected={expected}\n"
        )
    };
    std::fs::write(
        rules.join("A001/cases.toml"),
        case(
            "observed-plus-missing",
            "[{rule='A001',file='case.txt',start=0,end=0}]",
        ) + &case("negative-with-missing", "[]"),
    )
    .unwrap();
}

fn sealed_missing_request() -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), "listed-disk", true);
    sealed_missing(&rules, "'case.txt'=true");
    let cases = service::cases(&local(CaseReceipts::Unrecorded), &rules, scratch.path())?;
    let count = cases.len();
    for case in cases {
        assert!(matches!(
            &case.request.file_metadata,
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(values) if values.len() == 1
        ));
        assert!(!case.request.root.join("missing.txt").exists());
        local(CaseReceipts::Unrecorded)
            .validate_file_metadata(&case.request)
            .expect("only the readable source is observed");
    }
    Ok(count)
}

fn fabricated_metadata() -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), "listed-disk", true);
    sealed_missing(&rules, "'case.txt'=true,'missing.txt'=true");
    service::cases(&local(CaseReceipts::Unrecorded), &rules, scratch.path()).map(|cases| cases.len())
}

fn omitted_observations() -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let rules = fixture(root.path(), "isolated", false);
    service::cases(&local(CaseReceipts::Unrecorded), &rules, scratch.path()).map(|cases| cases.len())
}

fn empty_cases(empty_rule: bool) -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
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
    service::cases(&local(CaseReceipts::Unrecorded), rules.path(), scratch.path()).map(|cases| cases.len())
}

/// A rule `fixture` whose `cases.toml` holds `toml`, decoded and admitted.
fn decoded(toml: &str) -> Result<(tempfile::TempDir, Vec<ryo_benchmark_contracts::domain::workloads::types::Case>), ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir().unwrap();
    let rule = "fixture".try_into().unwrap();
    std::fs::create_dir_all(root.path().join("rules/fixture")).unwrap();
    std::fs::write(root.path().join("rules/fixture/cases.toml"), toml).unwrap();
    let cases = local(CaseReceipts::Unrecorded).rule_cases(ArtifactPath(&root.path().join("rules")), &rule)?;
    crate::domain::suite::service::validate_case_sources(&cases)?;
    Ok((root, cases.cases))
}

fn decoded_parser_case() -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let (_root, cases) = decoded("[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{rule='invalid-syntax',file='case.py',start=0,end=1}]")?;
    assert_eq!(
        cases[0].comparison,
        ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Full
    );
    Ok(cases.len())
}

fn inherited_identity() -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("rules/fixture")).unwrap();
    std::fs::write(root.path().join("rules/fixture/cases.toml"), "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{file='case.py',start=0,end=1}]").unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let answer = service::cases(&local(CaseReceipts::Unrecorded), &root.path().join("rules"), scratch.path())
        .map(|cases| cases.len());
    assert_eq!(
        std::fs::read_dir(scratch.path()).unwrap().count(),
        0,
        "admission refuses before source materialization"
    );
    answer
}

fn given_sibling() -> Result<Value, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let (_root, cases) = decoded("[[cases]]\nname='sibling'\ncomparison='full'\nrequested-files=['main.py']\nproject-search-roots=['']\nfiles={'main.py'='import localmod', 'localmod.py'='value=1'}\ngiven-project-inventory={completeness='partial', entries=[{path='localmod.py', state='file'}]}\nexpected=[]\n")?;
    let ryo_benchmark_contracts::domain::workloads::types::InventoryDeclaration::Declared(inventory) = &cases[0].given_project_inventory else {
        panic!("the case declares its project inventory");
    };
    assert_eq!(inventory.entries.len(), 1);
    assert_eq!(inventory.entries[0].state, ryo::linter::ProjectEntryState::File);
    Ok(cases.len())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::DeclaredMode(mode) => declared_mode(mode),
        Scenario::OmittedObservations => omitted_observations(),
        Scenario::SealedMissing => sealed_missing_request(),
        Scenario::FabricatedMetadata => fabricated_metadata(),
        Scenario::EmptyCases { empty_rule } => empty_cases(empty_rule),
        Scenario::InheritedIdentity => inherited_identity(),
        Scenario::DecodedParserCase => decoded_parser_case(),
        Scenario::GivenSibling => given_sibling(),
    }
    .map_err(|error| error.to_string().into())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
