//! What links the files to one another follows an edit of one of them, whatever the store holds.

use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::Rule;
#[cfg(all(feature = "json", feature = "javascript"))]
use ryo_sdk::linter::{
    Cache as SdkCache, Corpus, Languages, LintRequest, MemoryBudget, Rule as SdkRule, RuleId,
};
#[cfg(all(feature = "json", feature = "javascript"))]
use ryo_sdk::query::RyoQlQuery;

use crate::cache::{sorted, write};
use crate::counting;

/// The rule that reads how the files link to one another.
fn linking() -> Vec<Rule> {
    vec![crate::rule_from_query(
        "links",
        "let d = select * from \"source-dependency\"; emit d subject source key(target);",
        &["**".to_owned()],
    )]
}

#[test]
fn what_links_the_files_follows_an_edit_of_one_whatever_the_store_holds() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"linked\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
                    .to_owned(),
            ),
            ("src/lib.rs", "mod a;\nmod b;\n".to_owned()),
            ("src/a.rs", "pub fn f() {}\n".to_owned()),
            ("src/b.rs", "pub fn g() {}\n".to_owned()),
        ],
    );
    let rows = |kept| {
        sorted(
            counting::lint_counting(root.path(), linking(), kept)
                .expect("a run")
                .rows,
        )
    };
    assert_eq!(
        rows(Arc::clone(&store)),
        Vec::<Vec<String>>::new(),
        "nothing refers to anything yet"
    );
    assert_eq!(
        rows(Arc::clone(&store)),
        Vec::<Vec<String>>::new(),
        "and the run after it says the same"
    );
    // `a` now writes a path into `b`.
    write(
        root.path(),
        &[(
            "src/a.rs",
            "use crate::b::g;\npub fn f() { g(); }\n".to_owned(),
        )],
    );
    let after = rows(Arc::clone(&store));
    let oracle = sorted(
        counting::lint_counting(
            root.path(),
            linking(),
            Arc::new(ryo_core::outbound::lint_store::factory::memory()),
        )
        .expect("a run that keeps nothing")
        .rows,
    );
    assert_eq!(
        after, oracle,
        "the answer is the one a run that keeps nothing gives"
    );
    assert_eq!(after.len(), 1, "a refers to b: {after:?}");
    // The edit is undone.
    write(root.path(), &[("src/a.rs", "pub fn f() {}\n".to_owned())]);
    assert_eq!(rows(store), Vec::<Vec<String>>::new());
}

/// A rule of one file, and a rule of the project's dependencies, over one manifest: the words the second hands cannot be moved to
/// another run, so the manifest is read again, with both rules' findings.
const IDENTITY: &str = r#"let roots = select * from "mapping" where owner = id;
let names = select * from "entry" where key = "name" and kind = "string" and decoded != "";
let descriptions = select * from "entry" where key = "description" and kind = "string" and decoded != "";
let named = select l."id" as "id" from roots as l join names as r on l."id" = r."owner";
let described = select l."id" as "id" from roots as l join descriptions as r on l."id" = r."owner";
let unnamed = antijoin(left=roots, right=named, left-on=["id"], right-on=["id"]);
let undescribed = antijoin(left=roots, right=described, left-on=["id"], right-on=["id"]);
let lacking = union(left=unnamed, right=undescribed);
let placed = select l."file" as "file", r."span" as "span" from lacking as l join "syntax-tree" as r on l."id" = r."id";
emit placed subject "file" key("span");
"#;
const PINNED: &str = r#"let deps = select * from "package-dependency" where origin = "registry";
let keyed = select l."source" as "file", l."package" as "package", l."requirement" as "requirement", l."exact" as "exact", r."span" as "span" from deps as l join "entry" as r on l."source" = r."file" where r."key" = l."package" and r."kind" != "mapping";
let listed = select l."source" as "file", l."package" as "package", l."requirement" as "requirement", l."exact" as "exact", r."span" as "span" from deps as l join "sequence-element" as r on l."source" = r."file" where r."decoded" = l."package" or starts_with(r."decoded", concat(l."package", "=")) or starts_with(r."decoded", concat(l."package", "[")) or starts_with(r."decoded", concat(l."package", ">")) or starts_with(r."decoded", concat(l."package", "<")) or starts_with(r."decoded", concat(l."package", "~")) or starts_with(r."decoded", concat(l."package", "!"));
let located = union(left=keyed, right=listed);
let found = project(from=located, columns=["file", "package"]);
let unlocated = antijoin(left=deps, right=found, left-on=["source", "package"], right-on=["file", "package"]);
let roots = antijoin(left="syntax-tree", right="syntax-child", left-on=["id"], right-on=["child"]);
let placed-at-top = select l."source" as "file", l."package" as "package", l."requirement" as "requirement", l."exact" as "exact", r."span" as "span" from unlocated as l join roots as r on l."source" = r."file";
let every = union(left=located, right=placed-at-top);
let unpinned = select * from every where exact = "false";
emit unpinned subject "file" key("span");
"#;

#[cfg(all(feature = "json", feature = "javascript"))]
#[test]
fn a_file_whose_words_cannot_move_is_read_again_and_keeps_its_findings() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("root");
    let cache = tempfile::tempdir().expect("cache");
    std::fs::write(root.path().join("package.json"), "{\n  \"name\": \"\",\n  \"description\": \"\",\n  \"dependencies\": { \"hono\": \"^4.0.0\" }\n}\n").expect("w");
    std::fs::write(root.path().join("a.js"), "// one\n").expect("w");
    let languages = Languages(vec![ryo_json::linked(), ryo_javascript::linked()]);
    let rules = std::collections::BTreeMap::from([
        (
            RuleId::try_from("identity").expect("id"),
            SdkRule {
                query: RyoQlQuery::try_from(IDENTITY).expect("q"),
                applicability: ryo_sdk::linter::RuleApplicability::Always,
                parameters: std::collections::BTreeMap::new(),
                include: vec![ryo_sdk::linter::Glob::try_from("package.json").expect("g")],
                exclude: Vec::new(),
            },
        ),
        (
            RuleId::try_from("pinned").expect("id"),
            SdkRule {
                query: RyoQlQuery::try_from(PINNED).expect("q"),
                applicability: ryo_sdk::linter::RuleApplicability::Always,
                parameters: std::collections::BTreeMap::new(),
                include: vec![ryo_sdk::linter::Glob::try_from("package.json").expect("g")],
                exclude: Vec::new(),
            },
        ),
    ]);
    let run = || {
        let request = LintRequest {
            root: root.path().to_path_buf(),
            rules: rules.clone(),
            files: Vec::new(),
            hosted: Vec::new(),
            corpus: Corpus::Walked,
            beside: Vec::new(),
        };
        let pool = ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(
            std::num::NonZeroUsize::MIN.saturating_add(1),
        ));
        let report = ryo_sdk::linter::lint(
            languages.clone(),
            &pool,
            MemoryBudget::Machine,
            SdkCache::At(cache.path().to_path_buf()),
            request,
        )
        .expect("lint");
        let mut seen: Vec<String> = report
            .findings
            .iter()
            .map(|found| {
                let ryo_sdk::linter::Slot::Is(ryo_sdk::linter::Cell::File(path)) = &found.subject
                else {
                    panic!("a finding about a file");
                };
                format!("{}:{}", found.rule, path.0)
            })
            .collect();
        seen.sort();
        seen
    };
    let cold = run();
    assert_eq!(
        run(),
        cold,
        "an unchanged project answers the same findings"
    );
    std::fs::write(root.path().join("a.js"), "// two\n").expect("w");
    let warm = run();
    assert_eq!(warm, cold, "an edit elsewhere leaves them");
}
