//! Real retained-cache invalidation for legal hosted text that collides in the readable wire form.
use super::*;

#[test]
fn changed_hosted_text_invalidates_retained_cross_file_findings() {
    let root = tempfile::tempdir().expect("corpus");
    let store = tempfile::tempdir().expect("store");
    let source = root.path().join("case.txt");
    std::fs::write(&source, "x").expect("source");
    let modified = std::fs::metadata(&source).unwrap().modified().unwrap();
    let engine = |cache| {
        super::super::types::Linter::new(
            Languages(vec![crate::domain::language::types::Language::from_parts(
                crate::domain::language::types::LanguageName("fixture".into()),
                crate::domain::language::types::LanguageBuild("same-build".into()),
                std::sync::Arc::new(CapabilityProvider(&[], "host-identity", "unused")),
                std::sync::Arc::new(
                    ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
                ),
            )]),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let request = |first: &str, second: &str| {
        LintRequest {
        source_language: None, root: root.path().to_owned(), files: vec![], beside: vec![],
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        rules: BTreeMap::from([(RuleId::try_from("host-check").unwrap(), Rule {
            observations: Vec::new(),
            query: crate::resources::query::RyoQlQuery::try_from("let found = select file from \"host.values\" where first = \"x\ttext:y\"; emit found subject file key(file);").unwrap(),
            applicability: crate::resources::linter::RuleApplicability::Always,
            parameters: BTreeMap::new(), include: vec![], exclude: vec![],
        })]),
        hosted: vec![HostRelation {
            name: HostRelationName("host.values".into()),
            columns: vec![HostColumn { name: HostColumnName("file".into()), kind: HostKind::File }, HostColumn { name: HostColumnName("first".into()), kind: HostKind::Text }, HostColumn { name: HostColumnName("second".into()), kind: HostKind::Text }],
            rows: vec![HostRow(vec![Cell::File(RelativePath("case.txt".into())), Cell::Text(HostText(first.into())), Cell::Text(HostText(second.into()))])],
        }],
    }
    };
    let left = request("x\ttext:y", "z");
    let right = request("x", "y\ttext:z");
    let fresh_left = super::super::entry::lint_with(&engine(Cache::None), left.clone()).unwrap();
    let fresh_right = super::super::entry::lint_with(&engine(Cache::None), right.clone()).unwrap();
    assert_eq!(fresh_left.findings.len(), 1);
    assert!(fresh_right.findings.is_empty());
    let shared = engine(Cache::At(store.path().to_owned()));
    for expected in [
        (&left, &fresh_left),
        (&left, &fresh_left),
        (&right, &fresh_right),
        (&right, &fresh_right),
        (&left, &fresh_left),
    ] {
        assert_eq!(
            &super::super::entry::lint_with(&shared, expected.0.clone()).unwrap(),
            expected.1
        );
        assert_eq!(
            &super::super::entry::lint_with(
                &engine(Cache::At(store.path().to_owned())),
                expected.0.clone()
            )
            .unwrap(),
            expected.1,
            "fresh engine must also validate the persisted identity"
        );
    }
    assert_eq!(
        std::fs::metadata(source).unwrap().modified().unwrap(),
        modified
    );
}
