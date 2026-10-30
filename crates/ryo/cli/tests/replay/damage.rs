//! A store cut short or damaged is no store: the run after it answers what a run that keeps nothing gives.

use std::sync::Arc;

use crate::cache::{function, rules, sorted, uncached, write};
use crate::counting;

#[test]
fn a_store_cut_short_damaged_or_removed_is_empty_and_the_run_answers_as_with_none() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
            ("three/lib.rs", function("alone", 40, "2")),
        ],
    );
    let cold =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a cold run");
    let oracle = sorted(uncached(root.path()).rows);
    assert_eq!(sorted(cold.rows), oracle);
    let file = root.path().join(".ryo_cache/lint.store");
    // A store cut short, as a copy that stopped in the middle would leave it, is no store.
    let mut bytes = std::fs::read(&file).expect("the store");
    bytes.truncate(bytes.len() / 2);
    std::fs::write(&file, bytes).expect("cut");
    let after_a_cut =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a run");
    assert_eq!(
        after_a_cut.reads, 3,
        "a store cut short is empty, so every file is read"
    );
    assert_eq!(sorted(after_a_cut.rows), oracle);
    // Bytes that are not a store are no store either.
    std::fs::write(&file, b"not a store").expect("damaged");
    let after_damage =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a run");
    assert_eq!(after_damage.reads, 3);
    assert_eq!(sorted(after_damage.rows), oracle);
    // A store that is gone is none, and the run after it has one again.
    std::fs::remove_file(&file).expect("removed");
    let after_a_removal =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a run");
    assert_eq!(after_a_removal.reads, 3);
    assert_eq!(sorted(after_a_removal.rows), oracle);
    let warm = counting::lint_counting(root.path(), rules(), store).expect("a warm run");
    assert_eq!(warm.reads, 0);
    assert_eq!(sorted(warm.rows), oracle);
}

/// The next variable-length number of `contents` from `at`, which moves past it.
fn varint(contents: &[u8], at: &mut usize) -> usize {
    let (mut value, mut shift) = (0_usize, 0_u32);
    loop {
        let byte = *contents.get(*at).expect("a byte of the table of contents");
        *at += 1;
        value |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return value;
        }
        shift += 7;
    }
}

/// Where each section of a store file is, as `(start, length)`: the small tables, what the files handed but their words, then each
/// bucket, read from the table of contents at the end of the file.
fn sections(store: &[u8]) -> Vec<(usize, usize)> {
    let trailer = store.len() - 8;
    let length = usize::try_from(u64::from_le_bytes(
        store
            .get(trailer..)
            .and_then(|end| end.try_into().ok())
            .expect("eight bytes"),
    ))
    .expect("a length");
    let contents = store
        .get(trailer - length..trailer)
        .expect("the table of contents");
    let mut at = 0;
    let build = varint(contents, &mut at);
    at += build;
    let mut found = vec![
        (varint(contents, &mut at), varint(contents, &mut at)),
        (varint(contents, &mut at), varint(contents, &mut at)),
    ];
    for _ in 0..varint(contents, &mut at) {
        let _bucket = varint(contents, &mut at);
        found.push((varint(contents, &mut at), varint(contents, &mut at)));
    }
    found
}

#[test]
fn a_store_with_one_section_damaged_gives_the_findings_of_a_run_that_keeps_nothing() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
            ("three/lib.rs", function("alone", 40, "2")),
        ],
    );
    let oracle = sorted(uncached(root.path()).rows);
    let _cold =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a cold run");
    let file = root.path().join(".ryo_cache/lint.store");
    let count = sections(&std::fs::read(&file).expect("the store")).len();
    assert!(
        count > 2,
        "the store holds the buckets of the rules that cross files"
    );
    for which in 0..count {
        let mut bytes = std::fs::read(&file).expect("the store");
        let (start, length) = *sections(&bytes).get(which).expect("a section");
        let damaged = length.min(16);
        bytes
            .get_mut(start..start + damaged)
            .expect("the section")
            .fill(0xff);
        std::fs::write(&file, bytes).expect("damaged");
        let after = counting::lint_counting(root.path(), rules(), Arc::clone(&store))
            .expect("a run over a damaged store");
        assert_eq!(
            sorted(after.rows),
            oracle,
            "section {which} of the store is damaged: the answer is the one a run that keeps nothing gives"
        );
    }
}

/// `value` as the variable-length number a table of contents holds.
fn encoded(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let low = u8::try_from(value & 0x7f).expect("seven bits");
        value >>= 7;
        if value == 0 {
            out.push(low);
            return out;
        }
        out.push(low | 0x80);
    }
}

/// `store` with the length its table of contents gives the section of what the files handed set to `length`, the trailer fixed.
fn with_rest_length(store: &[u8], length: u64) -> Vec<u8> {
    let trailer = store.len() - 8;
    let toc_length = usize::try_from(u64::from_le_bytes(
        store
            .get(trailer..)
            .and_then(|end| end.try_into().ok())
            .expect("eight bytes"),
    ))
    .expect("a length");
    let toc_start = trailer - toc_length;
    let contents = store
        .get(toc_start..trailer)
        .expect("the table of contents");
    let mut at = 0;
    let build = varint(contents, &mut at);
    at += build;
    let _tables = (varint(contents, &mut at), varint(contents, &mut at));
    let before_rest = at;
    let rest_start = varint(contents, &mut at);
    let _rest_length = varint(contents, &mut at);
    let mut toc = contents.get(..before_rest).expect("the head").to_vec();
    toc.extend(encoded(u64::try_from(rest_start).expect("a start")));
    toc.extend(encoded(length));
    toc.extend(contents.get(at..).expect("the tail"));
    let mut out = store.get(..toc_start).expect("the sections").to_vec();
    out.extend(&toc);
    out.extend(u64::try_from(toc.len()).expect("a length").to_le_bytes());
    out
}

#[test]
fn a_store_whose_table_of_contents_gives_a_section_longer_than_the_file_is_no_store() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
        ],
    );
    let oracle = sorted(uncached(root.path()).rows);
    let _cold =
        counting::lint_counting(root.path(), rules(), Arc::clone(&store)).expect("a cold run");
    let file = root.path().join(".ryo_cache/lint.store");
    let bytes = std::fs::read(&file).expect("the store");
    std::fs::write(&file, with_rest_length(&bytes, u64::MAX)).expect("damaged");
    let after = counting::lint_counting(root.path(), rules(), store)
        .expect("a run over a table of contents that names more bytes than the file holds");
    assert_eq!(sorted(after.rows), oracle);
}

/// A TOML file written in UTF-16 is the report of its syntax errors, never the abort of the run: the query over the project answers.
#[cfg(feature = "toml")]
#[test]
fn a_utf16_toml_file_does_not_abort_the_run() {
    use ryo_sdk::linter::{Languages, MemoryBudget};
    use ryo_sdk::query::{RowLimit, RunRequest, RyoQlQuery};
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("consumer project");
    std::fs::write(
        root.path().join("utf16-comment.toml"),
        include_bytes!(
            "../../../languages/toml/tests/fixtures/suite/invalid/encoding/utf16-comment.toml"
        ),
    )
    .expect("the fixture");
    std::fs::write(root.path().join("ok.toml"), "# fine\na = 1\n").expect("a file");
    let answer = ryo_sdk::query::run(
        Languages(vec![ryo_toml::linked()]),
        &ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(
            std::num::NonZeroUsize::MIN.saturating_add(1),
        )),
        MemoryBudget::Machine,
        RunRequest {
            root: root.path().to_path_buf(),
            query: RyoQlQuery::try_from(
                r#"let e = select * from "syntax-error"; emit e subject file key(span);"#,
            )
            .expect("query"),
            limit: RowLimit(10),
        },
    )
    .expect("the run answers, it does not abort");
    assert!(
        answer.total.0 >= 1,
        "the file that does not parse gives its syntax errors: {answer:?}"
    );
}
