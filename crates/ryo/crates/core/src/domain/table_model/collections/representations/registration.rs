//! Every representation is registered exhaustively.
use super::{interface::RepresentationsItem, types::RepresentationKind};
static ITEMS: [&dyn RepresentationsItem; RepresentationKind::ALL.len()] = [
    &super::stored::item::StoredItem,
    &super::selected::item::SelectedItem,
    &super::tree::item::TreeItem,
    &super::computed::item::ComputedItem,
];
pub(crate) fn registered(kind: RepresentationKind) -> &'static dyn RepresentationsItem {
    match kind {
        RepresentationKind::Stored => ITEMS[0],
        RepresentationKind::Selected => ITEMS[1],
        RepresentationKind::Tree => ITEMS[2],
        RepresentationKind::Computed => ITEMS[3],
    }
}
