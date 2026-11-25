//! The public corpus measurement over the fakes of its boundaries.
use benchmark::inbound::use_cases::measure_corpus::{
    fixtures::{
        bench_of, discovering, measuring, project_of, use_case, with_corpus, with_suffixes,
    },
    interface::MeasureCorpusUseCase as _,
    types::SourceSuffix,
};
use cucumber::then;

#[then("measure_corpus refuses a corpus of no project, publishing no cost")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(!measure().0);
}

#[then("measure_corpus counts a.ts and b.mjs in the corpus shape and not c.vue")]
fn suffixed(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let suffixes = [".ts", ".mts", ".cts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"].map(SourceSuffix);
    let files = ["a.ts", "b.mjs", "c.vue"];
    let root = project_of(&files);
    let cache = project_of(&[]);
    let bench = bench_of(&["A"]);
    let mut observations = Vec::new();
    let measured = with_suffixes((bench.path(), cache.path()), &suffixes, |corpus| {
        measuring(root.path(), &files).measure_corpus(corpus, &mut observations)
    });
    let observations = String::from_utf8_lossy(&observations);
    // Past the corpus shape, both matched files are measured: the tracked check passed and their bytes were read. The fixture's
    // certificate seals no evidence, so the run then stops at the certified-evidence admission, after the shape this scenario is about.
    assert!(
        observations.contains(
            "project-0: matched Fixture source scope 2 files; excluded default-discovered formats 1 files (source-kind capability gap): c.vue\nproject-0: measured source files=2 bytes=4 "
        ),
        "{observations}"
    );
    assert!(
        observations.contains("project-0: corpus comparison failed: certified profile, rules, cases or executable changed\n"),
        "{measured:?}\n{observations}"
    );
    assert!(
        measured.is_err_and(|refusal| refusal.to_string()
            == "corpus comparisons failed: [ProjectName(\"project-0\")]")
    );
}

/// Whether an empty corpus run succeeded, and what it wrote.
fn measure() -> (bool, String) {
    let mut observations = Vec::new();
    let result = with_corpus(0, |corpus| {
        use_case().measure_corpus(corpus, &mut observations)
    });
    (
        result.is_ok(),
        String::from_utf8_lossy(&observations).into_owned(),
    )
}

#[then("measure_corpus refuses a project whose every file the suffix filter excludes, naming it")]
fn nothing_to_lint(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let suffixes = [".ts"].map(SourceSuffix);
    let files = ["c.vue"];
    let root = project_of(&files);
    let cache = project_of(&[]);
    let bench = bench_of(&["A"]);
    let mut observations = Vec::new();
    let measured = with_suffixes((bench.path(), cache.path()), &suffixes, |corpus| {
        discovering(root.path(), &files).measure_corpus(corpus, &mut observations)
    });
    let observations = String::from_utf8_lossy(&observations);
    assert!(
        observations.contains(
            "project-0: corpus comparison failed: no source left to lint after discovery and the valid profile\n"
        ),
        "{measured:?}\n{observations}"
    );
    assert!(measured.is_err(), "{observations}");
}
