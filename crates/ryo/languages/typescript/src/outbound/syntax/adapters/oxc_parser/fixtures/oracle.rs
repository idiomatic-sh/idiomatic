//! Semantic facts from the independently executed reference, compared with the supported relation rows.

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "Only the listed independently observable relations have reference facts; unsupported cells are rejected"
)]

use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows};

use super::walk::{Acceptance, acceptance, block};
