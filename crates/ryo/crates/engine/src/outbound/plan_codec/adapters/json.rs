//! The JSON adapter: the plan format of record (`build/vec/runs8/plan8.json`), written as Python's `json.dump` writes
//! it (`", "` and `": "` between items, every character past ASCII escaped).

pub(in crate::outbound::plan_codec) mod adapter;
mod arguments;
mod expressions;
mod fields;
mod formatter;
mod nodes;
mod ordered;
mod texts;
pub(super) mod schemas;

