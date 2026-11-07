//! The `discovery` port's fake runs the port's contract.
use super::super::contract::check;
use super::DiscoveryFake;
use ryo::linter::RelativePath;

#[test]
fn succeeds() {
    check(
        &DiscoveryFake {
            files: vec![RelativePath("b.txt".into()), RelativePath("a.txt".into())],
        },
        &DiscoveryFake::default(),
        std::path::Path::new("."),
    );
}
