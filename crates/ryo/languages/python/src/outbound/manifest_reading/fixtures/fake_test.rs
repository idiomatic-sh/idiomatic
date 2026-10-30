//! The `manifest_reading` port's fake runs the port's contract.

use super::super::contract::{Sample, check};
use super::ManifestReadingFake;

#[test]
fn succeeds() {
    check(
        |_| ManifestReadingFake,
        Sample {
            path: "pyproject.toml",
            readable: "[project]\nname = \"demo\"\n",
            broken: "[project\nname = ",
            others: &[],
        },
    );
}
