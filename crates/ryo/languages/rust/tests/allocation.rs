//! The allocation row of the language contract: reading a file allocates no more than the parser library does on its own. It runs alone in its process, with `dhat` as the global allocator, so nothing else counts.

/// The allocator that counts every allocation the read makes.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn main() {
    let linked = ryo_rust::linked();
    ryo::languages::contract::allocations_of_linked(
        &linked,
        ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")),
    );
}
