//! The `file_view` concept: the typescript language as the engine's walk and its fillers name it, and the view of one walked file.

#![cfg_attr(not(test), allow(dead_code, reason = "the walk is linked when step 0 of the ports lands; its tests read it until then"))]

pub(crate) mod service;
pub(crate) mod types;
