//! python's workloads and the growth of its reading: a generated module of functions, classes and imports, every relation asked,
//! through the linked language, at N and at 4N.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot read its own input has nothing to measure: the panic is the failure"
)]

#[path = "performance/manifest_reading/bench.rs"]
mod manifest_reading;

use ryo::languages::{self, contract};

/// How many definitions the generated module the growth check starts from holds.
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

/// One definition of the generated module, `{n}` standing for its number.
const MODEL: &str = "\n\n@dataclass(frozen=True)\nclass Model{n}(Base, Generic[T]):\n    name: str = \"model-{n}\"\n\n    def run(self, a, /, b=1, *args, c, **kw) -> list[int]:\n        # step {n}\n        values = [x * 2 for x in range(a) if x]\n        return os.path.join(f\"{self.name}-{n}\", str(values), other.helper(b))\n";

/// What stands for a definition's number in [`MODEL`].
const NUMBER: &str = "{n}";

/// A module `count` definitions long: imports, decorated classes with bases, methods with every kind of parameter, comprehensions,
/// f-strings and calls.
fn generated(count: usize) -> String {
    let mut text = String::from(
        "\"\"\"A generated module.\"\"\"\nimport os.path\nfrom . import sibling as other\n",
    );
    for each in 0..count {
        text.push_str(&MODEL.replace(NUMBER, &each.to_string()));
    }
    text
}

/// Reads `text` through the linked language, every relation of the catalog asked for, into the worker's table and rows.
fn read(
    text: &str,
    table: &mut languages::Elements,
    symbols: &mut languages::Symbols,
    out: &mut contract::Collect,
) {
    let linked = ryo_python::linked();
    let engine = linked.parse_engine();
    out.clear();
    contract::read(
        engine.as_ref(),
        languages::ParseRequest {
            path: languages::SourcePath("generated.py"),
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
    .expect("the module is read");
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

/// Reads modules of `SIZE` and of four times `SIZE` definitions and fails when the larger takes more than `GROWTH_LIMIT` times the
/// smaller.
fn complexity() {
    let small = timed(&generated(SIZE));
    let large = timed(&generated(4 * SIZE));
    assert!(
        large < small * GROWTH_LIMIT,
        "reading four times the module took {large:?} against {small:?}: the reading grows faster than linearly"
    );
}

/// Reads a generated module of `definitions` definitions, every relation of the catalog asked for: N and 4N.
#[divan::bench(args = [SIZE, 4 * SIZE])]
fn generated_every_relation(bencher: divan::Bencher<'_, '_>, definitions: usize) {
    let text = generated(definitions);
    let (mut table, mut symbols, mut out) = (
        languages::Elements::default(),
        languages::Symbols::default(),
        contract::Collect::default(),
    );
    bencher.bench_local(|| read(&text, &mut table, &mut symbols, &mut out));
}
