//! The allocation row of the engine, proved on language doubles: the allocations of a language's own library parse (`read_tree`) are in
//! the count of reading and in the count of the baseline alike, so they cancel, and only the table and the readers are judged. A
//! `read_tree` that allocates a scratch buffer in every file passes; a reader that allocates in every file fails. It runs alone in its
//! process, with `dhat` as the global allocator, so nothing else counts.

use std::panic::{AssertUnwindSafe, catch_unwind};

use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::parse::fixtures::allocating::AllocatingLanguage;
use ryo_language::inbound::use_cases::parse::fixtures::samples::{allocations, harness_case};
use ryo_language::inbound::use_cases::parse::service::baseline;
use ryo_language::inbound::use_cases::parse::types::{ParseRequest, SourceFile};

/// Counts every allocation the test makes, so a run's allocations are read back.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// Runs the allocation row over the double and says whether it held.
fn held(language: &AllocatingLanguage) -> bool {
    let sample = SourceFile {
        path: SourcePath("sample.double"),
        bytes: ByteSlice(b"fn one() {}\nfn two() {}\n"),
    };
    // The refusal is the row's answer here, not a failure: its message is not printed.
    let printed = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let held = catch_unwind(AssertUnwindSafe(|| {
        allocations(language, sample, &|bytes| {
            baseline(
                language,
                &ParseRequest {
                    path: sample.path,
                    file: ryo_relations::domain::relations::types::FileId(0),
                    bytes,
                    notes: ryo_language::inbound::use_cases::parse::types::NoteRequest::Unwanted,
                    fields: ryo_language::inbound::use_cases::parse::types::FieldRequest::Unwanted,
                },
            )
        });
    }))
    .is_ok();
    std::panic::set_hook(printed);
    held
}

fn main() {
    harness_case(
        "allocation::language::reading_allocates_as_the_parser_library_does",
        || {
            assert!(
                held(&AllocatingLanguage {
                    greedy_reader: false
                }),
                "a library parse that allocates in every file is the library's, and the row holds"
            );
            assert!(
                !held(&AllocatingLanguage {
                    greedy_reader: true
                }),
                "a reader that allocates in every element is not the library's, and the row does not hold"
            );
        },
    );
}
