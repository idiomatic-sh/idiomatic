//! The `axes` pass (`pass_axis.py`): an ancestor, descendant or nearest-ancestor axis whose `related` column no reader
//! uses, and (but for nearest-ancestor) whose readers cannot tell a row from its copies, is marked to answer only
//! whether a start has an admitted relative.

pub(crate) mod service;

