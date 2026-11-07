//! The test data of `linter lint` that fixes capability cache.

use crate::domain::capacity::types::MemoryBudget;
use crate::domain::file::types::RelativePath;
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::Corpus;
use crate::resources::linter::lint::types::LintRequest;
use crate::resources::linter::lint::types::Rule;
use crate::resources::linter::lint::types::RuleId;
use crate::resources::linter::lint::types::Workers;
use ryo_relations::domain::capabilities::types::OptionalCapability;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use super::outcome::Outcome;
use super::queries::{LITERAL_PARSE_POSITIVE, OPTIONAL_POSITIVE, RAW_FORMAT_POSITIVE};
use super::capability_provider::CapabilityProvider;
use super::rules::ordinary;

pub(crate) fn capability_cache(query: &str) -> Outcome {
    use ryo_relations::domain::capabilities::types::OptionalCapability;
    let advertised_capabilities: &'static [OptionalCapability] = if query == RAW_FORMAT_POSITIVE
    {
        &[
            OptionalCapability::RawFormatSegment,
            OptionalCapability::RawFormatField,
        ]
    } else if query == LITERAL_PARSE_POSITIVE {
        &[OptionalCapability::LiteralExpressionParse]
    } else {
        &[OptionalCapability::Identifier]
    };
    let root = tempfile::tempdir().expect("corpus");
    let store = tempfile::tempdir().expect("store");
    std::fs::write(root.path().join("case.txt"), "x").expect("source");
    let language = |caps, configuration, name| {
        crate::domain::language::types::Language::from_parts(
            crate::domain::language::types::LanguageName("fixture".into()),
            crate::domain::language::types::LanguageBuild("same-build".into()),
            std::sync::Arc::new(CapabilityProvider(caps, configuration, name)),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )
    };
    let engine = |caps, configuration, name, cache| {
        crate::resources::linter::lint::types::Linter::new(
            Languages(vec![language(caps, configuration, name)]),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    };
    let request = LintRequest {
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        source_language: crate::domain::language::types::SourceLanguage::Recognized,
        root: root.path().to_path_buf(),
        rules: BTreeMap::from([(
            RuleId::try_from("optional").expect("rule"),
            Rule {
                observations: Vec::new(),
                query: crate::resources::query::RyoQlQuery::try_from(query).expect("query"),
                applicability: crate::resources::linter::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: vec![
                    crate::resources::linter::lint::types::Glob::try_from("*.txt")
                        .expect("glob"),
                ],
                exclude: vec![],
            },
        )]),
        files: vec![],
        hosted: vec![],
        corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
        beside: vec![],
    };
    let forward = crate::resources::linter::lint::connections::build(&[language(
        &[
            OptionalCapability::Identifier,
            OptionalCapability::QualifiedName,
        ],
        "",
        "x",
    )]);
    let reverse = crate::resources::linter::lint::connections::build(&[language(
        &[
            OptionalCapability::QualifiedName,
            OptionalCapability::Identifier,
        ],
        "",
        "x",
    )]);
    assert_eq!(
        forward, reverse,
        "capability declaration order does not change identity"
    );

    let advertised = engine(
        advertised_capabilities,
        "",
        "x",
        Cache::At(store.path().to_path_buf()),
    );
    let first = crate::resources::linter::lint::entry::lint_with(&advertised, request.clone())
        .expect("advertised report");
    assert_eq!(
        first.findings.len(),
        1,
        "the cache must contain a real optional finding"
    );
    let mut selected = request.clone();
    selected.source_language = crate::domain::language::types::SourceLanguage::Named(
        crate::domain::language::types::LanguageName("fixture".into()),
    );
    let explicit =
        crate::resources::linter::lint::entry::lint_with(&advertised, selected.clone())
            .expect("explicit refusal report");
    assert!(
        explicit.findings.is_empty(),
        "cached path-recognized findings cannot bypass explicit opt-in"
    );
    assert_eq!(
        explicit.unread.len(),
        1,
        "a non-opting provider must refuse explicit selection"
    );
    let fresh_explicit = crate::resources::linter::lint::entry::lint_with(
        &engine(advertised_capabilities, "", "x", Cache::None),
        selected.clone(),
    )
    .expect("fresh explicit refusal");
    assert_eq!(
        explicit, fresh_explicit,
        "source selection must be in cache identity"
    );
    assert_eq!(
        crate::resources::linter::lint::entry::lint_with(&advertised, request.clone())
            .expect("ordinary selection restored"),
        first
    );
    selected.source_language = crate::domain::language::types::SourceLanguage::Named(
        crate::domain::language::types::LanguageName("missing-provider".into()),
    );
    assert!(matches!(
        crate::resources::linter::lint::entry::lint_with(&advertised, selected),
        Err(crate::resources::linter::lint::errors::LintError::SourceLanguage { .. })
    ));

    let ambiguous = crate::resources::linter::lint::types::Linter::new(
        Languages(vec![
            language(advertised_capabilities, "", "x"),
            language(advertised_capabilities, "", "x"),
        ]),
        crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::None,
    );
    let mut ambiguous_request = request.clone();
    ambiguous_request.source_language = crate::domain::language::types::SourceLanguage::Named(
        crate::domain::language::types::LanguageName("fixture".into()),
    );
    assert!(matches!(
        crate::resources::linter::lint::entry::lint_with(&ambiguous, ambiguous_request),
        Err(crate::resources::linter::lint::errors::LintError::SourceLanguage { .. })
    ));

    let absent = engine(&[], "", "x", Cache::At(store.path().to_path_buf()));
    let replay = crate::resources::linter::lint::entry::lint_with(&absent, request.clone())
        .expect("unadvertised report");
    let fresh = crate::resources::linter::lint::entry::lint_with(
        &engine(&[], "", "x", Cache::None),
        request.clone(),
    )
    .expect("fresh unadvertised report");
    assert_eq!(
        replay, fresh,
        "retained data must not resurrect unadvertised findings"
    );
    assert!(
        fresh.findings.is_empty(),
        "unadvertised means unavailable, not a positive"
    );
    assert!(
        !fresh.unknowns.is_empty(),
        "unavailability remains observable"
    );
    let restored = engine(
        advertised_capabilities,
        "",
        "x",
        Cache::At(store.path().to_path_buf()),
    );
    assert_eq!(
        crate::resources::linter::lint::entry::lint_with(&restored, request.clone())
            .expect("restored report"),
        first,
        "prior unavailable state cannot poison newly advertised capability results"
    );

    if query == OPTIONAL_POSITIVE {
        let target_310 = engine(
            &[OptionalCapability::Identifier],
            "python-target=3.10",
            "x",
            Cache::At(store.path().to_path_buf()),
        );
        let before_target_change =
            crate::resources::linter::lint::entry::lint_with(&target_310, request.clone())
                .expect("Python 3.10 result");
        let target_314 = engine(
            &[OptionalCapability::Identifier],
            "python-target=3.14",
            "y",
            Cache::At(store.path().to_path_buf()),
        );
        let after_target_change =
            crate::resources::linter::lint::entry::lint_with(&target_314, request.clone())
                .expect("Python 3.14 result");
        let fresh_314 = crate::resources::linter::lint::entry::lint_with(
            &engine(
                &[OptionalCapability::Identifier],
                "python-target=3.14",
                "y",
                Cache::None,
            ),
            request.clone(),
        )
        .expect("fresh Python 3.14 result");
        assert_ne!(before_target_change, after_target_change);
        assert_eq!(after_target_change, fresh_314);
        let restored_310 = crate::resources::linter::lint::entry::lint_with(
            &engine(
                &[OptionalCapability::Identifier],
                "python-target=3.10",
                "x",
                Cache::At(store.path().to_path_buf()),
            ),
            request.clone(),
        )
        .expect("restored Python 3.10 result");
        assert_eq!(restored_310, before_target_change);
        let stable_identity = crate::resources::linter::lint::connections::build(&[language(
            &[OptionalCapability::Identifier],
            "python-target=3.14",
            "y",
        )]);
        assert_eq!(
            stable_identity,
            crate::resources::linter::lint::connections::build(&[language(
                &[OptionalCapability::Identifier],
                "python-target=3.14",
                "y",
            )]),
            "unchanged parser settings keep the same cache identity"
        );
        assert_ne!(
            stable_identity,
            crate::resources::linter::lint::connections::build(&[language(
                &[OptionalCapability::Identifier],
                "python-target=3.10",
                "x",
            )]),
            "changed parser settings change cache identity"
        );
    }
    (1, 0, true, 0)
}
