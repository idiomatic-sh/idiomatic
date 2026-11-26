//! A request that says `Corpus::Walked` is answered as it always was: the files below the root the repository does not ignore, without the
//! languages' skipped directories nor the directory that holds the store; a file that cannot be read is reported unread, and a root with
//! nothing in it is an empty answer and a root that does not exist the refusal `Io` of that request. The expected answers are the old path's, recorded on the
//! trunk before the executor took over.

#![cfg(feature = "rust")]

use ryo_sdk::linter::{Cache, Corpus, LintError, LintReport, Languages};

/// The answer of a walked request, as text: the files its query found, the files it reported unread, or the refusal.
fn summary(answer: Result<LintReport, LintError>) -> String {
    match answer {
        Ok(report) => {
            let mut found: Vec<String> = report
                .findings
                .into_iter()
                .map(|finding| match finding.subject {
                    ryo_sdk::linter::Slot::Is(ryo_sdk::linter::Cell::File(path)) => path.0.to_string(),
                    other => format!("{other:?}"),
                })
                .collect();
            found.sort();
            let mut unread: Vec<String> = report.unread.into_iter().map(|each| each.path.0.to_string()).collect();
            unread.sort();
            format!("found {found:?} unread {unread:?}")
        }
        Err(error) => format!("refused {}", format!("{error:?}").split([' ', '{', '(']).next().unwrap_or_default()),
    }
}

/// The walked request over `root` whose rule lists every file it inventories, which reads no file's contents.
fn walked(root: &std::path::Path, cache: Cache) -> String {
    walked_by(root, cache, "let f = select * from \"filesystem.file\"; emit f subject path key(path);")
}

/// The walked request over `root` whose rule is `files`.
fn walked_by(root: &std::path::Path, cache: Cache, files: &str) -> String {
    let mut request = super::sdk::sdk_request(root, files, Vec::new());
    request.corpus = Corpus::Walked;
    let pool = ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(std::num::NonZeroUsize::MIN.saturating_add(1)));
    summary(run(Languages(vec![ryo_rust::linked()]), &pool, cache, request))
}

/// One request through the SDK's linter.
fn run(
    languages: Languages,
    pool: &ryo_sdk::linter::WorkerPool,
    cache: Cache,
    request: ryo_sdk::linter::LintRequest,
) -> Result<LintReport, LintError> {
    ryo_sdk::linter::Linter::new(languages, pool.clone(), cache)
        .lint(vec![request])
        .pop()
        .expect("one result for the one request")
}

fn write(root: &std::path::Path, path: &str, text: &str) {
    let at = root.join(path);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
    std::fs::write(at, text).expect("a file");
}

#[test]
fn a_walk_skips_the_languages_directories_and_the_ignored_ones() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a project");
    write(root.path(), "src/a.rs", "fn a() {}");
    write(root.path(), "target/skipped.rs", "fn s() {}");
    write(root.path(), "ignored/i.rs", "fn i() {}");
    write(root.path(), ".gitignore", "ignored/\n");
    assert_eq!(walked(root.path(), Cache::None), "found [\".gitignore\", \"src/a.rs\"] unread [\".gitignore\"]");
}

#[test]
fn a_walk_does_not_enter_the_store_beneath_its_root() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a project");
    write(root.path(), "src/a.rs", "fn a() {}");
    write(root.path(), ".kept/inside.rs", "fn k() {}");
    assert_eq!(walked(root.path(), Cache::At(root.path().join(".kept"))), "found [\"src/a.rs\"] unread []");
}

/// A project with a file that cannot be read, and one that can.
#[cfg(unix)]
fn with_a_locked_file() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt as _;
    let root = tempfile::tempdir().expect("a project");
    write(root.path(), "src/a.rs", "// fine\nfn a() {}");
    write(root.path(), "src/locked.rs", "// locked\nfn l() {}");
    std::fs::set_permissions(root.path().join("src/locked.rs"), std::fs::Permissions::from_mode(0o000)).expect("permissions");
    root
}

/// Decided 2026-11-26: a file no rule needs is not opened and not named; the old engine opened every inventoried file and named this one.
#[cfg(unix)]
#[test]
fn a_file_no_rule_reads_is_not_opened_and_not_named() {
    let _beside = crate::beside();
    let root = with_a_locked_file();
    assert_eq!(walked(root.path(), Cache::None), "found [\"src/a.rs\", \"src/locked.rs\"] unread []");
}

/// A file a rule needs and the lint cannot read is reported unread, as the old engine did, and the other files' findings stay.
#[cfg(unix)]
#[test]
fn a_file_a_rule_needs_and_cannot_read_is_reported_unread() {
    let _beside = crate::beside();
    let root = with_a_locked_file();
    let comments = "let c = select * from \"comment\"; emit c subject file key(span);";
    assert_eq!(walked_by(root.path(), Cache::None, comments), "found [\"src/a.rs\"] unread [\"src/locked.rs\"]");
}

#[test]
fn an_empty_root_is_an_empty_answer() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("an empty directory");
    assert_eq!(walked(root.path(), Cache::None), "found [] unread []");
}

#[test]
fn a_root_that_does_not_exist_is_refused() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    assert_eq!(walked(&root.path().join("nowhere"), Cache::None), "refused Io");
}
