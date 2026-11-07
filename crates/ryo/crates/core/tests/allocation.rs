//! The allocation row of the whole per-file path: `lint` over the core's own corpus, the fake language and every
//! operator's rule, allocates no more for a file than the parser does on its own, which for the fake is nothing. No
//! language's corpus is read here: a language's own allocation test is `tests/allocation.rs` of the language. The
//! measuring and the comparison are the core's contract, the same for every product. It runs alone in its process,
//! with `dhat` as the global allocator, so nothing else counts.

use ryo_core::inbound::use_cases::lint::fixtures::contract;

/// The allocator that counts every block, so the row measures what the path allocates.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// What the parser does for a file by itself: the fake language reads a file without allocating.
fn library(_bytes: &[u8]) {}

fn main() {
    contract::allocation(&contract::fake_corpus(), &library);
}
