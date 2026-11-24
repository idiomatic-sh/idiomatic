//! The shipping SDK reference, using one worker and a fresh in-memory run.
use ryo::linter::*;
use ryo::query::RyoQlQuery;
use std::{num::NonZeroUsize, path::Path};
pub(super) fn run(languages: Languages, pack: &Path, request: &Path, output: &Path) {
    let req = super::json_input::shipping(request);
    let globs = |v: &[String]| {
        v.iter()
            .map(|s| Glob::try_from(s.as_str()).expect("glob"))
            .collect()
    };
    let rules = req
        .selected_rules
        .iter()
        .map(|id| {
            let query =
                std::fs::read_to_string(pack.join("rules").join(id).join("query.ryoql")).unwrap();
            (
                RuleId::try_from(id.as_str()).unwrap(),
                Rule {
                    observations: Vec::new(),
                    query: RyoQlQuery::try_from(query.as_str()).unwrap(),
                    applicability: RuleApplicability::Always,
                    parameters: req.parameters.iter().cloned().collect(),
                    include: globs(&req.include),
                    exclude: globs(&req.exclude),
                },
            )
        })
        .collect();
    let files = req.inventory.unwrap_or(req.files);
    let corpus = if req.given {
        Corpus::Given(
            files
                .iter()
                .map(|file| CaseSource {
                    executable: ExecutablePermission::Unknown,
                    path: RelativePath(file.as_str().into()),
                    text: SourceText(
                        std::fs::read_to_string(req.root.join(file))
                            .expect("case text")
                            .into(),
                    ),
                })
                .collect(),
        )
    } else {
        Corpus::Listed(files.into_iter().map(|f| RelativePath(f.into())).collect())
    };
    let report = lint(
        languages,
        &WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::None,
        LintRequest {
            source_language: SourceLanguage::Recognized,
            root: req.root,
            project_search_scope: Default::default(),
            given_project_inventory: Default::default(),
            rules,
            files: Vec::new(),
            hosted: req.hosted,
            corpus,
            beside: Vec::new(),
        },
    )
    .expect("shipping query");
    assert!(
        report.unknowns.is_empty(),
        "shipping unknowns: {:?}",
        report.unknowns
    );
    super::json_output::write_findings(&report.findings, output);
    println!(
        "shipping findings {} unread {:?}",
        report.findings.len(),
        report.unread
    );
}
