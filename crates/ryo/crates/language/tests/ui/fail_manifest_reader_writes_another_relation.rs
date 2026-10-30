//! manifest_reader_writes_another_relation: the compiler rejects this.

use std::marker::PhantomData;

use ryo_language::domain::relations::collections::relations::package_dependency::item::PackageDependency;
use ryo_language::domain::relations::types::RowWriter;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::{ManifestInput, ManifestReader};

/// A reader of `package_dependency` that is given the writer of every relation, which its type refuses: it is given the rows of its own.
fn read(_input: &ManifestInput<'_>, _symbols: &mut Symbols, _out: &mut dyn RowWriter) {}

/// The reader of `package_dependency`, built over that read.
const READER: ManifestReader<PackageDependency> = ManifestReader {
    read,
    relation: PhantomData,
};

fn main() {
    let _ = READER;
}
