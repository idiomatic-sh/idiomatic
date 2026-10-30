//! The `manifest_reading` port's fake runs the port's contract.

use super::super::contract::check;
use super::ManifestReadingFake;

#[test]
fn succeeds() {
    check(&ManifestReadingFake);
}
