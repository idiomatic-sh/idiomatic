//! css's workloads, and the growth of its reading: a generated stylesheet, every relation asked, through the linked language.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot read its own input has nothing to measure: the panic is the failure"
)]

use ryo::languages::{self, contract};

/// How many rules the generated stylesheet the growth check starts from holds.
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

/// A stylesheet of the shape a framework ships, `count` rules long: imports, style rules with selectors, declarations, functions and
/// urls, conditional rules that nest rules, and comments.
fn generated(count: usize) -> String {
    let mut text = String::from("@charset \"UTF-8\";\n@import url(\"base.css\") layer(base);\n");
    text.extend((0..count).map(|each| {
        format!(
            "/* rule {each} */\n.card-{each}, #hero-{each} > a:hover {{ color: rgb({} 0 0 / 50%) !important; margin: 0 {}px; background: url(img/{each}.png) no-repeat; }}\n@media (min-width: {}em) {{ .card-{each} {{ padding: var(--gap-{each}); & .title {{ font: 700 1.5em/1.2 \"Open Sans\", sans-serif; }} }} }}\n",
            each % 256,
            each % 7,
            each % 90
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
    let linked = ryo_css::linked();
    let engine = linked.parse_engine();
    out.clear();
    contract::read(
        engine.as_ref(),
        languages::ParseRequest {
            path: languages::SourcePath("site.css"),
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
    .expect("the stylesheet is read");
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

/// Reads stylesheets of `SIZE` and of four times `SIZE` rules and fails when the larger takes more than `GROWTH_LIMIT` times the
/// smaller.
fn complexity() {
    let small = timed(&generated(SIZE));
    let large = timed(&generated(4 * SIZE));
    assert!(
        large < small * GROWTH_LIMIT,
        "reading four times the stylesheet took {large:?} against {small:?}: the reading grows faster than linearly"
    );
}

/// Reads a generated stylesheet of `rules` rules, every relation of the catalog asked for: N and 4N.
#[divan::bench(args = [SIZE, 4 * SIZE])]
fn generated_every_relation(bencher: divan::Bencher<'_, '_>, rules: usize) {
    let text = generated(rules);
    let (mut table, mut symbols, mut out) = (
        languages::Elements::default(),
        languages::Symbols::default(),
        contract::Collect::default(),
    );
    bencher.bench_local(|| read(&text, &mut table, &mut symbols, &mut out));
}
