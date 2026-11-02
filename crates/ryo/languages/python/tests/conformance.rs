//! The conformance row of the language contract: every relation the language answers `Reads` is shown by a fixture and written as expected, every one it answers `NotInThisLanguage` has no fixture with the construct in it and says why, every manifest relation it reads gives rows from a project fixture, and every kind of its trees is shown or marked unread.

fn main() {
    ryo::languages::contract::conformance_with(
        &ryo_python::linked(),
        ryo::languages::contract::ManifestDir(env!("CARGO_MANIFEST_DIR")),
        &ryo_python::fixtures::expected(),
        &ryo_python::fixtures::configuration::select,
    );
}
