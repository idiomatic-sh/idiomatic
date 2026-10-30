//! The `templates` port's fake runs the port's contract.

use super::super::contract::check;
use super::TemplatesFake;

#[test]
fn succeeds() {
    check(&TemplatesFake);
}
