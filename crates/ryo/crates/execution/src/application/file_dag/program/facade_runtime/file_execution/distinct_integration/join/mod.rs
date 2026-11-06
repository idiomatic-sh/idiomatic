//! Join assertions enter the actual compiled file owner and its real semantic port.
use super::*;
use crate::domain::{
    table_cells::types::TreeRows,
    table_model::{collections::representations::types::RepresentationKind, service::materialize},
};
use ryo_relations::{
    domain::relations::{
        service::to_bits,
        types::{ColumnsView, FactColumn, NodeId},
    },
    domain::terms::types::Symbol,
};
use ryo_values::domain::{
    stored_facts::types::KeptWords,
    table_words::{
        service::{admit_compact, encode_table, pack},
        types::{DecodeMode, WordSlice},
    },
};
use std::cell::Cell as Counter;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
#[cfg(test)]
mod capability;
#[cfg(test)]
mod cases;

#[cfg(test)]
mod lifecycle;
#[cfg(test)]
mod populations;

#[cfg(test)]
mod column_chain;

#[cfg(test)]
mod diagnostics;
#[cfg(test)]
mod storage;

#[cfg(test)]
mod direct;

#[cfg(test)]
mod generations;

#[cfg(test)]
mod raw_scope;

#[cfg(test)]
mod release;
