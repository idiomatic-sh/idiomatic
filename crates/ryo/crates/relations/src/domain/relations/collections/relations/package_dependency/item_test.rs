//! The `package_dependency` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{PackageDependency, PackageDependencyRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, Optionality};
use crate::domain::dependency_kind::types::DependencyKind;
use crate::domain::dependency_origin::types::DependencyOrigin;
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 16];
type Failure = ();

fn row() -> PackageDependencyRow {
    PackageDependencyRow {
        source: FileId(3),
        target: FileId(5),
        binding: Symbol(7),
        package: Symbol(11),
        kind: DependencyKind::Build,
        condition: Symbol(17),
        optional: Optionality::Optional,
        origin: DependencyOrigin::Git,
        requirement: Symbol(19),
        exact: Flag::Raised,
        branch: Symbol(23),
        tag: Symbol(29),
        revision: Symbol(31),
        interpreter: Symbol(37),
        catalog: Symbol(41),
        replacement: Symbol(43),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_16_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(7),
        Bits(11),
        Bits(4_294_902_786),
        Bits(17),
        Bits(1),
        Bits(4_294_903_298),
        Bits(19),
        Bits(1),
        Bits(23),
        Bits(29),
        Bits(31),
        Bits(37),
        Bits(41),
        Bits(43),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(PackageDependency::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
