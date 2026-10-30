//! What every item of the `relations` collection must pass: the shared rows of the language kernel.

/// Runs the kernel's shared rows against one item.
pub(crate) fn check<Item>()
where
    Item: ryo::languages::ReadItem,
{
    ryo::languages::contract::check_item::<Item>();
}
