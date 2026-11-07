//! The allocation row of the language contract: reading a file allocates no more than the parser library does on its own. It runs alone in its process, with `dhat` as the global allocator, so nothing else counts.

/// Counts every allocation the process makes, so the row measures the language's reading alone.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn main() {
    let linked = ryo_python::linked();
    ryo::languages::contract::allocations_of_linked(
        &linked,
        ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")),
    );
}
