//! javascript's embedded workload.
//!
//! The `embedded` hex's adapters back to back: the javascript language with the vue adapter alone, and with the svelte adapter alone, each
//! read a single-file component of its format, every relation of the catalog asked, so the faster is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot read its own input has nothing to measure: the panic is the failure"
)]

use ryo::languages::{self, contract};
use ryo_javascript::fixtures::contracts::{svelte, vue};

/// A vue component: a template, a script block and a style block.
const VUE: &str = "<template>\n  <p>{{ message }}</p>\n</template>\n<script>\nexport default {\n  data() {\n    return { message: 'hello' };\n  },\n};\n</script>\n<style>\np { color: red; }\n</style>\n";

/// A svelte component: a script block and its markup.
const SVELTE: &str = "<script>\n  let count = 0;\n  function increment() {\n    count += 1;\n  }\n</script>\n<button on:click={increment}>{count}</button>\n";

/// The embedded adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// The vue single-file component format.
    Vue,
    /// The svelte single-file component format.
    Svelte,
}

impl Variant {
    /// Every adapter, each on a component of its format.
    const ALL: [Self; 2] = [Self::Vue, Self::Svelte];
}

/// Reads the component at `path`, of text `text`, through `subject`, every relation of the catalog asked for, on each iteration.
fn read(bencher: divan::Bencher<'_, '_>, subject: &impl languages::Parse, path: &str, text: &str) {
    let (mut table, mut symbols, mut out) = (
        languages::Elements::default(),
        languages::Symbols::default(),
        contract::Collect::default(),
    );
    bencher.bench_local(|| {
        out.clear();
        contract::read(
            subject,
            languages::ParseRequest {
                path: languages::SourcePath(path),
                file: languages::FileId(0),
                bytes: languages::ByteSlice(text.as_bytes()),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Unwanted,
            },
            &mut table,
            &mut symbols,
            languages::all(),
            &mut out,
        )
        .expect("the component is read");
    });
}

/// Reads a component of the variant's format with the language whose embedded blocks only that adapter reads.
#[divan::bench(args = Variant::ALL)]
fn component_every_relation(bencher: divan::Bencher<'_, '_>, variant: Variant) {
    match variant {
        Variant::Vue => read(bencher, &vue(), "component.vue", VUE),
        Variant::Svelte => read(bencher, &svelte(), "component.svelte", SVELTE),
    }
}
