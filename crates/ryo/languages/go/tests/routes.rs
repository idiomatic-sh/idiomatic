//! The routes row of the language contract: the walk's rows over every fixture sample equal the rows frozen while the old reader still
//! existed (`tests/frozen/`). The frozen comparison detects change, not correctness: the byte oracles, the three scores and the 294 cases
//! hold correctness.

fn main() {
    ryo::languages::contract::routes_frozen(
        &ryo_go::linked(),
        ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")),
        "go",
    );
}
