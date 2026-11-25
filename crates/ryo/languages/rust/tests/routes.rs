//! The routes row of the language contract: over every fixture sample, the old reader and the walk's fillers write the same rows.

fn main() {
    ryo::languages::contract::routes_agree(
        &ryo_rust::linked(),
        ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")),
    );
}
