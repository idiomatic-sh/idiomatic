//! The routes row of the language contract: over every fixture sample, the old reader and the walk's fillers write the same rows;
//! and the walk's rows equal the rows frozen while the old reader still existed (`tests/frozen/`). The frozen comparison detects change,
//! not correctness: the byte oracles, the three scores and the 294 cases hold correctness.

fn main() {
    let language = ryo_css::linked();
    ryo::languages::contract::routes_agree(&language, ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")));
    ryo::languages::contract::routes_frozen(&language, ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")), "css");
}
