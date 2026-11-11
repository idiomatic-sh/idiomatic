//! Join assertions enter the actual compiled file owner and its real semantic port.
use super::fixtures::*;
use ryo_execution_domain::domain::table_model::{service::materialize, types::RepresentationKind};
use ryo_relations::{
    domain::relations::{
        service::to_bits,
        types::{ColumnsView, FactColumn, NodeId},
    },
    domain::terms::types::Symbol,
};
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::{DecodeMode, WordSlice};
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
mod family;

#[cfg(test)]
mod lifecycle;
#[cfg(test)]
mod populations;

#[cfg(test)]
mod diagnostics;
#[cfg(test)]
mod storage;

#[cfg(test)]
mod direct;

#[cfg(test)]
mod generations;

#[cfg(test)]
mod release;
