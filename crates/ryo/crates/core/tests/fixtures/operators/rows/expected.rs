// The findings the rule of `operators/rows/input.ryoql` must give over the `function` fixture, in any order: the one name its one table
// holds, `run`, once whatever the number of files, the functions of that name being found after the files.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::name;

/// The findings the operator's rule gives over the `function` fixture, as this fixture states them.
pub(super) fn expected() -> Vec<Finding> {
    vec![finding(name("run"), vec![name("run")])]
}
