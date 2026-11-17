//! The `packages` port's fake runs the port's contract.

use super::super::contract::check;
use super::PackagesFake;
use tastemaker_values::domain::packages::types::PackageRoot;

#[test]
fn succeeds() {
    let fake = PackagesFake::of("p", &[("rules/r/0.1.0/rule.toml", "description = \"r\"\n")]);
    check(&fake, &PackageRoot("/p".into()));
}
