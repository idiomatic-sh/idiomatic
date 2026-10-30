//! markdown's workloads and the growth of its reading: a generated document, every relation asked,
//! through the linked language.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot read its own input has nothing to measure: the panic is the failure"
)]

use ryo::languages::{self, contract};

/// How many sections the generated document the growth check starts from has.
const SIZE: usize = 2_500;

/// The growth of the time for four times the input beyond which the reading is not linear enough: four is linear, sixteen quadratic.
const GROWTH_LIMIT: u32 = 5;

/// How many times the growth check reads each input: it compares the fastest read of each size.
const ROUNDS: usize = 5;

/// The argument `cargo bench` passes to a bench and `cargo test` does not: the growth check times the reading, so only the timing run makes it.
const BENCH_FLAG: &str = "--bench";

fn main() {
    if std::env::args().any(|argument| argument == BENCH_FLAG) {
        complexity();
    }
    divan::Divan::default()
        .max_time(std::time::Duration::from_secs(10))
        .config_with_args()
        .main();
}

/// A document of the shape a README has, `count` sections long, each with a heading, prose with emphasis, code and a link, a nested
/// list, a fenced block, a quote and a table.
fn generated(count: usize) -> String {
    let mut text = String::from("---\ntitle: generated\n---\n<!-- generated for the bench -->\n");
    text.extend((0..count).map(|each| {
        format!(
            "\n## Section {each}\n\nSome *prose* with `code_{each}` and a [link](https://example.com/{each}).\n\n- one\n  - two [^n{each}]\n- [x] three\n\n```rust\nfn f{each}() {{}}\n```\n\n> a quote\n\n| a | b |\n|---|---|\n| {each} | x |\n\n[^n{each}]: a note\n"
        )
    }));
    text
}

/// Reads `text` through the linked language, every relation of the catalog asked for, into the worker's table and rows.
fn read(
    text: &str,
    table: &mut languages::Elements,
    symbols: &mut languages::Symbols,
    out: &mut contract::Collect,
) {
    let linked = ryo_markdown::linked();
    let engine = linked.parse_engine();
    out.clear();
    contract::read(
        engine.as_ref(),
        languages::ParseRequest {
            path: languages::SourcePath("README.md"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(text.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        table,
        symbols,
        languages::all(),
        out,
    )
    .expect("the document is read");
}

/// How long reading `text` takes at the fastest of `ROUNDS` reads: the fastest is the read the rest of the machine slowed
/// least, so one disturbed read never decides the growth check.
fn timed(text: &str) -> std::time::Duration {
    let (mut table, mut symbols, mut out) = (
        languages::Elements::default(),
        languages::Symbols::default(),
        contract::Collect::default(),
    );
    std::iter::repeat_with(|| {
        let started = std::time::Instant::now();
        read(text, &mut table, &mut symbols, &mut out);
        started.elapsed()
    })
    .take(ROUNDS)
    .min()
    .unwrap_or_default()
}

/// Reads documents of `SIZE` and of four times `SIZE` sections and fails when the larger takes more than `GROWTH_LIMIT` times the
/// smaller.
fn complexity() {
    let small = timed(&generated(SIZE));
    let large = timed(&generated(4 * SIZE));
    assert!(
        large < small * GROWTH_LIMIT,
        "reading four times the document took {large:?} against {small:?}: the reading grows faster than linearly"
    );
}

/// Reads a generated document of `sections` sections, every relation of the catalog asked for: N and 4N.
#[divan::bench(args = [SIZE, 4 * SIZE])]
fn generated_every_relation(bencher: divan::Bencher<'_, '_>, sections: usize) {
    let text = generated(sections);
    let (mut table, mut symbols, mut out) = (
        languages::Elements::default(),
        languages::Symbols::default(),
        contract::Collect::default(),
    );
    bencher.bench_local(|| read(&text, &mut table, &mut symbols, &mut out));
}
