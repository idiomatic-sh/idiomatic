//! manifest_item_written_by_hand: a manifest reader is declared with `ryo::reads!`, never implemented, so the compiler rejects this.

use std::marker::PhantomData;

use ryo::languages::{self, ManifestInput, ManifestReader, Rows, Symbols};

struct ByHand;

fn read(
    _input: &ManifestInput<'_>,
    _symbols: &mut Symbols,
    _out: &mut dyn Rows<languages::WorkspaceMemberRow>,
) {
}

impl languages::ManifestItem for ByHand {
    type Relation = languages::WorkspaceMember;
    type Derived = ();

    const READER: ManifestReader<languages::WorkspaceMember> = ManifestReader {
        read,
        relation: PhantomData,
    };
}

fn main() {}
